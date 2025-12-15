// Mengimpor StatusCode untuk merepresentasikan HTTP status code
use axum::http::StatusCode;
// Mengimpor trait IntoResponse untuk mengkonversi nilai menjadi HTTP response
use axum::response::IntoResponse;
// Mengimpor fungsi process_multiprocess dari modul watermark_service
use crate::services::watermark_service::process_multiprocess;
// Mengimpor PathBuf untuk merepresentasikan path file sistem
use std::path::PathBuf;
// Mengimpor untuk tracking waktu proses
use std::time::Instant;
// Mengimpor Multipart untuk menerima file upload
use axum::extract::Multipart;
// Mengimpor fungsi untuk ZIP dan file I/O
use std::fs;
use std::io::Write;
use zip::ZipWriter;
use zip::write::FileOptions;
// Mengimpor untuk HTTP response dengan body
use axum::body::Body;
use axum::http::header;
use tokio_util::io::ReaderStream;

// =====================================================================
// PURE DATA STRUCTURES (IMMUTABLE)
// =====================================================================

#[derive(Debug, Clone)]
struct UploadedFile {
    name: String,
    path: PathBuf,
}

#[derive(Debug)]
struct CategorizedFiles {
    photos: Vec<PathBuf>,
    watermark: Option<PathBuf>,
}

// =====================================================================
// PURE HELPER FUNCTIONS
// =====================================================================

// Fungsi pure untuk menyimpan file yang diupload
// Mengembalikan UploadedFile (immutable struct)
async fn save_uploaded_file(
    name: String,
    filename: String,
    data: bytes::Bytes,
) -> UploadedFile {
    let path = PathBuf::from(format!("tmp/{}", filename));
    tokio::fs::write(&path, &data).await.expect("Gagal menyimpan file");
    UploadedFile { name, path }
}

// Fungsi pure untuk mengkategorikan uploaded files menggunakan fold
// Input: Vec<UploadedFile> (immutable)
// Output: CategorizedFiles (immutable)
fn categorize_files(files: Vec<UploadedFile>) -> CategorizedFiles {
    files.into_iter().fold(
        CategorizedFiles {
            photos: Vec::new(),
            watermark: None,
        },
        |acc, file| {
            if file.name == "photos" {
                // Immutable append - membuat Vec baru dengan elemen tambahan
                CategorizedFiles {
                    photos: [acc.photos, vec![file.path]].concat(),
                    watermark: acc.watermark,
                }
            } else if file.name == "watermark" {
                CategorizedFiles {
                    photos: acc.photos,
                    watermark: Some(file.path),
                }
            } else {
                acc
            }
        },
    )
}

// Fungsi untuk membuat ZIP file dari list hasil
// Note: ZipWriter inherently memerlukan &mut, tapi kita encapsulate mutability di dalam fungsi ini
// sehingga dari luar terlihat sebagai pure function (input Vec<PathBuf> -> output Vec<u8>)
fn create_zip_from_results(results: &[PathBuf]) -> Vec<u8> {
    let buffer = Vec::new();
    let cursor = std::io::Cursor::new(buffer);
    
    // Encapsulated mutation - hanya di dalam scope fungsi ini
    let zip_writer = ZipWriter::new(cursor);
    
    // Fold untuk iterate results, tapi ZipWriter butuh mutable reference internal
    let mut final_writer = results.iter().enumerate().fold(
        zip_writer,
        |writer, (index, path)| {
            write_file_to_zip(writer, path, index + 1)
        }
    );
    
    // Extract bytes dari writer
    final_writer
        .finish()
        .expect("Gagal finalize ZIP")
        .into_inner()
}

// Helper function untuk menulis satu file ke ZIP
// Menggunakan ownership transfer pattern untuk simulate immutability
// Note: mut diperlukan karena ZipWriter::write membutuhkan &mut self
fn write_file_to_zip<W: Write + std::io::Seek>(
    mut zip: ZipWriter<W>,
    result_path: &PathBuf,
    index: usize,
) -> ZipWriter<W> {
    let file_data = fs::read(result_path).expect("Gagal membaca file hasil");
    let filename = format!("watermarked_{}.png", index);
    
    // Meskipun internally mutable, kita transfer ownership untuk simulate functional style
    zip.start_file(filename, FileOptions::default())
        .expect("Gagal memulai file di ZIP");
    
    zip.write_all(&file_data)
        .expect("Gagal menulis data ke ZIP");
    
    zip
}

