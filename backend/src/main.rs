use axum::Router;
use tower_http::cors::CorsLayer;

mod routes;
mod services;
mod utils;

use axum::extract::DefaultBodyLimit;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest("/api", routes::routes())
        .layer(CorsLayer::very_permissive())
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100MB upload max

    println!("Server running at http://127.0.0.1:3000");

    axum::serve(
        tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap(),
        app
    )
    .await
    .unwrap();
}
