pub mod config;
pub mod db;
pub mod email;
pub mod handlers;
pub mod pdf;
pub mod render;
pub mod state;

use crate::config::Config;
use crate::handlers::enrollment::{
	emergency_contact_row_handler, enrollment_handler, enrollment_submit_handler,
};
use crate::handlers::showcase::{index_handler, privacy_policy_handler, statute_handler};
use crate::state::AppState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use axum::middleware;
use axum::response::Response;
use axum::routing::{get, post};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

/// Room for a certificate photo straight off a phone camera, which the 2 MB default would
/// reject. Must stay at or above `MAX_CERTIFICATE_BYTES` in the enrolment handler,
/// otherwise the request is cut off before that check can report a friendly error.
const MAX_UPLOAD_BYTES: usize = 16 * 1024 * 1024;

/// Forces revalidation of static assets.
///
/// `ServeDir` sends `ETag` and `Last-Modified` but no `Cache-Control`. With no explicit
/// lifetime, a browser falls back to *heuristic* freshness — roughly 10% of the time since
/// the file was last modified — and reuses its copy without asking, so an edited
/// stylesheet can keep serving stale for minutes. `no-cache` still allows caching; it just
/// requires a revalidation first, which the existing `ETag` answers with an inexpensive 304.
async fn revalidate_static(mut response: Response) -> Response {
	response
		.headers_mut()
		.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
	response
}

/// Starts logging.
///
/// The level comes from `RUST_LOG`, falling back to a default that shows one line per
/// request plus everything this crate emits, while keeping the noisy internals of rustls
/// and hyper quiet. Examples:
///
/// - `RUST_LOG=debug` — everything, including the SMTP conversation
/// - `RUST_LOG=shine_backend=debug` — verbose for this crate only
fn init_tracing() {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info,shine_backend=debug,tower_http=info,lettre=info"));

	tracing_subscriber::fmt()
		.with_env_filter(filter)
		.with_target(true)
		// Wall-clock time only: these logs are read while debugging, not shipped anywhere.
		.without_time()
		.init();
}

#[tokio::main]
async fn main() {
	init_tracing();

	// Validated up front: a missing SMTP password should stop the server now, not lose
	// somebody's enrolment later.
	let config = match Config::from_env() {
		Ok(e) => e,
		Err(e) => {
			tracing::error!("configuration is invalid: {e}");
			tracing::error!("copy .env.example to .env and fill it in");
			std::process::exit(1);
		}
	};

	info!(
		smtp = %format!("{}:{}", config.smtp.host, config.smtp.port),
		from = %config.smtp.mail,
		enrollment_recipient = %config.enrollment_recipient,
		whatsapp = config.whatsapp_number.is_some(),
		"configuration loaded"
	);
	if config.whatsapp_number.is_none() {
		tracing::warn!("WHATSAPP_NUMBER is unset: the last enrolment step shows a placeholder");
	}

	let static_files = Router::new()
		.fallback_service(ServeDir::new("static"))
		.layer(middleware::map_response(revalidate_static));

	let app = Router::new()
		.route("/", get(index_handler))
		.route("/privacy-policy", get(privacy_policy_handler))
		.route("/statute", get(statute_handler))
		.route("/enrollment", get(enrollment_handler))
		.route(
			"/enrollment",
			post(enrollment_submit_handler).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
		)
		.route(
			"/enrollment/emergency-contact",
			get(emergency_contact_row_handler),
		)
		.nest_service("/static", static_files)
		// Outermost, so it also records requests rejected by the layers below it.
		// The levels are set explicitly: TraceLayer defaults to DEBUG, which means request
		// logging silently disappears the moment someone runs with `RUST_LOG=info`.
		.layer(
			TraceLayer::new_for_http()
				.make_span_with(DefaultMakeSpan::new().level(Level::INFO))
				.on_response(DefaultOnResponse::new().level(Level::INFO)),
		)
		.with_state(AppState::new(config));

	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

	let listener = match tokio::net::TcpListener::bind(addr).await {
		Ok(listener) => listener,
		Err(e) => {
			tracing::error!("cannot bind {addr}: {e}");
			std::process::exit(1);
		}
	};
	info!("listening on http://{addr}");

	if let Err(e) = axum::serve(listener, app).await {
		tracing::error!("server stopped: {e}");
		std::process::exit(1);
	}
}
