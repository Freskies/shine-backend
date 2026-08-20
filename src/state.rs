use crate::config::Config;
use std::sync::Arc;

/// Shared application state.
///
/// Only the validated configuration for now. Nothing about an enrolment is kept between
/// requests: the wizard submits in one shot and the result leaves as email, so there is no
/// draft to park and nothing sensitive lingering in memory.
#[derive(Clone)]
pub struct AppState {
	pub config: Arc<Config>,
}

impl AppState {
	pub fn new(config: Config) -> Self {
		Self {
			config: Arc::new(config),
		}
	}
}
