use super::templates::MembershipForm;
use askama::Template;
use base64::Engine;
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

static COMPILER: OnceLock<TypstCompiler> = OnceLock::new();

struct TypstCompiler {
	library: LazyHash<Library>,
	book: LazyHash<FontBook>,
	fonts: Vec<Font>,
}

impl TypstCompiler {
	fn init() -> Self {
		let mut db = fontdb::Database::new();
		db.load_system_fonts();

		let mut fonts = Vec::new();
		for face in db.faces() {
			if let Some(Some(font)) = db.with_face_data(face.id, |data, face_index| {
				Font::new(Bytes::new(data.to_vec()), face_index)
			}) {
				fonts.push(font);
			}
		}

		let book = LazyHash::new(FontBook::from_fonts(&fonts));
		let library = LazyHash::new(Library::builder().build());

		Self {
			library,
			book,
			fonts,
		}
	}
}

struct InMemoryWorld {
	compiler: &'static TypstCompiler,
	main_source: Source,
	signature_bytes: Bytes,
	autonomy_signature_bytes: Option<Bytes>,
}

impl InMemoryWorld {
	fn new(text: String, signature_raw: &[u8], autonomy_signature_raw: Option<&[u8]>) -> Self {
		let compiler = COMPILER.get_or_init(TypstCompiler::init);
		let main_source = Source::detached(text);
		Self {
			compiler,
			main_source,
			signature_bytes: Bytes::new(signature_raw.to_vec()),
			autonomy_signature_bytes: autonomy_signature_raw.map(|raw| Bytes::new(raw.to_vec())),
		}
	}
}

impl World for InMemoryWorld {
	fn library(&self) -> &LazyHash<Library> {
		&self.compiler.library
	}

	fn book(&self) -> &LazyHash<FontBook> {
		&self.compiler.book
	}

	fn main(&self) -> FileId {
		self.main_source.id()
	}

	fn source(&self, id: FileId) -> FileResult<Source> {
		if id == self.main_source.id() {
			Ok(self.main_source.clone())
		} else {
			Err(FileError::NotFound(std::path::PathBuf::from(
				id.vpath().get_without_slash(),
			)))
		}
	}

	fn file(&self, id: FileId) -> FileResult<Bytes> {
		static LOGO_SHINE: &[u8] =
			include_bytes!("../../../static/web-app-manifest-512x512.png");
		static LOGO_UISP: &[u8] = include_bytes!("../../../static/logo_uisp.png");

		let path = std::path::Path::new(id.vpath().get_without_slash());
		match path.file_name().and_then(|n| n.to_str()) {
			Some("signature.png") => Ok(self.signature_bytes.clone()),
			Some("signature2.png") => self
				.autonomy_signature_bytes
				.clone()
				.ok_or_else(|| FileError::NotFound(path.to_path_buf())),
			Some("web-app-manifest-512x512.png") => Ok(Bytes::new(LOGO_SHINE.to_vec())),
			Some("logo_uisp.png") => Ok(Bytes::new(LOGO_UISP.to_vec())),
			_ => Err(FileError::NotFound(path.to_path_buf())),
		}
	}

	fn font(&self, id: usize) -> Option<Font> {
		self.compiler.fonts.get(id).cloned()
	}

	fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
		None
	}
}

fn decode_signature(data_url: &str) -> Result<Vec<u8>, String> {
	let clean_sig = data_url.split(',').nth(1).unwrap_or(data_url);
	base64::engine::general_purpose::STANDARD
		.decode(clean_sig)
		.map_err(|e| format!("Errore decodifica Base64 firma: {e}"))
}

pub fn generate(mut form: MembershipForm) -> Result<Vec<u8>, String> {
	form.autonomy_signature = form
		.autonomy_signature
		.filter(|s| !s.trim().is_empty() && s.contains(','));
	form.autonomy_place_and_date = form
		.autonomy_place_and_date
		.filter(|s| !s.trim().is_empty());

	let signature_bytes = decode_signature(&form.signature)?;
	let autonomy_signature_bytes = form
		.autonomy_signature
		.as_deref()
		.map(decode_signature)
		.transpose()?;

	let typst_markup = form.render().map_err(|e| e.to_string())?;

	let world = InMemoryWorld::new(
		typst_markup,
		&signature_bytes,
		autonomy_signature_bytes.as_deref(),
	);

	let document: PagedDocument = typst::compile(&world)
		.output
		.map_err(|errors| format!("Errore compilazione Typst: {:?}", errors))?;

	let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
		.map_err(|e| format!("Errore esportazione PDF: {:?}", e))?;

	Ok(pdf_bytes)
}
