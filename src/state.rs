use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a generated PDF stays downloadable.
const PDF_TTL: Duration = Duration::from_secs(600);

/// Hard cap on stored PDFs, so a burst of submissions cannot exhaust memory.
const MAX_STORED_PDFS: usize = 256;

struct StoredPdf {
	bytes: Vec<u8>,
	created: Instant,
}

/// Shared application state.
///
/// HTMX swaps HTML into the DOM and cannot handle a binary response body, so
/// `POST /membership_form` parks the freshly generated PDF here and answers with
/// an HTML fragment plus an `HX-Trigger` header. The browser then pulls the
/// bytes from `GET /membership_form/download/{id}`.
#[derive(Clone, Default)]
pub struct AppState {
	pdfs: Arc<Mutex<HashMap<String, StoredPdf>>>,
}

impl AppState {
	pub fn new() -> Self {
		Self::default()
	}

	/// Stores `bytes` and returns the single-use download id.
	pub fn insert_pdf(&self, bytes: Vec<u8>) -> String {
		let id = uuid::Uuid::new_v4().to_string();
		let mut pdfs = self.pdfs.lock().unwrap_or_else(|e| e.into_inner());

		pdfs.retain(|_, pdf| pdf.created.elapsed() < PDF_TTL);
		while pdfs.len() >= MAX_STORED_PDFS {
			let oldest = pdfs
				.iter()
				.min_by_key(|(_, pdf)| pdf.created)
				.map(|(id, _)| id.clone());
			match oldest {
				Some(id) => {
					pdfs.remove(&id);
				}
				None => break,
			}
		}

		pdfs.insert(
			id.clone(),
			StoredPdf {
				bytes,
				created: Instant::now(),
			},
		);
		id
	}

	/// Removes and returns the PDF for `id` if it exists and has not expired.
	pub fn take_pdf(&self, id: &str) -> Option<Vec<u8>> {
		let mut pdfs = self.pdfs.lock().unwrap_or_else(|e| e.into_inner());
		let pdf = pdfs.remove(id)?;
		(pdf.created.elapsed() < PDF_TTL).then_some(pdf.bytes)
	}
}
