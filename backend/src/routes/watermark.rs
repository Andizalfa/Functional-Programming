use axum::{
    body::Body,
    extract::Multipart,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::stream::{self, TryStreamExt};
use std::{io::Write, path::PathBuf, time::Instant};
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};

use crate::services::watermark_service::process_multiprocess;
use crate::utils::error::AppError;

#[derive(Debug, Clone)]
struct UploadedFile {
    field: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct CategorizedFiles {
    photos: Vec<PathBuf>,
    watermark: Option<PathBuf>,
}

/* =========================================================
   PURE CORE (fungsi tunggal, tidak IO)
   ========================================================= */

fn categorize_files(files: Vec<UploadedFile>) -> CategorizedFiles {
    files.into_iter().fold(
        CategorizedFiles {
            photos: vec![],
            watermark: None,
        },
        |acc, f| match f.field.as_str() {
            "photos" => CategorizedFiles {
                photos: acc.photos.into_iter().chain(std::iter::once(f.path)).collect(),
                watermark: acc.watermark,
            },
            "watermark" => CategorizedFiles {
                photos: acc.photos,
                watermark: Some(f.path),
            },
            _ => acc,
        },
    )
}

fn validate_inputs(cat: CategorizedFiles) -> Result<(Vec<PathBuf>, PathBuf), AppError> {
    match (cat.photos.is_empty(), cat.watermark) {
        (false, Some(wm)) => Ok((cat.photos, wm)),
        _ => Err(AppError::BadRequest(
            "Harap upload foto dan watermark".to_string(),
        )),
    }
}

fn zip_entry_name(index: usize) -> String {
    format!("watermarked_{}.png", index + 1)
}

/// Tanpa `mut headers` di body fungsi: pakai collect.
/// (Mutasi tetap terjadi di internal builder HeaderMap.)
fn build_headers(duration_secs: f64) -> Result<HeaderMap, AppError> {
    let x_process_time = HeaderValue::from_str(&format!("{:.5}", duration_secs))
        .map_err(|e| AppError::Internal(format!("Header invalid: {e}")))?;

    Ok(
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("application/zip")),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"hasil_watermark.zip\""),
            ),
            (axum::http::HeaderName::from_static("x-process-time"), x_process_time),
        ]
        .into_iter()
        .collect::<HeaderMap>(),
    )
}

/* =========================================================
   IO HELPERS (fungsi tunggal, ada side-effect)
   ========================================================= */

/// Tanpa `mut fields`: pakai stream `try_unfold` + `try_collect`.
/// `mut` tetap ada di dalam closure karena `next_field(&mut self)` memang wajib.
async fn read_multipart(multipart: Multipart) -> Result<Vec<(String, String, Bytes)>, AppError> {
    stream::try_unfold(multipart, |mut mp| async move {
        let next = mp
            .next_field()
            .await
            .map_err(|e| AppError::BadRequest(format!("Gagal membaca multipart: {e}")))?;

        match next {
            None => Ok(None),
            Some(field) => {
                let name = field.name().unwrap_or("").to_string();
                let filename = field.file_name().unwrap_or("unknown").to_string();

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Gagal membaca bytes: {e}")))?;

                Ok(Some(((name, filename, data), mp)))
            }
        }
    })
    .try_collect::<Vec<_>>()
    .await
}

async fn save_uploaded_file(
    field: String,
    filename: String,
    data: Bytes,
) -> Result<UploadedFile, AppError> {
    tokio::fs::create_dir_all("tmp")
        .await
        .map_err(|e| AppError::Internal(format!("Gagal membuat folder tmp: {e}")))?;

    let safe_name = format!("{}_{}", Uuid::new_v4(), filename);
    let path = PathBuf::from(format!("tmp/{safe_name}"));

    tokio::fs::write(&path, &data)
        .await
        .map_err(|e| AppError::Internal(format!("Gagal menyimpan file upload: {e}")))?;

    Ok(UploadedFile { field, path })
}

/// `ZipWriter` inherently stateful.
/// Kita bungkus ke helper `with_zip_writer` agar `mut` tidak tersebar.
fn zip_from_paths(results: &[PathBuf]) -> Result<Vec<u8>, AppError> {
    with_zip_writer(Vec::<u8>::new(), |zip| write_results_to_zip(zip, results))
}

/// Membungkus stateful writer sehingga mutability “terlokalisasi”.
fn with_zip_writer(
    buffer: Vec<u8>,
    f: impl FnOnce(
        ZipWriter<std::io::Cursor<Vec<u8>>>,
    ) -> Result<ZipWriter<std::io::Cursor<Vec<u8>>>, AppError>,
) -> Result<Vec<u8>, AppError> {
    let cursor = std::io::Cursor::new(buffer);
    let zip = ZipWriter::new(cursor);

    // `finish()` butuh &mut self → mut hanya di sini (wajib).
    let mut zip = f(zip)?;
    let finished = zip
        .finish()
        .map_err(|e| AppError::Internal(format!("Gagal finalize ZIP: {e}")))?;

    Ok(finished.into_inner())
}

/// Menulis semua file hasil ke ZIP.
/// Mutability terjadi pada `ZipWriter` karena API menuntut &mut internal.
fn write_results_to_zip(
    zip: ZipWriter<std::io::Cursor<Vec<u8>>>,
    results: &[PathBuf],
) -> Result<ZipWriter<std::io::Cursor<Vec<u8>>>, AppError> {
    results.iter().enumerate().try_fold(zip, |mut zip, (i, path)| {
        let file_data = std::fs::read(path)
            .map_err(|e| AppError::Internal(format!("Gagal membaca file hasil: {e}")))?;

        zip.start_file(zip_entry_name(i), FileOptions::default())
            .map_err(|e| AppError::Internal(format!("Gagal start file ZIP: {e}")))?;

        zip.write_all(&file_data)
            .map_err(|e| AppError::Internal(format!("Gagal write ZIP: {e}")))?;

        Ok(zip)
    })
}

/* =========================================================
   ORCHESTRATORS (fungsi yang memanggil fungsi-fungsi di atas)
   ========================================================= */

async fn collect_uploaded_files(multipart: Multipart) -> Result<Vec<UploadedFile>, AppError> {
    let fields = read_multipart(multipart).await?;

    futures::future::try_join_all(
        fields
            .into_iter()
            .map(|(name, filename, data)| save_uploaded_file(name, filename, data)),
    )
    .await
}

pub async fn process_watermark(multipart: Multipart) -> Result<Response, AppError> {
    let start = Instant::now();

    let (photos, watermark) = collect_uploaded_files(multipart)
        .await
        .map(categorize_files)
        .and_then(validate_inputs)?;

    let results = process_multiprocess(photos, watermark)?;

    let zip_bytes = zip_from_paths(&results)?;
    let headers = build_headers(start.elapsed().as_secs_f64())?;

    Ok((StatusCode::OK, headers, Body::from(zip_bytes)).into_response())
}
