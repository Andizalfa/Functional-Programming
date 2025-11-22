use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer};

async fn hello() -> &'static str {
    "Backend Axum jalan!"
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(hello))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    println!("Server running at http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
