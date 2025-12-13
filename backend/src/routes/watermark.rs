use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::services::watermark_service::process_multiprocess;
use std::path::PathBuf;

pub async fn process_watermark() -> impl IntoResponse {

    // contoh (biasanya dari multipart handler)
    let images = vec![
        PathBuf::from("tmp/img1.png"),
        PathBuf::from("tmp/img2.png"),
    ];

    let watermark = PathBuf::from("tmp/watermark.png");

    let results = process_multiprocess(images, watermark);

    (
        StatusCode::OK,
        format!("Diproses {} file secara MULTIPROCESSING", results.len()),
    )
}