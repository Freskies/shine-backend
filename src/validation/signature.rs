//! Whether a signature canvas was actually signed, read from the bytes it serialized to.
//!
//! Server-side only, like [`super::fiscal_code`] and [`super::file_type`], and for the same
//! reason: no pattern can express it. A canvas that was never touched serializes to a PNG that
//! is perfectly well formed — `data:image/png;base64,` prefix, correct header, decodable — and
//! it used to satisfy every check on the way in. Two membership documents arrived with the
//! autonomy signature drawn and the mandatory one an empty line, because nothing between the
//! pad and the PDF ever asked whether there was any ink in it.
//!
//! There were two ways to get there, and neither leaves a trace in the data:
//!
//! - The pad reported "drawn" from a flag set on the first pointer move, which a swipe across
//!   the box sets as readily as a signature — `.signature-box` carries `touch-action: none`, so
//!   a finger dragged over it draws instead of scrolling the page.
//! - The flag outlives the strokes. Assigning `width` to a canvas wipes it, and the pixels are
//!   the only place the strokes existed: a backing store the browser had already dropped — iOS
//!   does that under memory pressure, and this page also holds the preview of a photo of up to
//!   12 MB — came back empty, and the flag still said yes.
//!
//! So the ink is counted here, on the bytes that arrived, and `enrollment.js` counts it the same
//! way on the canvas before serializing it. The threshold travels to the page inside the rules
//! JSON (see `min_ink_ratio` in [`super::ClientRule`]) so that it exists once.
//!
//! The pad is never given a background, which is what makes the count simple: transparency *is*
//! the empty box, and ink is anything the strokes painted over it.

use base64::Engine;
use png::{ColorType, Transformations};
use std::io::Cursor;

/// The share of the box a signature has to cover.
///
/// A stroke is 2 CSS pixels wide, so a signature a few hundred pixels long covers around one
/// percent of a pad whatever the device pixel ratio, while an accidental drag or a resting
/// fingertip covers well under a tenth of that. The gap between the two is wide, and the
/// threshold sits low in it on purpose: an initial or a cross is a valid signature, and refusing
/// one costs an enrolment that was fine.
pub const MIN_INK_RATIO: f32 = 0.003;

/// How opaque a pixel has to be to count as ink, out of 255.
///
/// The strokes are antialiased, so their edges arrive as partial alpha. Low enough to keep a
/// thin stroke, high enough that nothing rounds up from a fully transparent box.
const INK_ALPHA: u8 = 32;

/// Refused before a row is walked. A canvas is a few hundred pixels each way; anything of this
/// size is a hand-made request, and walking it would cost real time.
const MAX_PIXELS: u64 = 32 * 1024 * 1024;

/// What was in the box.
pub enum Verdict {
	/// Enough ink to be a signature.
	Signed,
	/// Decoded, and all but empty.
	Blank,
	/// Not a PNG this server can read. Carries the reason, for the log — the applicant is told
	/// to sign again, which is the only thing they can do about it.
	Unreadable(&'static str),
	/// Decoded, but with no alpha channel to tell ink from background. Accepted, and carries the
	/// reason for the log line that says so. See [`check`].
	Unmeasurable(&'static str),
}

/// Shown when the box came back empty. Also the sentence the page shows, which is why it is a
/// constant rather than written at the one call site.
pub const BLANK: &str = "Il riquadro della firma è vuoto: tracciala prima di inviare.";

/// Shown when the PNG would not decode. Deliberately does not speculate about the cause: the
/// applicant has one move either way, and the reason is in the log.
pub const UNREADABLE: &str = "Questa firma non è arrivata leggibile: tracciala di nuovo.";

/// Counts the ink in a canvas data URL.
///
/// Both escapes — [`Verdict::Unmeasurable`] and the size guard — follow the rule
/// [`super::file_type`] is built on: a refusal that is wrong costs an enrolment that was fine,
/// which is worse than the thing it would have caught. So a PNG whose colour type carries no
/// alpha is accepted rather than guessed at, and it is the caller's job to log that it went
/// through unmeasured.
pub fn check(data_url: &str) -> Verdict {
	// The prefix is checked by the caller; what is needed here is the payload after the comma.
	let Some(encoded) = data_url.split(',').nth(1) else {
		return Verdict::Unreadable("no comma in the data URL");
	};
	let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim()) else {
		return Verdict::Unreadable("the base64 payload would not decode");
	};

	// A cursor rather than the slice itself: the decoder seeks, to skip the chunks it is not
	// reading.
	let mut decoder = png::Decoder::new(Cursor::new(&bytes));
	// Palette and low bit depths become plain 8-bit samples, and a `tRNS` chunk becomes the
	// alpha channel it stands for, so the walk below only ever has to know how many channels
	// there are and where the alpha sits. Deliberately *not* `Transformations::ALPHA`, which
	// would bolt an all-opaque channel onto an image that has none and turn every pixel of it
	// into ink.
	decoder.set_transformations(Transformations::normalize_to_color8());

	let mut reader = match decoder.read_info() {
		Ok(reader) => reader,
		Err(_) => return Verdict::Unreadable("the PNG header would not decode"),
	};

	let info = reader.info();
	let (width, height) = (u64::from(info.width), u64::from(info.height));
	let pixels = width * height;
	if pixels == 0 {
		return Verdict::Unreadable("the PNG declares no pixels");
	}
	if pixels > MAX_PIXELS {
		return Verdict::Unreadable("the PNG declares more pixels than a canvas can hold");
	}

	let (color, _) = reader.output_color_type();
	let channels = color.samples();
	// Where the alpha sits, which is always last when there is one.
	let alpha = match color {
		ColorType::Rgba | ColorType::GrayscaleAlpha => channels - 1,
		// An opaque image: every pixel is as present as every other, and the strokes cannot be
		// told from what they were drawn on. No browser produces one from `toDataURL` on a pad
		// that was never filled, so this is a format nobody here wrote — not evidence of ink,
		// and not evidence of its absence either.
		_ => return Verdict::Unmeasurable("the PNG has no alpha channel"),
	};

	let mut ink: u64 = 0;
	loop {
		match reader.next_row() {
			// Interlaced PNGs hand back one pass at a time, so a row is not a line of the image
			// — but every pixel still arrives exactly once, which is all the count needs.
			Ok(Some(row)) => {
				ink += row
					.data()
					.chunks_exact(channels)
					.filter(|pixel| pixel[alpha] >= INK_ALPHA)
					.count() as u64;
			}
			Ok(None) => break,
			Err(_) => return Verdict::Unreadable("the PNG image data would not decode"),
		}
	}

	if (ink as f32) < MIN_INK_RATIO * pixels as f32 {
		return Verdict::Blank;
	}
	Verdict::Signed
}

/// The two canvases as the browser serializes them, for the tests here and for the ones in
/// [`super`] that judge a whole form: what a signature field holds is a PNG, so a fixture that
/// is anything else tests a path no applicant can reach.
#[cfg(test)]
pub mod fixture {
	use super::*;