// =====================================================================
// MAIN HANDLER (FUNCTIONAL STYLE)
// =====================================================================

// Fungsi handler async untuk endpoint pemrosesan watermark
// Business logic menggunakan immutable data structures dan pure functions
pub async fn process_watermark(multipart: Multipart) -> impl IntoResponse {
    // Mulai tracking waktu
    let start_time = Instant::now();
    
    // Membuat folder tmp jika belum ada
    fs::create_dir_all("tmp").expect("Gagal membuat folder tmp");

    // Collect semua uploaded files (I/O encapsulated)
    let files = collect_uploaded_files(multipart).await;
    
    // Kategorikan files menggunakan pure function
    let categorized = categorize_files(files);

    // Validasi input
    if categorized.photos.is_empty() || categorized.watermark.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "Harap upload foto dan watermark".to_string(),
        ).into_response();
    }

    // Extract watermark path (sudah divalidasi di atas)
    let watermark_path = categorized.watermark.unwrap();

    // Memanggil fungsi process_multiprocess (pure function)
    let results = process_multiprocess(categorized.photos, watermark_path);

    // Membuat ZIP file (encapsulated mutation)
    let zip_bytes = create_zip_from_results(&results);
    
    // Simpan ZIP ke file
    let zip_path = PathBuf::from("tmp/hasil_watermark.zip");
    tokio::fs::write(&zip_path, &zip_bytes)
        .await
        .expect("Gagal menyimpan ZIP");

    // Hitung waktu proses
    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    
    // Buat response dengan ZIP file dan waktu proses
    create_zip_response_with_time(zip_path, duration_secs).await
}

// =====================================================================
// I/O HELPER FUNCTIONS (Encapsulated Mutations)
// =====================================================================
// Note: Untuk I/O operations, kita encapsulate mutability di dalam fungsi
// sehingga dari perspektif caller, fungsi terlihat pure (immutable input/output)

// Fungsi untuk collect semua uploaded files dari multipart
// Mutation encapsulated, returns immutable Vec
async fn collect_uploaded_files(multipart: Multipart) -> Vec<UploadedFile> {
    // Extract fields dari multipart
    let fields = extract_fields_from_multipart(multipart).await;
    
    // Process semua fields secara paralel menggunakan futures::join_all
    futures::future::join_all(
        fields.into_iter().map(|(name, filename, data)| {
            save_uploaded_file(name, filename, data)
        })
    ).await
}

// Fungsi untuk extract fields dari multipart
// Encapsulated mutation - mutation hanya terjadi di dalam fungsi ini
// External interface: Multipart -> Vec (pure dari perspektif caller)
async fn extract_fields_from_multipart(
    multipart: Multipart,
) -> Vec<(String, String, bytes::Bytes)> {
    // Gunakan explicit mut di sini, tapi encapsulated dalam fungsi
    // Dari perspektif caller, fungsi ini immutable: Multipart in -> Vec out
    let results = Vec::new();
    let multipart = multipart;
    
    // Gunakan helper yang mutable internally
    extract_with_mut(multipart, results).await
}

// Helper function dengan explicit mut (encapsulated)
// Ini adalah detail implementasi yang tidak ter-expose ke business logic
async fn extract_with_mut(
    multipart: Multipart,
    results: Vec<(String, String, bytes::Bytes)>,
) -> Vec<(String, String, bytes::Bytes)> {
    let mut multipart = multipart;  // Mutable untuk next_field()
    let mut results = results;       // Mutable untuk push()
    
    // Iterasi dengan while let (pragmatic approach untuk I/O)
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                let filename = field.file_name().unwrap_or("unknown").to_string();
                match field.bytes().await {
                    Ok(data) => {
                        results.push((name, filename, data));
                    }
                    Err(_) => {}
                }
            }
            _ => break,
        }
    }
    
    results
}

// Fungsi untuk membuat ZIP response dengan waktu proses
async fn create_zip_response_with_time(zip_path: PathBuf, duration_secs: f64) -> axum::response::Response {
    let file = tokio::fs::File::open(&zip_path).await.unwrap();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE, 
        axum::http::HeaderValue::from_static("application/zip")
    );
    headers.insert(
        header::CONTENT_DISPOSITION, 
        axum::http::HeaderValue::from_static("attachment; filename=\"hasil_watermark.zip\"")
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-process-time"),
        axum::http::HeaderValue::from_str(&format!("{:.5}", duration_secs)).unwrap()
    );

    (StatusCode::OK, headers, body).into_response()
}