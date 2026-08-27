/*
 * Decides how the certificate PDF is offered, per device, at load time.
 *
 * Why this is not decided on the server, where the rest of this project's rules live: the
 * page reaching a visitor may come from a cache. `Vary: User-Agent` is the correct
 * instruction and several caches — Cloudflare among them — ignore it, so one visitor's
 * variant gets served to everybody. Deciding here keeps the document identical for every
 * browser, which means no cache can hand out the wrong one.
 *
 * What the three cases are, all established by testing rather than by specification:
 *
 * - Most browsers, including desktop Firefox: the `download` attribute on the link is
 *   enough, and the markup already carries it. Nothing to do.
 * - The Firefox family on Android: the attribute is ignored for a PDF, which opens in a
 *   viewer with no way to save it. `Content-Disposition: attachment` is obeyed, so the link
 *   is pointed at the mount that sends it.
 * - Firefox and Focus on iOS: the attribute is ignored *and* an attachment fails outright
 *   with "Frame load interrupted" — WebKit cancels the navigation and the app has nowhere to
 *   put the file. Nothing the server sends helps, so the page says to use another browser.
 *
 * Progressive enhancement in the honest direction: this script only ever *adds* a note or
 * retargets the link. Blocked, the button behaves exactly as rendered — inline URL plus
 * `download`, which is what works for everybody outside the Firefox family.
 */

(() => {
	const link = document.getElementById("certificate-download");
	const note = document.getElementById("certificate-note");
	if (!link || !note) return;

	const ua = navigator.userAgent.toLowerCase();

	const ios = /iphone|ipad|ipod/.test(ua);
	const mobile = ios || /mobile|android/.test(ua);

	/*
	 * `(KHTML, like Gecko)` sits in every WebKit and Chrome string, so `Gecko/` — with a
	 * slash — is what identifies real Gecko. `FxiOS` marks Firefox for iOS, which carries no
	 * `Firefox` token at all, and `Focus`/`Klar` the privacy shells, whose string on Android
	 * looks like Chrome.
	 */
	const firefoxFamily =
		(ua.includes("gecko/") && ua.includes("firefox/")) ||
		ua.includes("fxios/") ||
		ua.includes("focus/") ||
		ua.includes("klar/");

	if (!mobile || !firefoxFamily) return;

	/*
	 * Being wrong towards the note is the expensive direction — it tells somebody to change
	 * browser for nothing — which is why iOS is recognised by its platform token and not by
	 * the product alone.
	 */
	if (ios) {
		note.hidden = false;
		return;
	}

	link.href = link.dataset.attachmentUrl;
})();
