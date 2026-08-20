use askama::Template;
use axum::response::IntoResponse;

use crate::render::HtmlTemplate;

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate;

pub async fn index_handler() -> impl IntoResponse {
	HtmlTemplate(IndexTemplate)
}
