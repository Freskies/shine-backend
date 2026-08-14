use super::templates::Form;
use askama::Template;
use base64::Engine;
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst_layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

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
			// fontdb restituisce Option<Option<Font>>
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

// Implementazione del contesto World in-memory di Typst
struct InMemoryWorld {
	compiler: &'static TypstCompiler,
	main_source: Source,
	signature_bytes: Bytes,
}

impl InMemoryWorld {
	fn new(text: String, signature_raw: &[u8]) -> Self {
		let compiler = COMPILER.get_or_init(TypstCompiler::init);
		let main_source = Source::detached(text);
		Self {
			compiler,
			main_source,
			signature_bytes: Bytes::new(signature_raw.to_vec()),
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
			Err(FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
		}
	}

	fn file(&self, id: FileId) -> FileResult<Bytes> {
		let path = id.vpath().as_rootless_path();
		if path.file_name().and_then(|n| n.to_str()) == Some("signature.png") {
			Ok(self.signature_bytes.clone())
		} else {
			Err(FileError::NotFound(path.to_path_buf()))
		}
	}

	fn font(&self, id: usize) -> Option<Font> {
		self.compiler.fonts.get(id).cloned()
	}

	fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
		None
	}
}

pub fn generate(form: Form) -> Result<Vec<u8>, String> {
	// 1. Decodifica la firma Base64 inviata dal Canvas
	let clean_sig = form.signature.split(',').nth(1).unwrap_or(&form.signature);
	let signature_bytes = base64::engine::general_purpose::STANDARD
		.decode(clean_sig)
		.map_err(|e| format!("Errore decodifica Base64 firma: {e}"))?;

	// 2. Renderizza il testo Typst tramite Askama
	let typst_markup = form.render().map_err(|e| e.to_string())?;

	// 3. Inizializza il World in-memory
	let world = InMemoryWorld::new(typst_markup, &signature_bytes);

	// 4. Compila il documento Typst in PDF
	let document: PagedDocument = typst::compile(&world)
		.output
		.map_err(|errors| format!("Errore compilazione Typst: {:?}", errors))?;

	let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
		.map_err(|e| format!("Errore esportazione PDF: {:?}", e))?;

	Ok(pdf_bytes)
}