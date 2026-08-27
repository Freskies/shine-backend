//! The one place that inspects `User-Agent`.
//!
//! Sniffing is a last resort, kept here so it stays a single testable function instead of a
//! condition sprinkled through handlers or templates.

/// How a browser can be made to *save* a PDF, rather than look at it.
///
/// Established by testing, not by specification, because the three behaviours differ in ways
/// no feature test can ask about: Firefox implements no `navigator.userAgentData`, and
/// nothing in the DOM describes what a built-in viewer offers.
#[derive(Debug, PartialEq, Eq)]
pub enum PdfDownload {
	/// The `download` attribute on the link is enough. Safari and Chrome on iOS, and every
	/// desktop browser tested, including desktop Firefox.
	Attribute,
	/// The attribute is ignored, so the response has to carry
	/// `Content-Disposition: attachment`. The Firefox family on Android, which has a real
	/// download manager behind it.
	Header,
	/// Neither works. Firefox and Focus on iOS ignore the attribute *and* fail on an
	/// attachment with "Frame load interrupted": WebKit cancels the navigation and the app
	/// has nowhere to put the file. Served inline the PDF at least opens in the viewer, which
	/// on this browser offers no way to save it — so the page has to say to use another one.
	Unsupported,
}

/// Reads the `User-Agent` and decides which of the three cases applies.
///
/// Three products, three shapes of string, because only one of them is Gecko:
///
/// - Firefox for Android: `... (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0`
/// - Firefox for iOS: WebKit, marked by an `FxiOS/` token — no `Firefox/`
/// - Focus / Klar: a shell around the platform engine, marked only by `Focus/` or `Klar/`;
///   on Android its string is Chrome-like
///
/// `Gecko/` is required alongside `Firefox/` because `(KHTML, like Gecko)` appears in every
/// WebKit and Chrome string, so `Gecko` alone matches nearly everything. Desktop is excluded
/// by requiring a mobile token: desktop Firefox saves the file with no help. Being wrong in
/// the `Unsupported` direction is the expensive one — it tells a visitor to change browser for
/// nothing — which is why iOS is matched on the platform token and not on the product alone.
pub fn pdf_download(user_agent: &str) -> PdfDownload {
	let ua = user_agent.to_ascii_lowercase();

	let ios = ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod");
	let mobile = ios || ua.contains("mobile") || ua.contains("android");

	let firefox_family = (ua.contains("gecko/") && ua.contains("firefox/"))
		|| ua.contains("fxios/")
		|| ua.contains("focus/")
		|| ua.contains("klar/");

	match (mobile && firefox_family, ios) {
		(true, true) => PdfDownload::Unsupported,
		(true, false) => PdfDownload::Header,
		(false, _) => PdfDownload::Attribute,
	}
}

#[cfg(test)]
mod tests {
	use super::PdfDownload::*;
	use super::*;

	#[test]
	fn ios_firefox_family_cannot_download_at_all() {
		for ua in [
			// Firefox for iOS.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) FxiOS/127.0 Mobile/15E148 Safari/605.1.15",
			// Focus for iOS.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) Focus/128.0 Mobile/15E148 Safari/605.1.15",
			// iPad, which reports no `Mobile` token in some configurations.
			"Mozilla/5.0 (iPad; CPU OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) FxiOS/127.0 Safari/605.1.15",
		] {
			assert_eq!(pdf_download(ua), Unsupported, "for: {ua}");
		}
	}

	#[test]
	fn android_firefox_family_needs_the_header() {
		for ua in [
			// Firefox for Android.
			"Mozilla/5.0 (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0",
			// Firefox for Android, tablet: no `Mobile` token, but `Android` is there.
			"Mozilla/5.0 (Android 14; Tablet; rv:143.0) Gecko/143.0 Firefox/143.0",
			// Focus for Android: a WebView, so the string says Chrome.
			"Mozilla/5.0 (Linux; Android 11; CPH2269 Build/RP1A.200720.011) \
			 AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Focus/8.0.8 \
			 Chrome/123.0.6312.118 Mobile Safari/537.36",
			// Klar, the German-market build of Focus.
			"Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) \
			 Version/4.0 Klar/8.0.8 Chrome/120.0.6099.45 Mobile Safari/537.36",
		] {
			assert_eq!(pdf_download(ua), Header, "for: {ua}");
		}
	}

	#[test]
	fn everything_else_honours_the_attribute() {
		for ua in [
			// Desktop Firefox: saves the file with no help from us.
			"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:143.0) Gecko/20100101 Firefox/143.0",
			// Safari on iOS: tested working.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
			// Chrome on iOS: tested working.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) CriOS/126.0.0.0 Mobile/15E148 Safari/604.1",
			// Chrome for Android: `like Gecko`, no `Gecko/`.
			"Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 \
			 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36",
			// A productivity app, not a browser: `focused/` is not `focus/`.
			"Be Focused/104 CFNetwork/1494.0.7 Darwin/23.4.0",
			"",
		] {
			assert_eq!(pdf_download(ua), Attribute, "for: {ua}");
		}
	}

	#[test]
	fn matching_ignores_case() {
		assert_eq!(
			pdf_download("MOZILLA/5.0 (ANDROID 14; MOBILE; RV:143.0) GECKO/143.0 FIREFOX/143.0"),
			Header
		);
	}
}
