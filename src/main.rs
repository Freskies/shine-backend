pub mod db;
pub mod handlers;
pub mod pdf;
pub mod render;
pub mod state;

use crate::handlers::showcase::{
	enrollment_handler, index_handler, membership_form_post_handler, membership_handler,
	membership_pdf_download_handler,
};
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
	let static_files = ServeDir::new("static");

	let app = Router::new()
		.route("/", get(index_handler))
		.route("/membership_form", get(membership_handler))
		.route("/membership_form", post(membership_form_post_handler))
		.route(
			"/membership_form/download/{id}",
			get(membership_pdf_download_handler),
		)
		.route("/enrollment", get(enrollment_handler))
		.nest_service("/static", static_files)
		.with_state(AppState::new());

	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	println!("🚀 Server avviato su http://{addr}");

	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app).await.unwrap()
}
