pub mod handlers;
pub mod db;

use axum::Router;
use axum::routing::get;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use crate::handlers::showcase::index_handler;

#[tokio::main]
async fn main() {
	let static_files = ServeDir::new("static");

	let app = Router::new()
		.route("/", get(index_handler))
		.nest_service("/static", static_files);

	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	println!("🚀 Server avviato su http://{addr}");

	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app).await.unwrap()
}
