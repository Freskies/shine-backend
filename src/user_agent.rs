//! The one place that inspects `User-Agent`.
//!
//! Sniffing is a last resort, kept here so it stays a single testable function instead of a
//! condition sprinkled through handlers or templates.

/// True when the request comes from Firefox, Firefox Focus or Klar on a phone or tablet.
///
/// Asked so the certificate download can carry a note: on mobile Firefox the `download`
/// attribute is ignored for a PDF and the file opens in the built-in viewer instead of
/// being saved. There is no honest alternative to the string — Firefox implements no
/// `navigator.userAgentData`, and no feature test describes "opens a viewer".
///
/// Three products, three shapes, because only one of them is Gecko:
///
/// - Firefox for Android: `... (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0`
/// - Firefox for iOS: WebKit, marked by an `FxiOS/` token — no `Firefox/`
/// - Focus / Klar: a WebView shell whose string is Chrome-like, marked only by `Focus/` or
///   `Klar/`
///
/// `Gecko/` is required alongside `Firefox/` because `(KHTML, like Gecko)` appears in every
/// WebKit and Chrome string, so `Gecko` alone matches nearly everything. The mobile check is
/// what keeps desktop Firefox out; a false positive only adds a line of advice that does not
/// apply, so the match is deliberately loose.
pub fn is_mobile_firefox(user_agent: &str) -> bool {
	let ua = user_agent.to_ascii_lowercase();

	let mobile = ua.contains("mobile")
		|| ua.contains("android")
		|| ua.contains("iphone")
		|| ua.contains("ipad");

	mobile
		&& ((ua.contains("gecko/") && ua.contains("firefox/"))
			|| ua.contains("fxios/")
			|| ua.contains("focus/")
			|| ua.contains("klar/"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn matches_mobile_firefox_family() {
		for ua in [
			// Firefox for Android.
			"Mozilla/5.0 (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0",
			// Firefox for Android, tablet: no `Mobile` token, but `Android` is there.
			"Mozilla/5.0 (Android 14; Tablet; rv:143.0) Gecko/143.0 Firefox/143.0",
			// Firefox for iOS.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) FxiOS/127.0 Mobile/15E148 Safari/605.1.15",
			// Focus for Android: a WebView, so the string says Chrome.
			"Mozilla/5.0 (Linux; Android 11; CPH2269 Build/RP1A.200720.011) \
			 AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Focus/8.0.8 \
			 Chrome/123.0.6312.118 Mobile Safari/537.36",
			// Klar, the German-market build of Focus.
			"Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) \
			 Version/4.0 Klar/8.0.8 Chrome/120.0.6099.45 Mobile Safari/537.36",
		] {
			assert!(is_mobile_firefox(ua), "should have matched: {ua}");
		}
	}

	#[test]
	fn ignores_everything_else() {
		for ua in [
			// Desktop Firefox: the viewer saves the file just fine.
			"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:143.0) Gecko/20100101 Firefox/143.0",
			// Chrome for Android: `like Gecko`, no `Gecko/`.
			"Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 \
			 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36",
			// Safari on iOS.
			"Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 \
			 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
			// A productivity app, not a browser: `focused/` is not `focus/`.
			"Be Focused/104 CFNetwork/1494.0.7 Darwin/23.4.0",
			"",
		] {
			assert!(!is_mobile_firefox(ua), "should not have matched: {ua}");
		}
	}

	#[test]
	fn matching_ignores_case() {
		assert!(is_mobile_firefox(
			"MOZILLA/5.0 (ANDROID 14; MOBILE; RV:143.0) GECKO/143.0 FIREFOX/143.0"
		));
	}
}
