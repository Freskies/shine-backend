use askama::Template;
use askama_axum::IntoResponse;

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate;

pub async fn index_handler() -> impl IntoResponse {
	IndexTemplate
}