	/// A pad at the size the page renders one, in device pixels on a phone.
	pub const WIDTH: u32 = 900;
	pub const HEIGHT: u32 = 420;

	/// Serializes an RGBA buffer the way `canvas.toDataURL("image/png")` does.
	pub fn data_url(width: u32, height: u32, pixels: &[u8]) -> String {
		let mut png_bytes = Vec::new();
		{
			let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
			encoder.set_color(ColorType::Rgba);
			encoder.set_depth(png::BitDepth::Eight);
			let mut writer = encoder.write_header().unwrap();
			writer.write_image_data(pixels).unwrap();
		}
		format!(
			"data:image/png;base64,{}",
			base64::engine::general_purpose::STANDARD.encode(&png_bytes)
		)
	}

	/// An untouched pad: never filled, so every pixel is transparent.
	pub fn empty_pad() -> Vec<u8> {
		vec![0; (WIDTH * HEIGHT * 4) as usize]
	}

	/// Paints an opaque rectangle, the way a stroke of `line_width` running `length` pixels
	/// across the pad would.
	pub fn stroke(pixels: &mut [u8], length: u32, line_width: u32, top: u32) {
		for y in top..top + line_width {
			for x in 0..length {
				let offset = ((y * WIDTH + x) * 4) as usize;
				pixels[offset + 3] = 255;
			}
		}
	}

	/// A pad nobody touched.
	pub fn untouched() -> String {
		data_url(WIDTH, HEIGHT, &empty_pad())
	}

	/// A small but real signature: three strokes of a couple of hundred pixels.
	pub fn signed() -> String {
		let mut pixels = empty_pad();
		for row in 0..3 {
			stroke(&mut pixels, 220, 6, 6 + row * 20);
		}
		data_url(WIDTH, HEIGHT, &pixels)
	}
}

#[cfg(test)]
mod tests {
	use super::fixture::*;
	use super::*;

	#[test]
	fn an_untouched_pad_is_blank() {
		assert!(matches!(check(&untouched()), Verdict::Blank));
	}

	/// The shape of what reached the association: a box with no signature in it, arriving as a
	/// PNG that decodes perfectly. A finger dragged across the pad on the way to scrolling the
	/// page leaves exactly this, and the old flag counted it as signed.
	#[test]
	fn an_accidental_drag_is_blank() {
		let mut pixels = empty_pad();
		stroke(&mut pixels, 40, 6, 0);
		assert!(matches!(
			check(&data_url(WIDTH, HEIGHT, &pixels)),
			Verdict::Blank
		));
	}

	#[test]
	fn a_signature_is_signed() {
		assert!(matches!(check(&signed()), Verdict::Signed));
	}

	/// A cross or an initial covers a fraction of the box, and it is a signature. The threshold
	/// has to stay below it.
	#[test]
	fn a_short_mark_is_still_a_signature() {
		let mut pixels = empty_pad();
		stroke(&mut pixels, 200, 12, 0);
		assert!(matches!(
			check(&data_url(WIDTH, HEIGHT, &pixels)),
			Verdict::Signed
		));
	}

	#[test]
	fn something_that_is_not_a_png_is_unreadable() {
		assert!(matches!(
			check("data:image/png;base64,AAAA"),
			Verdict::Unreadable(_)
		));
		assert!(matches!(
			check("data:image/png;base64,not base64 at all!"),
			Verdict::Unreadable(_)
		));
		assert!(matches!(check("data:image/png"), Verdict::Unreadable(_)));
	}

	/// Nothing in this codebase produces one, but a hand-made request can: it is accepted,
	/// because an opaque image carries no way of telling a stroke from its background, and the
	/// caller logs that it went through unmeasured.
	#[test]
	fn an_opaque_png_is_accepted_unmeasured() {
		let mut png_bytes = Vec::new();
		{
			let mut encoder = png::Encoder::new(&mut png_bytes, 4, 4);
			encoder.set_color(ColorType::Rgb);
			encoder.set_depth(png::BitDepth::Eight);
			let mut writer = encoder.write_header().unwrap();
			writer.write_image_data(&[0; 4 * 4 * 3]).unwrap();
		}
		let url = format!(
			"data:image/png;base64,{}",
			base64::engine::general_purpose::STANDARD.encode(&png_bytes)
		);
		assert!(matches!(check(&url), Verdict::Unmeasurable(_)));
	}
}
