//! Throwaway checker: compiles a .typ file with the crates this project already vendors.
//! Usage: cargo run --example compile_typ -- playground/MATH.typ out.pdf
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

struct FsWorld {
	root: PathBuf,
	library: LazyHash<Library>,
	book: LazyHash<FontBook>,
	fonts: Vec<Font>,
	main: FileId,
	cache: Mutex<HashMap<FileId, Source>>,
}

impl FsWorld {
	fn new(root: PathBuf, main_rel: &str) -> Self {
		let mut db = fontdb::Database::new();
		db.load_system_fonts();
		let mut fonts = Vec::new();
		for face in db.faces() {
			if let Some(Some(f)) = db.with_face_data(face.id, |d, i| Font::new(Bytes::new(d.to_vec()), i)) {
				fonts.push(f);
			}
		}
		Self {
			root,
			library: LazyHash::new(Library::builder().build()),
			book: LazyHash::new(FontBook::from_fonts(&fonts)),
			fonts,
			main: RootedPath::new(
				VirtualRoot::Project,
				VirtualPath::new(main_rel).expect("valid virtual path"),
			)
			.intern(),
			cache: Mutex::new(HashMap::new()),
		}
	}
	fn resolve(&self, id: FileId) -> PathBuf {
		self.root.join(id.vpath().get_without_slash())
	}
}

impl World for FsWorld {
	fn library(&self) -> &LazyHash<Library> { &self.library }
	fn book(&self) -> &LazyHash<FontBook> { &self.book }
	fn main(&self) -> FileId { self.main }
	fn source(&self, id: FileId) -> FileResult<Source> {
		if let Some(s) = self.cache.lock().unwrap().get(&id) { return Ok(s.clone()); }
		let path = self.resolve(id);
		let text = std::fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
		let src = Source::new(id, text);
		self.cache.lock().unwrap().insert(id, src.clone());
		Ok(src)
	}
	fn file(&self, id: FileId) -> FileResult<Bytes> {
		let path = self.resolve(id);
		std::fs::read(&path).map(Bytes::new).map_err(|e| FileError::from_io(e, &path))
	}
	fn font(&self, i: usize) -> Option<Font> { self.fonts.get(i).cloned() }
	fn today(&self, _: Option<typst::foundations::Duration>) -> Option<Datetime> { None }
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let input = args.get(1).expect("usage: compile_typ <file.typ> [out.pdf]");
	let out = args.get(2).cloned().unwrap_or_else(|| "out.pdf".into());

	let world = FsWorld::new(std::env::current_dir().unwrap(), input);
	let res = typst::compile::<PagedDocument>(&world);

	for w in res.warnings.iter() {
		println!("WARN  {}", w.message);
	}
	match res.output {
		Ok(doc) => {
			println!("OK    compiled, {} page(s)", doc.pages().len());
			let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).expect("pdf");
			std::fs::write(&out, &pdf).unwrap();
			println!("OK    wrote {} ({} bytes)", out, pdf.len());
		}
		Err(errs) => {
			println!("FAILED with {} error(s):", errs.len());
			for e in errs.iter() {
				println!("  ERROR {}", e.message);
				for h in e.hints.iter() { println!("        hint: {}", h.v); }
			}
			std::process::exit(1);
		}
	}
}
