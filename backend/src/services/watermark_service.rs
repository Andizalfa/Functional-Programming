use axum::extract::Multipart;
use std::io::{Cursor, Write};
use zip::write::FileOptions;

use crate::utils::image_ops::process_photos_parallel;

pub async fn process(mut multipart: Multipart) -> Vec<u8> {
    let mut photos = vec![];
    let mut watermark_bytes = None;

    // ===========================
    // 1. SAFE MULTIPART READER
    // ===========================
    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("❌ Multipart read error: {:?}", e);
            return empty_zip(); // return ZIP kosong agar server tidak PANIC
        }
    } {
        // Nama field
        let name = match field.name() {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Filename (aman walau tidak ada)
        let filename = field.file_name().unwrap_or("file.png").to_string();

        // Baca bytes
        let bytes = match field.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                eprintln!("❌ Error membaca bytes: {:?}", e);
                continue;
            }
        };

        // Simpan sesuai jenis field
        match name.as_str() {
            "watermark" => watermark_bytes = Some(bytes),
            "photos" | "photos[]" => photos.push((filename, bytes)),
            other => {
                eprintln!("⚠️ Field tidak dikenal: {}", other);
            }
        }
    }

    // Jika watermark tidak ada → return ZIP kosong (lebih aman daripada panic)
    let wm_data = match watermark_bytes {
        Some(data) => data,
        None => {
            eprintln!("❌ Watermark tidak ditemukan dalam form-data!");
            return empty_zip();
        }
    };

    // Load watermark (juga aman)
    let wm_img = match image::load_from_memory(&wm_data) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("❌ Gagal load watermark: {:?}", e);
            return empty_zip();
        }
    };

    // ===========================
    // 2. PROSES PARALEL
    // ===========================
    let opacity = 0.5;
    let processed_results = process_photos_parallel(photos, wm_img, opacity);

    // ===========================
    // 3. ZIP OUTPUT
    // ===========================
    create_zip(processed_results)
}


// ===========================
// ZIP UTILITY #1 — ZIP KOSONG
// ===========================
fn empty_zip() -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let _ = zip.start_file("error.txt", opts);
        let _ = zip.write_all("Multipart gagal dibaca".as_bytes());
    }
    buffer.into_inner()
}


// ===========================
// ZIP UTILITY #2 — ZIP HASIL
// ===========================
fn create_zip(files: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (filename, bytes) in files {
            let _ = zip.start_file(filename, opts);
            let _ = zip.write_all(&bytes);
        }
    }
    buffer.into_inner()
}
