//! What the uploaded certificate actually is, read from its own first bytes.
//!
//! Server-only, and for the same reason as [`super::fiscal_code`]: no regex and no `accept`
//! attribute can express it. The browser's declared content type is a claim, not a fact —
//! it is chosen by the phone's file picker, and two of its habits used to reach the
//! association's mailbox unopposed:
//!
//! - an iPhone photo taken out of Files rather than the photo library arrives as `image/heic`,
//!   which is a truthful claim about a file most recipients cannot open at all;
//! - some Android WebViews send `application/octet-stream` for a perfectly ordinary JPEG,
//!   which the old declared-type gate refused.
//!
//! So the claim is ignored and the bytes are asked instead. This is not a security boundary —
//! nothing on this server decodes the file — it is the one check that can promise the person
//! who opens the email that the attachment will open.
//!
//! The formats in [`Kind::openable`] are the ones Windows, macOS and the common webmail
//! clients handle without installing anything. The rest are recognized on purpose rather
//! than lumped into [`Kind::Unknown`], because naming the format is what lets the applicant
//! be told how to get out of it.

/// A format the certificate may arrive in, identified by its own bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
	Jpeg,
	Png,
	Pdf,
	Gif,
	Bmp,
	Webp,
	Tiff,
	/// Apple's default camera format, and the reason this module exists.
	Heic,
	Avif,
	/// Recognized so it can be refused: an SVG opens, but it is a script the recipient would
	/// be running, not a photograph of a certificate.
	Svg,
	Unknown,
}

impl Kind {
	/// Whether the person who receives the email can open it without installing a codec.
	pub fn openable(self) -> bool {
		matches!(
			self,
			Self::Jpeg | Self::Png | Self::Pdf | Self::Gif | Self::Bmp | Self::Webp | Self::Tiff
		)
	}

	/// The extension the attachment should carry, whatever the phone called it.
	pub fn extension(self) -> Option<&'static str> {
		Some(match self {
			Self::Jpeg => "jpg",
			Self::Png => "png",
			Self::Pdf => "pdf",
			Self::Gif => "gif",
			Self::Bmp => "bmp",
			Self::Webp => "webp",
			Self::Tiff => "tif",
			_ => return None,
		})
	}

	/// The MIME type to attach it as, replacing whatever the browser claimed.
	pub fn mime(self) -> &'static str {
		match self {
			Self::Jpeg => "image/jpeg",
			Self::Png => "image/png",
			Self::Pdf => "application/pdf",
			Self::Gif => "image/gif",
			Self::Bmp => "image/bmp",
			Self::Webp => "image/webp",
			Self::Tiff => "image/tiff",
			Self::Heic => "image/heic",
			Self::Avif => "image/avif",
			Self::Svg => "image/svg+xml",
			Self::Unknown => "application/octet-stream",
		}
	}

	/// A short name for the logs, so the next unopenable file is one grep away rather than
	/// something to reconstruct backwards from the mailbox.
	pub fn label(self) -> &'static str {
		match self {
			Self::Jpeg => "jpeg",
			Self::Png => "png",
			Self::Pdf => "pdf",
			Self::Gif => "gif",
			Self::Bmp => "bmp",
			Self::Webp => "webp",
			Self::Tiff => "tiff",
			Self::Heic => "heic",
			Self::Avif => "avif",
			Self::Svg => "svg",
			Self::Unknown => "unknown",
		}
	}

	/// Why this file cannot be accepted, phrased so the applicant knows what to do next.
	///
	/// `None` for the formats that are fine. The HEIC sentence is the important one: it is the
	/// only rejection here an applicant reaches by doing nothing wrong, so it has to name the
	/// way out rather than the problem.
	pub fn refusal(self) -> Option<&'static str> {
		Some(match self {
			Self::Heic => {
				"Il telefono ha inviato la foto in formato HEIC, che chi la riceve non riesce \
				 ad aprire. Riprova scegliendo la foto dalla «Libreria foto» invece che da \
				 «Sfoglia»/«File», oppure carica un PDF."
			}
			Self::Avif => {
				"Il telefono ha inviato la foto in formato AVIF, che chi la riceve non riesce \
				 ad aprire. Riprova scegliendo la foto dalla «Libreria foto», oppure carica \
				 un PDF."
			}
			Self::Svg => "Carica una foto o un PDF: un disegno SVG non va bene come certificato.",
			Self::Unknown => {
				"Non riusciamo a leggere questo file: potrebbe essersi danneggiato durante il \
				 caricamento. Riprova, oppure scatta una nuova foto."
			}
			_ => return None,
		})
	}
}

