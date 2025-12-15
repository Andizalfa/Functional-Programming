use axum::Router;
use tower_http::cors::{CorsLayer, Any};
use axum::extract::DefaultBodyLimit;
use backend::routes;

#[tokio::main]
async fn main() {
    // Konfigurasi CORS dengan expose headers untuk custom header
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers([axum::http::HeaderName::from_static("x-process-time")]);

    
    let app = Router::new()
        .nest("/api", routes::routes())
        .layer(cors)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100MB upload max

    println!("Server running at http://127.0.0.1:3000");

    axum::serve(
        tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap(),
        app
    )
    .await
    .unwrap();
}
