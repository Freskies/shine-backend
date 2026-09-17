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

/// Whether `needle` appears anywhere in `haystack`.
///
/// Searched from the end, for the one caller left: the PDF tail, where a `%%EOF` that is there
/// at all is in the last handful of bytes.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
	haystack.len() >= needle.len() && haystack.windows(needle.len()).rev().any(|w| w == needle)
}

/// What the bytes say about whether all of the file arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
	Complete,
	/// The file stops before the format says it should. Carries the reason, for the log line:
	/// the first false refusal has to be diagnosable from a log rather than from the applicant's
	/// mailbox.
	Truncated(&'static str),
	/// The structure could not be followed to the end. Not the same claim as [`Self::Truncated`]
	/// and deliberately not treated as one — see [`completeness`].
	Unparsable(&'static str),
}

/// What an applicant is told about a file that stops early.
///
/// One sentence for every format, because the cause is the same one every time and it is not
/// about the format: the phone handed over part of a file.
pub const INCOMPLETE_FILE: &str = "Il file è arrivato incompleto: il telefono ne ha inviato \
	 solo una parte. Se hai scelto la foto da Google Foto, aprila prima nell'app per scaricarla \
	 sul telefono, poi riprova — oppure scattane una nuova.";

/// Whether the file ends the way its own format says it must.
///
/// [`sniff`] reads the first bytes, and a half-written file has those too: a photo handed over
/// while the gallery was still transcoding it — or one Google Foto had not finished downloading
/// from the cloud — opens with a perfectly valid header and then simply stops. Two of them
/// reached the association as grey half-images, because every check on the way in had only ever
/// looked at the front of the file.
///
/// The end marker is found by *walking the format*, and the walk stops at the first one that is
/// structurally the end. Asking whether the marker appears anywhere — which is what this did
/// until it was found not to work — cannot answer the question on the files this form actually
/// receives: every phone-camera JPEG carries a complete thumbnail JPEG inside its EXIF segment,
/// ending in `FF D9` within the first tens of KB, so a photo cut off mid-scan produced a match
/// and was accepted. A linearized PDF is the same trap with `%%EOF`, which it repeats right
/// after the first-page cross-reference near the *front* of the file. The whole-file search was
/// not gratuitous, though, and what replaces it has to keep what it was for: Motion Photo — the
/// Android camera default — appends an MP4 after the real EOI, and Apple and Samsung append
/// whole secondary JPEGs, so the marker is nowhere near the end of a perfectly good photo.
/// Stopping at the first structural EOI is what makes both cases come out right.
///
/// Only JPEG, PNG and PDF are walked. They are what arrives, and each says unambiguously where
/// it ends; BMP and TIFF carry their extent in the header instead, and a GIF trailer is a single
/// `0x3B` byte that trailing padding would fake. A format with nothing reliable to follow is
/// reported [`Completeness::Complete`] rather than guessed at, and so is one whose structure
/// does not parse: a false refusal blocks an enrolment that was fine, which is worse than the
/// file it would catch.
pub fn completeness(bytes: &[u8], kind: Kind) -> Completeness {
	match kind {
		Kind::Jpeg => jpeg(bytes),
		Kind::Png => png(bytes),
		Kind::Pdf => pdf(bytes),
		_ => Completeness::Complete,
	}
}

/// Walks the JPEG marker structure until the image ends, or until it cannot go on.
fn jpeg(bytes: &[u8]) -> Completeness {
	// `sniff` matched `FF D8 FF` at offset 0, so the SOI is right at the front. If it is ever
	// loosened to hunt for an SOI further in, this has to start at the one it found.
	let mut cursor = 2;

	// Every turn of this loop advances the cursor by at least two bytes, so no real file needs
	// more turns than that. The cap is a backstop for a branch that ever stops advancing: this
	// runs inline in the request task, over up to 12 MB of bytes nobody here controls, where a
	// non-advancing branch is a hung handler rather than a wrong answer.
	for _ in 0..bytes.len() / 2 + 16 {
		// A marker is `FF` followed by the byte naming it, with any number of `FF` fill bytes in
		// between (T.81 B.1.1.2).
		match bytes.get(cursor) {
			None => return Completeness::Truncated("jpeg ends where a marker should start"),
			Some(0xFF) => {}
			Some(_) => return Completeness::Unparsable("jpeg is not at a marker"),
		}
		let mut at = cursor + 1;
		while bytes.get(at) == Some(&0xFF) {
			at += 1;
		}
		let Some(&marker) = bytes.get(at) else {
			return Completeness::Truncated("jpeg ends on a marker with no name");
		};
		// Where this marker's own bytes end: its length, if it has one, starts here.
		let payload = at + 1;

		let next = match marker {
			// The answer this walk exists to give. Everything after it — an appended MP4, a
			// second JPEG, padding — is somebody else's file riding along, and none of it is
			// this check's business.
			0xD9 => return Completeness::Complete,
			// Payload-less: TEM and the restart markers.
			0x01 | 0xD0..=0xD7 => payload,
			// A second SOI is either two files concatenated or a desync; walking into it would
			// end up reporting the inner image's truncation as the outer one's.
			0xD8 => return Completeness::Unparsable("jpeg carries a second SOI"),
			// `FF 00` is a stuffed byte, which means the cursor is in image data, not on a
			// marker.
			0x00 => return Completeness::Unparsable("jpeg desynced onto a stuffed byte"),
			// Arithmetic coding (SOF9–SOF11, SOF13–SOF15) and JPEG-LS do not stuff `FF 00`, so
			// the rule `scan_end` relies on does not hold for them and their scan cannot be
			// walked. Neither reaches this form in practice, and guessing would refuse a file
			// that opens.
			0xC9..=0xCB | 0xCD..=0xCF | 0xF7 => {
				return Completeness::Unparsable("jpeg is not huffman-coded");
			}
			// Start of scan: a header of its own, and then the entropy-coded data, which is not
			// length-prefixed and has to be scanned through.
			0xDA => {
				let header = match segment_end(bytes, payload) {
					Ok(end) => end,
					Err(why) => return why,
				};
				match scan_end(bytes, header) {
					Some(end) => end,
					None => return Completeness::Truncated("jpeg scan ends without a marker"),
				}
			}
			// Everything else carries a length and is skipped whole — which is what makes the
			// EXIF thumbnail invisible: APP1 is capped at 65533 bytes, so a thumbnail is always
			// inside one segment, and its `FF D9` is never looked at.
			_ => match segment_end(bytes, payload) {
				Ok(end) => end,
				Err(why) => return why,
			},
		};

		debug_assert!(next > cursor, "the jpeg walk stopped advancing at {cursor}");
		cursor = next;
	}

	Completeness::Unparsable("jpeg walk did not terminate")
}

/// Where the segment whose two-byte length starts at `at` ends, or why it cannot be skipped.
fn segment_end(bytes: &[u8], at: usize) -> Result<usize, Completeness> {
	let Some(length) = bytes.get(at..at + 2) else {
		return Err(Completeness::Truncated("jpeg ends inside a segment length"));
	};
	let length = u16::from_be_bytes([length[0], length[1]]) as usize;

	// `Ls` counts its own two bytes, so two is the smallest a real one can be — and a smaller
	// value would leave the cursor where it was.
	if length < 2 {
		return Err(Completeness::Unparsable(
			"jpeg segment declares an impossible length",
		));
	}
	let end = at + length;
	if end > bytes.len() {
		return Err(Completeness::Truncated("jpeg segment overruns the buffer"));
	}
	Ok(end)
}

/// Where the entropy-coded data starting at `from` ends: the `FF` of the next marker.
///
/// Inside a Huffman scan a literal `FF` is stored as `FF 00` (T.81 B.1.1.5), and `FF D0`–`FF D7`
/// are restart markers, so neither ends the scan. In theory everything else does — but a corrupt
/// pair of bytes reading as a marker would then be skipped by a "length" that is really image
/// data, and the file refused for it. So only the markers that may legally follow a scan are
/// honored, and anything else is taken for what it almost certainly is: data.
fn scan_end(bytes: &[u8], from: usize) -> Option<usize> {
	let mut at = from;
	while at + 1 < bytes.len() {
		if bytes[at] != 0xFF {
			at += 1;
			continue;
		}
		let ends_the_scan = matches!(
			bytes[at + 1],
			// EOI, and the tables or frame header a further scan brings with it. A progressive
			// image has several scans, with DHT/DQT/DRI/COM/APPn between them.
			0xD9 | 0xC0..=0xCF | 0xDA | 0xDB | 0xDC | 0xDD | 0xDF | 0xE0..=0xEF | 0xFE
		);
		if ends_the_scan {
			return Some(at);
		}
		at += 1;
	}
	None
}

/// Walks the PNG chunk chain to `IEND`.
fn png(bytes: &[u8]) -> Completeness {
	// Past the eight-byte signature `sniff` matched.
	let mut cursor = 8;

	loop {
		let Some(header) = bytes.get(cursor..cursor + 8) else {
			return Completeness::Truncated("png ends inside a chunk header");
		};
		let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
		let name = &header[4..8];

		// The spec caps a chunk at `2^31 - 1` and names it in four letters. Outside that, the
		// number is not a length and the walk has nothing to follow — which is not the same as
		// a file that arrived short.
		if length > 0x7FFF_FFFF {
			return Completeness::Unparsable("png chunk declares an impossible length");
		}
		if !name.iter().all(u8::is_ascii_alphabetic) {
			return Completeness::Unparsable("png chunk is not named in four letters");
		}
		// Reached by walking, so a `tEXt` or `eXIf` chunk whose *data* spells IEND — or an
		// `IDAT` whose compressed bytes happen to, which is a coin flip away on a big photo —
		// cannot stand in for it.
		if name == b"IEND" {
			return Completeness::Complete;
		}

		// Header, data, and the four CRC bytes this walk does not verify: nothing here decodes
		// the image, and whether the data is *correct* is not the question being asked.
		let Some(next) = (cursor + 8).checked_add(length as usize + 4) else {
			return Completeness::Unparsable("png chunk length does not fit an address");
		};
		if next > bytes.len() {
			return Completeness::Truncated("png chunk overruns the buffer");
		}

		debug_assert!(next > cursor, "the png walk stopped advancing at {cursor}");
		cursor = next;
	}
}

/// Whether the PDF still has the `%%EOF` that belongs on its last line.
fn pdf(bytes: &[u8]) -> Completeness {
	// Trailing whitespace and NUL padding first: a storage or mail layer can add some, and
	// truncation only ever removes bytes, so trimming costs no detection.
	let end = bytes
		.iter()
		.rposition(|byte| !matches!(byte, 0x00 | b'\n' | b'\r' | b' ' | b'\t' | 0x0C))
		.map_or(0, |at| at + 1);

	// ISO 32000-1 §7.5.5 puts `%%EOF` on the last line, and readers look for it in the final
	// 1024 bytes; the window is four times that, for the tools that append a little after it.
	// A window is the whole point: a linearized PDF — "fast web view", which is what scanners
	// and Acrobat produce — carries an earlier `%%EOF` right after the first-page
	// cross-reference, so asking the whole file passes one that was cut in half. Incremental
	// updates and signatures repeat it too, but theirs is the one at the end.
	let tail = &bytes[end.saturating_sub(4096)..end];
	if contains(tail, b"%%EOF") {
		Completeness::Complete
	} else {
		Completeness::Truncated("pdf has no %%EOF in its tail")
	}
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

	/// A segment: `FF`, the marker, the two-byte length that counts itself, and the payload.
	///
	/// Every fixture here is built from this rather than written out, because the walk follows
	/// the lengths now. The ones this replaced did not survive the change and could not: one of
	/// them declared an 8307-byte APP0 inside a 30-byte file — invented lengths that a
	/// whole-file search never had to notice.
	fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
		let mut out = vec![0xFF, marker];
		out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
		out.extend_from_slice(payload);
		out
	}

	/// A marker with nothing after it: SOI, EOI, RSTn, TEM.
	fn marker(marker: u8) -> Vec<u8> {
		vec![0xFF, marker]
	}

	/// Quantization table, frame header, Huffman table and the start-of-scan header: what every
	/// baseline photo puts between its metadata and its image data. The contents are filler —
	/// the walk reads the lengths and nothing else.
	fn tables_and_sos() -> Vec<u8> {
		[
			segment(0xDB, &[0; 65]),
			segment(
				0xC0,
				&[0x08, 0x0C, 0x00, 0x10, 0x00, 0x03, 0x01, 0x22, 0x00],
			),
			segment(0xC4, &[0; 29]),
			segment(
				0xDA,
				&[0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00],
			),
		]
		.concat()
	}

	/// Image data, carrying the two sequences that must not be taken for the end of it: a
	/// stuffed `FF 00`, and a restart marker.
	fn scan() -> Vec<u8> {
		vec![0x12, 0x34, 0xFF, 0x00, 0x56, 0xFF, 0xD0, 0x78, 0x9A]
	}

	/// The metadata a phone camera writes: an EXIF segment carrying a *complete thumbnail JPEG*,
	/// and the MPF segment announcing the secondary images appended after the primary one.
	fn camera_metadata() -> Vec<u8> {
		let thumbnail = [
			marker(0xD8),
			segment(0xDB, &[0; 65]),
			segment(0xC0, &[0x08, 0x00, 0x78, 0x00, 0xA0, 0x01, 0x11, 0x00]),
			segment(0xDA, &[0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]),
			vec![0xAB, 0xFF, 0x00, 0xCD],
			// The marker the check this replaced found and took for the photograph's own.
			marker(0xD9),
		]
		.concat();

		let mut exif = b"Exif\x00\x00".to_vec();
		// The TIFF header the IFDs hang off, then the thumbnail IFD1 points at.
		exif.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00]);
		exif.extend_from_slice(&thumbnail);

		[
			segment(0xE1, &exif),
			segment(0xE2, b"MPF\x00 two more images follow this one"),
		]
		.concat()
	}

	/// A whole photo as a phone hands it over.
	fn camera_photo() -> Vec<u8> {
		[
			marker(0xD8),
			camera_metadata(),
			tables_and_sos(),
			scan(),
			marker(0xD9),
		]
		.concat()
	}

	/// Length, type, data, and four bytes where the CRC goes. The walk does not verify it —
	/// nothing here decodes the image, and whether the data is *correct* is not the question —
	/// so the filler is deliberate and has to stay filler.
	fn png_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
		let mut out = (data.len() as u32).to_be_bytes().to_vec();
		out.extend_from_slice(name);
		out.extend_from_slice(data);
		out.extend_from_slice(b"crc0");
		out
	}

	fn png_file(chunks: &[Vec<u8>]) -> Vec<u8> {
		let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
		for chunk in chunks {
			out.extend_from_slice(chunk);
		}
		out
	}

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

		// The walks are handed the `kind` by their caller, so each has to hold up on bytes that
		// could never have produced it.
		for kind in [Kind::Jpeg, Kind::Png, Kind::Pdf] {
			assert!(matches!(
				completeness(b"", kind),
				Completeness::Truncated(_)
			));
			assert!(matches!(
				completeness(&[0xFF], kind),
				Completeness::Truncated(_) | Completeness::Unparsable(_)
			));
		}
	}

	/// The file that made this rewrite necessary, and the reason the check it replaced never
	/// stopped one: a photo cut off mid-scan, whose EXIF thumbnail is a whole JPEG and ends the
	/// way the photograph was supposed to.
	#[test]
	fn a_camera_photo_cut_mid_scan_is_caught_despite_its_exif_thumbnail() {
		let cut = [marker(0xD8), camera_metadata(), tables_and_sos(), scan()].concat();

		assert!(
			contains(&cut, &[0xFF, 0xD9]),
			"this is not the file that was getting through"
		);
		assert_eq!(sniff(&cut), Kind::Jpeg, "the header is intact, as it was");
		assert!(matches!(
			completeness(&cut, Kind::Jpeg),
			Completeness::Truncated(_)
		));

		// The same photo, all of it.
		assert_eq!(
			completeness(&camera_photo(), Kind::Jpeg),
			Completeness::Complete
		);
	}

	/// The two files this check was added for: a JPEG that opens correctly and then stops.
	#[test]
	fn a_half_written_photo_is_caught() {
		let truncated = [marker(0xD8), tables_and_sos(), scan()].concat();
		assert_eq!(
			sniff(&truncated),
			Kind::Jpeg,
			"the header is intact, as it was"
		);

		let refusal = match completeness(&truncated, Kind::Jpeg) {
			Completeness::Truncated(reason) => reason,
			other => panic!("no EOI: this file is unfinished, not {other:?}"),
		};
		assert!(
			!refusal.is_empty(),
			"the log line has to say which way it was unfinished"
		);
		assert!(
			INCOMPLETE_FILE.contains("incompleto"),
			"the applicant has to be told what is wrong: {INCOMPLETE_FILE}"
		);

		let complete = [truncated, marker(0xD9)].concat();
		assert_eq!(completeness(&complete, Kind::Jpeg), Completeness::Complete);
	}

	/// Motion Photo — the Android camera default — appends an MP4 after the EOI, so the marker
	/// is nowhere near the end of the file. Testing the last two bytes would refuse most of the
	/// photos this form receives, which is why the walk stops at the first structural EOI
	/// instead of looking at where the file happens to end.
	#[test]
	fn an_android_motion_photo_is_not_mistaken_for_a_cut_one() {
		let trailer = [b"\x00\x00\x00\x18ftypmp42".to_vec(), vec![0x00; 64]].concat();
		let motion_photo = [camera_photo(), trailer].concat();

		assert_eq!(
			completeness(&motion_photo, Kind::Jpeg),
			Completeness::Complete
		);
	}

	/// What the MPF segment announces: Apple's HDR gain map, or the other lens' frame, appended
	/// as a whole second JPEG behind the first one's EOI.
	#[test]
	fn a_secondary_jpeg_appended_after_the_eoi_is_still_complete() {
		let with_gain_map = [camera_photo(), camera_photo()].concat();

		assert_eq!(
			completeness(&with_gain_map, Kind::Jpeg),
			Completeness::Complete
		);
	}

	/// A progressive photo has several scans with tables between them. Stopping at the first
	/// marker that follows image data would call every one of them complete halfway through.
	#[test]
	fn a_progressive_photo_is_walked_scan_by_scan() {
		let scans = [
			segment(0xDB, &[0; 65]),
			segment(
				0xC2,
				&[0x08, 0x0C, 0x00, 0x10, 0x00, 0x03, 0x01, 0x22, 0x00],
			),
			segment(0xC4, &[0; 29]),
			segment(0xDA, &[0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]),
			scan(),
			segment(0xC4, &[0; 29]),
			segment(0xDA, &[0x01, 0x01, 0x00, 0x01, 0x3F, 0x00]),
			scan(),
		]
		.concat();

		let complete = [marker(0xD8), scans.clone(), marker(0xD9)].concat();
		assert_eq!(completeness(&complete, Kind::Jpeg), Completeness::Complete);

		// Cut after the second scan's header: the first pass decodes into a blurred image, and
		// the rest never arrived.
		let cut = [marker(0xD8), scans[..scans.len() - scan().len()].to_vec()].concat();
		assert!(matches!(
			completeness(&cut, Kind::Jpeg),
			Completeness::Truncated(_)
		));
	}

	/// The sequences inside image data that look like the end of it and are not: a stuffed `FF`,
	/// the restart markers, and the fill bytes allowed before a real marker.
	#[test]
	fn stuffed_bytes_and_restart_markers_do_not_end_a_scan() {
		let data = vec![
			0xFF, 0x00, // a literal FF, as the scan has to store it
			0xFF, 0x00, 0xD9, // which is how an FF D9 gets *into* the image data
			0xFF, 0xD0, 0xFF, 0xD7, // restart markers
			0x11, 0x22, //
			0xFF, 0xFF, 0xFF, 0xD9, // fill bytes, then the EOI that counts
		];
		let photo = [marker(0xD8), tables_and_sos(), data].concat();
		assert_eq!(completeness(&photo, Kind::Jpeg), Completeness::Complete);

		let cut = [
			marker(0xD8),
			tables_and_sos(),
			vec![0xFF, 0x00, 0xFF, 0x00, 0xD9, 0xFF, 0xD0, 0x11],
		]
		.concat();
		assert!(matches!(
			completeness(&cut, Kind::Jpeg),
			Completeness::Truncated(_)
		));
	}

	/// A colour profile is split across several APP2 segments whose payloads are arbitrary
	/// bytes — `FF D8` and `FF D9` among them.
	#[test]
	fn an_icc_profile_split_across_segments_is_skipped_whole() {
		let chunk = |index: u8| {
			let mut payload = b"ICC_PROFILE\x00".to_vec();
			payload.extend_from_slice(&[index, 0x02]);
			payload.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xD9, 0xFF, 0xDA]);
			segment(0xE2, &payload)
		};

		let photo = [
			marker(0xD8),
			chunk(1),
			chunk(2),
			tables_and_sos(),
			scan(),
			marker(0xD9),
		]
		.concat();

		assert_eq!(completeness(&photo, Kind::Jpeg), Completeness::Complete);
	}

	/// Shapes the walk cannot follow, all of which come out accepted. A structure this code does
	/// not understand is not evidence that bytes are missing, and refusing on a guess costs an
	/// enrolment that was fine — which is the one outcome worse than the grey photo.
	#[test]
	fn a_jpeg_the_walk_cannot_follow_is_let_through() {
		let cases: [(&str, Vec<u8>); 5] = [
			// Junk between two segments, so the cursor lands on something that is not a marker.
			(
				"desynced",
				[
					marker(0xD8),
					segment(0xE1, b"Exif\x00\x00"),
					vec![0x2A, 0x11],
					tables_and_sos(),
					scan(),
				]
				.concat(),
			),
			// A length below the two bytes it counts itself, which would also leave the cursor
			// where it was.
			(
				"length of zero",
				[marker(0xD8), vec![0xFF, 0xE1, 0x00, 0x00], scan()].concat(),
			),
			(
				"length of one",
				[marker(0xD8), vec![0xFF, 0xE1, 0x00, 0x01], scan()].concat(),
			),
			// Arithmetic coding does not stuff `FF 00`, so `FF 4A` is legal image data there and
			// the scan cannot be walked at all.
			(
				"arithmetic-coded",
				[
					marker(0xD8),
					segment(
						0xC9,
						&[0x08, 0x0C, 0x00, 0x10, 0x00, 0x03, 0x01, 0x22, 0x00],
					),
					segment(0xDA, &[0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]),
					scan(),
				]
				.concat(),
			),
			// Two files concatenated: walking into the second would report its truncation as the
			// first one's.
			(
				"second SOI",
				[
					marker(0xD8),
					segment(0xE1, b"Exif\x00\x00"),
					marker(0xD8),
					tables_and_sos(),
					scan(),
					marker(0xD9),
				]
				.concat(),
			),
		];

		for (name, bytes) in cases {
			let verdict = completeness(&bytes, Kind::Jpeg);
			assert!(
				matches!(verdict, Completeness::Unparsable(_)),
				"the {name} file was judged, and it should not have been: {verdict:?}"
			);
		}
	}

	/// Every possible cut point of one photo, in a single loop: nothing before the EOI may come
	/// out complete, and everything from the EOI on must. It is also what proves the walk always
	/// terminates and never panics, wherever the bytes happen to stop.
	#[test]
	fn truncating_a_photo_anywhere_never_reports_complete() {
		let photo = camera_photo();
		let trailer = [b"\x00\x00\x00\x18ftypmp42".to_vec(), vec![0x00; 32]].concat();
		let motion_photo = [photo.clone(), trailer].concat();

		for cut in 0..photo.len() {
			assert_ne!(
				completeness(&photo[..cut], Kind::Jpeg),
				Completeness::Complete,
				"a photo cut at {cut} of {} came out whole",
				photo.len()
			);
		}
		// Past the EOI the rest is a passenger: a Motion Photo whose MP4 never finished
		// uploading is still a photograph that opens.
		for cut in photo.len()..=motion_photo.len() {
			assert_eq!(
				completeness(&motion_photo[..cut], Kind::Jpeg),
				Completeness::Complete,
				"a photo carrying {} bytes of its trailer came out short",
				cut - photo.len()
			);
		}
	}

	#[test]
	fn a_png_is_walked_to_its_iend() {
		let complete = png_file(&[
			png_chunk(b"IHDR", &[0; 13]),
			png_chunk(b"IDAT", b"compressed image data"),
			png_chunk(b"IEND", b""),
		]);
		assert_eq!(completeness(&complete, Kind::Png), Completeness::Complete);

		let mut cut = png_file(&[png_chunk(b"IHDR", &[0; 13])]);
		cut.extend_from_slice(&40_000u32.to_be_bytes());
		cut.extend_from_slice(b"IDAT");
		cut.extend_from_slice(b"the rest of this chunk never arrived");
		assert!(matches!(
			completeness(&cut, Kind::Png),
			Completeness::Truncated(_)
		));
	}

	/// The PNG side of the thumbnail trap: a text chunk whose *data* spells the name of the
	/// chunk the file has to end with. Compressed image data can spell it by chance too, which
	/// on a photo-sized file is not a remote possibility.
	#[test]
	fn a_chunk_that_merely_spells_iend_does_not_pass_for_it() {
		let mut cut = png_file(&[
			png_chunk(b"IHDR", &[0; 13]),
			png_chunk(b"iTXt", b"Comment\x00IEND, and then the file stops"),
		]);
		assert!(
			contains(&cut, b"IEND"),
			"this is not the file that was getting through"
		);

		cut.extend_from_slice(&40_000u32.to_be_bytes());
		cut.extend_from_slice(b"IDAT");
		cut.extend_from_slice(b"cut");
		assert!(matches!(
			completeness(&cut, Kind::Png),
			Completeness::Truncated(_)
		));
	}

	#[test]
	fn an_animated_png_and_its_odd_chunks_are_walked_too() {
		let apng = png_file(&[
			png_chunk(b"IHDR", &[0; 13]),
			png_chunk(b"acTL", &[0; 8]),
			png_chunk(b"fcTL", &[0; 26]),
			// A zero-length chunk is legal, and must not stall the walk.
			png_chunk(b"IDAT", b""),
			png_chunk(b"fdAT", &[0; 4]),
			png_chunk(b"IEND", b""),
		]);
		assert_eq!(completeness(&apng, Kind::Png), Completeness::Complete);

		// Bytes after IEND: the walk stops there and never sees them.
		let padded = [apng, vec![0x00; 16]].concat();
		assert_eq!(completeness(&padded, Kind::Png), Completeness::Complete);
	}

	/// A length or a name outside what the spec allows says the walk has lost the chain, not
	/// that bytes are missing. `FF FF FF FF` would also run the cursor past what an address can
	/// hold, which in a debug build is a panic rather than a wrong answer.
	#[test]
	fn a_png_the_walk_cannot_follow_is_let_through() {
		let mut impossible_length = png_file(&[png_chunk(b"IHDR", &[0; 13])]);
		impossible_length.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
		impossible_length.extend_from_slice(b"IDAT");
		assert!(matches!(
			completeness(&impossible_length, Kind::Png),
			Completeness::Unparsable(_)
		));

		let mut unnamed = png_file(&[png_chunk(b"IHDR", &[0; 13])]);
		unnamed.extend_from_slice(&4u32.to_be_bytes());
		unnamed.extend_from_slice(&[0x00, 0x11, 0x22, 0x33]);
		unnamed.extend_from_slice(b"data");
		assert!(matches!(
			completeness(&unnamed, Kind::Png),
			Completeness::Unparsable(_)
		));
	}

	/// A linearized PDF — "fast web view", which is what a scanner or Acrobat produces — puts a
	/// cross-reference and a `%%EOF` for the first page at the *front* of the file. So asking
	/// whether the file contains one at all accepts a download that stopped in the middle.
	#[test]
	fn a_linearized_pdf_cut_in_half_is_caught_despite_its_first_page_eof() {
		let mut cut =
			b"%PDF-1.6\n<< /Linearized 1 /L 82304 /O 12 >>\nxref\n0 12\ntrailer\n<< /Size 12 >>\n\
			  startxref\n0\n%%EOF\n"
				.to_vec();
		cut.extend_from_slice(&vec![b'x'; 8192]);

		assert!(
			contains(&cut, b"%%EOF"),
			"this is not the file that was getting through"
		);
		assert_eq!(sniff(&cut), Kind::Pdf);
		assert!(matches!(
			completeness(&cut, Kind::Pdf),
			Completeness::Truncated(_)
		));
	}

	#[test]
	fn a_pdf_that_ends_where_it_should_is_complete() {
		// Two revisions, as an incremental update or a signature leaves behind: the marker that
		// counts is the last one.
		let updated = b"%PDF-1.7\n...\nstartxref\n0\n%%EOF\n...\nstartxref\n120\n%%EOF\n";
		assert_eq!(completeness(updated, Kind::Pdf), Completeness::Complete);

		// Padding added after the marker by whatever carried the file. Truncation only ever
		// removes bytes, so this is trimmed before the tail is measured.
		let padded = [b"%PDF-1.7\n...\n%%EOF\n".to_vec(), vec![0x00; 3000]].concat();
		assert_eq!(completeness(&padded, Kind::Pdf), Completeness::Complete);
	}

	/// The formats with no trailer worth following must come out complete. Guessing at them
	/// would refuse a submission that was fine.
	#[test]
	fn formats_without_an_end_marker_are_left_alone() {
		for kind in ALL {
			if matches!(kind, Kind::Jpeg | Kind::Png | Kind::Pdf) {
				continue;
			}
			assert_eq!(
				completeness(b"BM\x00\x00 whatever", kind),
				Completeness::Complete,
				"{} has no end marker to judge it by",
				kind.label()
			);
		}
		assert_eq!(completeness(b"", Kind::Bmp), Completeness::Complete);
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