/// Reads the leading bytes and says what the file is.
///
/// Order matters in one place only: the ISO base media container is shared by HEIC and AVIF,
/// which are told apart by the brand that follows `ftyp` rather than by the header itself.
pub fn sniff(bytes: &[u8]) -> Kind {
	if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
		return Kind::Jpeg;
	}
	if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
		return Kind::Png;
	}
	if bytes.starts_with(b"%PDF-") {
		return Kind::Pdf;
	}
	if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
		return Kind::Gif;
	}
	if bytes.starts_with(b"BM") {
		return Kind::Bmp;
	}
	// RIFF says "some container"; the four bytes at offset 8 say which one.
	if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
		return Kind::Webp;
	}
	if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
	{
		return Kind::Tiff;
	}
	if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
		return match &bytes[8..12] {
			// Every brand Apple stamps on a still photo or a burst, plus the two generic
			// HEIF ones a third-party app may write.
			b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"hevm" | b"hevs"
			| b"mif1" | b"msf1" => Kind::Heic,
			b"avif" | b"avis" => Kind::Avif,
			_ => Kind::Unknown,
		};
	}
	// Text, so it may carry a byte-order mark, a declaration or a comment before the tag.
	// Only the start is examined: a well-formed SVG has to open its root element early.
	let head = bytes.get(..512).unwrap_or(bytes);
	if let Ok(text) = std::str::from_utf8(head) {
		let text = text.trim_start_matches('\u{feff}').trim_start();
		if text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg")) {
			return Kind::Svg;
		}
	}
	Kind::Unknown
}

