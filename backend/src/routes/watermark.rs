use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::services::watermark_service::process_multiprocess;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashMap;
use axum::extract::Multipart;
use std::fs;
use std::io::Write;
use zip::ZipWriter;
use zip::write::FileOptions;
use axum::body::Body;
use axum::http::header;
use tokio_util::io::ReaderStream;

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

async fn save_uploaded_file(
    name: String,
    filename: String,
    data: bytes::Bytes,
) -> UploadedFile {
    let path = PathBuf::from(format!("tmp/{}", filename));
    tokio::fs::write(&path, &data).await.expect("Gagal menyimpan file");
    UploadedFile { name, path }
}

fn categorize_files(files: Vec<UploadedFile>) -> CategorizedFiles { // Refactored to use HashMap for grouping
    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for UploadedFile { name, path } in files {
        map.entry(name).or_default().push(path);
    }

    let photos = map.remove("photos").unwrap_or_default();
    let watermark = map.remove("watermark").and_then(|mut v| v.into_iter().next());

    CategorizedFiles { photos, watermark }
}

fn create_zip_from_results(results: &[(PathBuf, f64)]) -> Vec<u8> {
    let buffer = Vec::new();
    let cursor = std::io::Cursor::new(buffer);
    
    let mut zip_writer = ZipWriter::new(cursor);

    // Tulis setiap file hasil ke ZIP
    for (index, (path, _dur)) in results.iter().enumerate() {
        zip_writer = write_file_to_zip(zip_writer, path, index + 1);
    }

    // Buat manifest JSON yang berisi nama file di dalam ZIP dan durasi per-file
    let mut items: Vec<String> = Vec::new();
    for (index, (_path, dur)) in results.iter().enumerate() {
        let filename = format!("watermarked_{}.png", index + 1);
        items.push(format!("{{\"file\":\"{}\",\"duration\":{:.5}}}", filename, dur));
    }
    let manifest = format!("[{}]", items.join(","));

    zip_writer.start_file("manifest.json", FileOptions::default())
        .expect("Gagal memulai manifest di ZIP");
    zip_writer.write_all(manifest.as_bytes()).expect("Gagal menulis manifest ke ZIP");

    zip_writer
        .finish()
        .expect("Gagal finalize ZIP")
        .into_inner()
}

fn write_file_to_zip<W: Write + std::io::Seek>(
    mut zip: ZipWriter<W>,
    result_path: &PathBuf,
    index: usize,
) -> ZipWriter<W> {
    let file_data = fs::read(result_path).expect("Gagal membaca file hasil");
    let filename = format!("watermarked_{}.png", index);
    
    zip.start_file(filename, FileOptions::default())
        .expect("Gagal memulai file di ZIP");
    
    zip.write_all(&file_data)
        .expect("Gagal menulis data ke ZIP");
    
    zip
}

pub async fn process_watermark(multipart: Multipart) -> impl IntoResponse {
    let start_time = Instant::now();
    
    fs::create_dir_all("tmp").expect("Gagal membuat folder tmp");

    let files = collect_uploaded_files(multipart).await;
    
    let categorized = categorize_files(files);

    if categorized.photos.is_empty() || categorized.watermark.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "Harap upload foto dan watermark".to_string(),
        ).into_response();
    }

    let watermark_path = categorized.watermark.unwrap();

    // Panggil service yang sekarang mengembalikan tuple (path_output, durasi_detik)
    let results = process_multiprocess(categorized.photos, watermark_path);

    let zip_bytes = create_zip_from_results(&results);
    
    let zip_path = PathBuf::from("tmp/hasil_watermark.zip");
    tokio::fs::write(&zip_path, &zip_bytes)
        .await
        .expect("Gagal menyimpan ZIP");

    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    
    create_zip_response_with_time(zip_path, duration_secs).await
}

async fn collect_uploaded_files(multipart: Multipart) -> Vec<UploadedFile> {
    let fields = extract_fields_from_multipart(multipart).await;
    
    futures::future::join_all(
        fields.into_iter().map(|(name, filename, data)| {
            save_uploaded_file(name, filename, data)
        })
    ).await
}

async fn extract_fields_from_multipart(
    multipart: Multipart,
) -> Vec<(String, String, bytes::Bytes)> {
    extract_with_mut(multipart).await
}

async fn extract_with_mut(
    multipart: Multipart,
) -> Vec<(String, String, bytes::Bytes)> {
    use futures::stream::StreamExt;
    
    let stream = futures::stream::unfold(multipart, |mut mp| async move {
        match mp.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                let filename = field.file_name().unwrap_or("unknown").to_string();
                match field.bytes().await {
                    Ok(data) => Some(((name, filename, data), mp)),
                    Err(_) => None
                }
            }
            _ => None,
        }
    });
    
    stream.collect::<Vec<_>>().await
}

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