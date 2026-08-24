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
		static LOGO_SHINE: &[u8] = include_bytes!("../../../static/web-app-manifest-512x512.png");
		static LOGO_UISP: &[u8] = include_bytes!("../../../static/logo_uisp.png");

		let path = std::path::Path::new(id.vpath().get_without_slash());
		match path.file_name().and_then(|n| n.to_str()) {
			Some("signature.png") => Ok(self.signature_bytes.clone()),
			Some("signature2.png") => self
				.autonomy_signature_bytes
				.clone()
				.ok_or_else(|| FileError::NotFound(path.to_path_buf())),
			Some("logo_shine.png") => Ok(Bytes::new(LOGO_SHINE.to_vec())),
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

/// Escapes user input interpolated into a Typst *markup* block (`#form_data[…]`).
///
/// Left unescaped, `@` starts a reference, `#` enters code mode, `$` opens math, 
/// and an odd `]` closes the enclosing block early — each aborts compilation.
fn typst_escape(s: String) -> String {
	let mut out = String::with_capacity(s.len());
	for c in s.chars() {
		if matches!(
			c,
			'\\' | '@' | '#' | '$' | '[' | ']' | '*' | '_' | '<' | '>' | '`' | '~' | '='
		) {
			out.push('\\');
		}
		out.push(c);
	}
	out
}

fn typst_escape_opt(s: Option<String>) -> Option<String> {
	s.map(typst_escape)
}

/// Escapes user input interpolated inside a Typst *string literal* (`"…"`),
/// where only the backslash and the closing quote are significant.
fn typst_escape_str(s: String) -> String {
	s.replace('\\', r"\\").replace('"', "\\\"")
}

fn typst_escape_str_opt(s: Option<String>) -> Option<String> {
	s.map(typst_escape_str)
}

/// True when the browser sent a non-empty canvas data URL.
fn is_drawn_signature(value: &str) -> bool {
	!value.trim().is_empty() && value.contains(',')
}

pub fn generate(mut form: MembershipForm) -> Result<Vec<u8>, String> {
	// The conditional sections are only hidden by CSS `:has()`, so their inputs
	// are still submitted. Clear whatever the toggles say does not apply, rather
	// than trusting the browser to have blanked the fields.
	if form.is_minor.is_none() {
		form.minor_last_name = None;
		form.minor_first_name = None;
		form.minor_birth_place = None;
		form.minor_birth_province = None;
		form.minor_birth_date = None;
		form.minor_residence_city = None;
		form.minor_residence_address = None;
		form.minor_residence_number = None;
		form.minor_residence_cap = None;
		form.minor_residence_province = None;
		form.minor_fiscal_code = None;
		form.commute_alone = None;
	}

	if form.commute_alone.is_none() {
		form.autonomy_signature = None;
		form.autonomy_place_and_date = None;
	}

	form.autonomy_signature = form.autonomy_signature.filter(|s| is_drawn_signature(s));
	form.autonomy_place_and_date = form
		.autonomy_place_and_date
		.filter(|s| !s.trim().is_empty());

	// An empty main signature would decode to zero bytes and only fail later, as
	// an opaque Typst image error.
	if !is_drawn_signature(&form.signature) {
		return Err("Firma del richiedente mancante".to_string());
	}

	form.last_name = typst_escape(form.last_name);
	form.first_name = typst_escape(form.first_name);
	form.birth_place = typst_escape(form.birth_place);
	form.birth_province = typst_escape(form.birth_province);
	form.birth_date = typst_escape(form.birth_date);
	form.residence_city = typst_escape(form.residence_city);
	form.residence_address = typst_escape(form.residence_address);
	form.residence_number = typst_escape(form.residence_number);
	form.residence_cap = typst_escape(form.residence_cap);
	form.residence_province = typst_escape(form.residence_province);
	form.phone = typst_escape(form.phone);
	form.email = typst_escape(form.email);
	form.fiscal_code = typst_escape(form.fiscal_code);
	// These two land inside Typst string literals, not markup blocks.
	form.place_and_date = typst_escape_str(form.place_and_date);
	form.minor_last_name = typst_escape_opt(form.minor_last_name);
	form.minor_first_name = typst_escape_opt(form.minor_first_name);
	form.minor_birth_place = typst_escape_opt(form.minor_birth_place);
	form.minor_birth_province = typst_escape_opt(form.minor_birth_province);
	form.minor_birth_date = typst_escape_opt(form.minor_birth_date);
	form.minor_residence_city = typst_escape_opt(form.minor_residence_city);
	form.minor_residence_address = typst_escape_opt(form.minor_residence_address);
	form.minor_residence_number = typst_escape_opt(form.minor_residence_number);
	form.minor_residence_cap = typst_escape_opt(form.minor_residence_cap);
	form.minor_residence_province = typst_escape_opt(form.minor_residence_province);
	form.minor_fiscal_code = typst_escape_opt(form.minor_fiscal_code);
	form.autonomy_place_and_date = typst_escape_str_opt(form.autonomy_place_and_date);

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