/// Puts `kind`'s own extension on `filename`, and says whether that changed anything.
///
/// The attachment has to be named for what it is: a HEIC called `.jpg` is exactly the file
/// this module was written to stop, and the mirror case — a JPEG the picker called
/// `image.tmp` — reaches a mail client that then refuses to preview it. An extension that
/// already agrees is left alone, including its case, so the usual submission arrives with
/// the name the applicant recognizes.
pub fn with_extension(filename: &str, kind: Kind) -> (String, bool) {
	let Some(extension) = kind.extension() else {
		return (filename.to_string(), false);
	};

	// `jpeg` and `tiff` are the same file as `jpg` and `tif`; rewriting those would be churn
	// the applicant notices for no gain.
	let equivalent = |current: &str| {
		let current = current.to_ascii_lowercase();
		current == extension
			|| matches!(
				(extension, current.as_str()),
				("jpg", "jpeg") | ("tif", "tiff")
			)
	};

	match filename.rsplit_once('.') {
		Some((_, current)) if equivalent(current) => (filename.to_string(), false),
		// A dotfile has no stem to keep, so its extension is appended rather than replaced.
		Some((stem, _)) if !stem.trim().is_empty() => (format!("{stem}.{extension}"), true),
		_ => (format!("{filename}.{extension}"), true),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Every variant, so the invariant below is checked against the whole enum rather than
	/// against whichever formats the other tests happen to mention.
	const ALL: [Kind; 11] = [
		Kind::Jpeg,
		Kind::Png,
		Kind::Pdf,
		Kind::Gif,
		Kind::Bmp,
		Kind::Webp,
		Kind::Tiff,
		Kind::Heic,
		Kind::Avif,
		Kind::Svg,
		Kind::Unknown,
	];

	/// [`Kind::openable`] and [`Kind::refusal`] are two spellings of one decision, and the
	/// handler only consults the second. A format added to the first without a sentence in the
	/// second would be silently refused with no message; the reverse would refuse a format the
	/// enum calls fine. Neither would fail any other test here.
	#[test]
	fn openable_and_refusal_cannot_disagree() {
		for kind in ALL {
			assert_eq!(
				kind.openable(),
				kind.refusal().is_none(),
				"{} says openable={} but refusal={:?}",
				kind.label(),
				kind.openable(),
				kind.refusal()
			);
			// An accepted format has to be able to name itself, since the attachment is
			// renamed after it.
			assert_eq!(
				kind.openable(),
				kind.extension().is_some(),
				"{} cannot be accepted without an extension to carry",
				kind.label()
			);
		}
	}

	/// The minimum prefix each format is recognized by. Short on purpose: a phone that
	/// truncates a photo still has to be identified, not lumped into `Unknown`.
	#[test]
	fn recognizes_every_format_it_claims_to() {
		assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Kind::Jpeg);
		assert_eq!(sniff(b"\x89PNG\r\n\x1a\n\x00"), Kind::Png);
		assert_eq!(sniff(b"%PDF-1.7\n"), Kind::Pdf);
		assert_eq!(sniff(b"GIF89a..."), Kind::Gif);
		assert_eq!(sniff(b"BM\x00\x00"), Kind::Bmp);
		assert_eq!(sniff(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Kind::Webp);
		assert_eq!(sniff(&[0x49, 0x49, 0x2A, 0x00, 0x08]), Kind::Tiff);
		assert_eq!(sniff(&[0x4D, 0x4D, 0x00, 0x2A, 0x00]), Kind::Tiff);
	}

	/// The whole point of the module: an iPhone photo out of Files, which used to be
	/// forwarded to the association as an attachment nobody could open.
	#[test]
	fn an_iphone_photo_is_refused_by_name() {
		let heic = b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00";
		let kind = sniff(heic);

		assert_eq!(kind, Kind::Heic);
		assert!(!kind.openable());
		let refusal = kind.refusal().expect("HEIC has to say something");
		assert!(
			refusal.contains("Libreria foto"),
			"the way out has to be named: {refusal}"
		);
	}

	/// `mif1` is what a third-party app stamps instead of `heic`, and `avif` shares the
	/// container with both. Getting the brand wrong would refuse the file with the other
	/// format's instructions.
	#[test]
	fn iso_container_brands_are_told_apart() {
		assert_eq!(
			sniff(b"\x00\x00\x00\x18ftypmif1\x00\x00\x00\x00"),
			Kind::Heic
		);
		assert_eq!(
			sniff(b"\x00\x00\x00\x18ftypavif\x00\x00\x00\x00"),
			Kind::Avif
		);
		// An MP4 is the same container: refused, but not as a photo format.
		assert_eq!(
			sniff(b"\x00\x00\x00\x18ftypisom\x00\x00\x00\x00"),
			Kind::Unknown
		);
	}

	#[test]
	fn svg_is_recognized_through_its_preamble() {
		assert_eq!(
			sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\">"),
			Kind::Svg
		);
		assert_eq!(
			sniff(b"\xef\xbb\xbf<?xml version=\"1.0\"?>\n<svg width=\"10\">"),
			Kind::Svg
		);
		// XML that is not an SVG has no business being identified as one.
		assert_eq!(sniff(b"<?xml version=\"1.0\"?><rss></rss>"), Kind::Unknown);
	}

	#[test]
	fn empty_and_tiny_inputs_do_not_panic() {
		assert_eq!(sniff(b""), Kind::Unknown);
		assert_eq!(sniff(b"R"), Kind::Unknown);
		assert_eq!(sniff(b"RIFF"), Kind::Unknown);
		assert_eq!(sniff(&[0xFF, 0xD8]), Kind::Unknown);
	}

	/// A lie about the extension is corrected; the truth is left untouched, case and all.
	#[test]
	fn the_extension_follows_the_bytes() {
		assert_eq!(
			with_extension("IMG_1234.jpg", Kind::Png),
			("IMG_1234.png".to_string(), true)
		);
		assert_eq!(
			with_extension("IMG_1234.jpg", Kind::Jpeg),
			("IMG_1234.jpg".to_string(), false)
		);
		// Same format under another spelling: not worth a rename.
		assert_eq!(
			with_extension("scan.JPEG", Kind::Jpeg),
			("scan.JPEG".to_string(), false)
		);
		assert_eq!(
			with_extension("scan.JPG", Kind::Jpeg),
			("scan.JPG".to_string(), false)
		);
		// No extension at all, which is what the `certificato` fallback carries.
		assert_eq!(
			with_extension("certificato", Kind::Jpeg),
			("certificato.jpg".to_string(), true)
		);
		// A refused format has no extension to offer, so the name is left as it came.
		assert_eq!(
			with_extension("photo.jpg", Kind::Heic),
			("photo.jpg".to_string(), false)
		);
	}

	/// Names that would otherwise produce an empty stem or a doubled dot.
	#[test]
	fn odd_names_still_come_out_usable() {
		assert_eq!(
			with_extension("certificato.", Kind::Pdf),
			("certificato.pdf".to_string(), true)
		);
		assert_eq!(
			with_extension(".gitignore", Kind::Jpeg),
			(".gitignore.jpg".to_string(), true)
		);
		assert_eq!(with_extension("", Kind::Jpeg), (".jpg".to_string(), true));
	}
}
