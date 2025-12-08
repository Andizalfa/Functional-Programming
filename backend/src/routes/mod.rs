pub mod watermark;

use axum::{Router, routing::post};
use watermark::process_watermark;

pub fn routes() -> Router {
    Router::new()
        .route("/watermark", post(process_watermark))
}
