/*
 * Enrolment wizard: phase navigation, signature pads, emergency-contact removal and the
 * pre-submit checks.
 *
 * Loaded as a module (see enrollment.html), so the declarations below are scoped to this file
 * and strict mode is already on — hence no "use strict". Anything the outside has to reach
 * must be put on `window` explicitly, as `__phase3` at the bottom is.
 *
 * The two phases are sections of a single form, toggled by `data-phase` on the wrapper.
 * They are never fetched or removed because a file input cannot be repopulated from
 * script: dropping phase 1 from the DOM would lose the chosen photo.
 */

const body = document.querySelector("body");
const wizard = document.getElementById("enrollment-wizard");
const form = document.getElementById("enrollment-form");

/*
 * HTMX halts a non-GET request whose form fails constraint validation, but by default it
 * does so without telling anyone: this makes it show the browser's own message on the
 * first offending field instead of appearing to ignore the click.
 */
htmx.config.reportValidityOfForms = true;

/* FIELD RULES */

/*
 * Every format rule arrives as JSON generated from `RULES` in src/validation/mod.rs — the
 * same table the submission is judged against. Applying them from here, rather than writing
 * `pattern="…"` into the markup, is what keeps one description of each rule: for the browser
 * and the server to disagree, the server would have to send a rule it does not itself
 * enforce.
 *
 * `required` is deliberately not in the JSON. It stays in the markup and in
 * syncConditionalSections(), which is the only party that knows whether the minor and the
 * autonomy sections are currently on screen.
 */
const RULES = new Map(
	JSON.parse(document.getElementById("validation-rules").textContent)
		.map(rule => [rule.name, rule]),
);

/* FIELD-LEVEL ERRORS */

/*
 * Every message is shown above the field it is about — the browser's own bubble vanishes on
 * the next click, and a list at the foot of a form this long makes the reader match names to
 * inputs by hand. Client and server print through the same two functions, so a rejected CAP
 * reads the same whether the browser caught it on blur or the server caught it on submit.
 *
 * Three fields keep the paragraph they already have in the markup: the certificate sits in
 * the file picker of phase 1, outside any `.membership-form__field`, and the two canvases are
 * not form controls, so `canSubmit` writes their messages itself. Everything else gets a
 * paragraph created on first use and then reused, so a second failed attempt rewrites the
 * text in place instead of moving the field down the page again.
 */
const MARKUP_ERRORS = new Map([
	["certificate", "certificate-error"],
	["signature", "signature-error"],
	["autonomy_signature", "autonomy-signature-error"],
]);

let errorSlotCount = 0;

function errorSlot (input, create) {
	const inMarkup = MARKUP_ERRORS.get(input.name);
	if (inMarkup) return document.getElementById(inMarkup);

	const field = input.closest(".membership-form__field");
	if (!field) return null;

	const existing = field.querySelector(":scope > .membership-form__error");
	if (existing || !create) return existing;

	const slot = document.createElement("p");
	slot.className = "membership-form__error";
	/* Its own id, not one derived from the input's: the four contact inputs are appended
	   again for every row, so those ids are not unique and cannot anchor anything. */
	slot.id = `field-error-${++errorSlotCount}`;
	field.prepend(slot);
	return slot;
}

function showFieldError (input, message) {
	if (!message) return clearFieldError(input);

	input.setAttribute("aria-invalid", "true");
	const slot = errorSlot(input, true);
	if (!slot) return;

	slot.textContent = message;
	slot.hidden = false;
	/* Read out when the field takes focus, which is where a rejection sends it. */
	input.setAttribute("aria-describedby", slot.id);
}

function clearFieldError (input) {
	input.removeAttribute("aria-invalid");
	const slot = errorSlot(input, false);
	if (slot) slot.hidden = true;
}

/* A fresh attempt must not leave the previous one's messages standing next to fields that
   have since been corrected. */
function clearAllFieldErrors () {
	form.querySelectorAll("[aria-invalid]").forEach(clearFieldError);
}

/*
 * The browser's own wording for a failed `pattern` is "Match the requested format", which
 * tells nobody anything. These are the sentences the server would have answered with.
 */
function messageFor (input, rule) {
	const v = input.validity;
	if (v.valueMissing) return "Questo campo è obbligatorio.";
	if (v.tooShort) return `Servono almeno ${rule.minLength} caratteri.`;
	if (v.tooLong) return `Non può superare i ${rule.maxLength} caratteri.`;
	if (v.patternMismatch || v.typeMismatch) return rule.hint;

	/*
	 * A date. Its window is the one rule with no attribute to become — the field is a text
	 * input, so there is no `min` for the browser to enforce — and the two remaining checks are
	 * made here, against the bounds the server resolved and with the sentences it wrote. The
	 * pattern above has already established the shape, so the parse only asks whether those
	 * digits are a day that exists: 31/02 satisfies the shape and nothing else.
	 */
	if (rule.min && input.value) {
		const iso = isoFromTyped(input.value);
		if (!iso) return "Questo giorno non esiste sul calendario.";
		/* ISO dates compare as strings, so no calendar arithmetic is needed. */
		if (iso < rule.min) return rule.minMessage || rule.hint;
		if (iso > rule.max) return rule.maxMessage || rule.hint;
	}

	return "";
}

/*
 * Re-runs the native checks and replaces whatever message they produced.
 *
 * setCustomValidity() is sticky — a non-empty message keeps the field invalid until it is
 * cleared — so it is emptied first, the validity flags are read while they mean something,
 * and only then is the replacement set.
 */
function refreshValidity (input, rule) {
	input.setCustomValidity("");
	const message = messageFor(input, rule);
	input.setCustomValidity(message);
	return message;
}

function applyRules (root) {
	for (const input of root.querySelectorAll("input[name]")) {
		const rule = RULES.get(input.name);
		/* Ruled once: this runs again for every contact row HTMX appends, and the listeners
		   below must not stack up on the inputs that were already there. */
		if (!rule || input.dataset.ruled) continue;
		/* The two signatures. A hidden input is exempt from constraint validation, so there is
		   nothing for the browser to enforce and no `blur` to enforce it on: their rule is the
		   share of the pad the ink has to cover, and `initPad` is what reads it. */
		if (input.type === "hidden") continue;
		input.dataset.ruled = "1";

		if (rule.pattern) input.pattern = rule.pattern;
		if (rule.minLength) input.minLength = rule.minLength;
		if (rule.maxLength) input.maxLength = rule.maxLength;
		input.title = rule.hint;
		/* `rule.min` and `rule.max` are deliberately not assigned: a date window has no
		   attribute on a text input. `messageFor` compares against them, and `initDateField`
		   hands them to the picker. */

		/* Province abbreviations and fiscal codes are matched against uppercase-only
		   patterns, which is also what the server compares after normalising, so the field
		   is kept uppercase as it is typed rather than corrected afterwards. */
		if (rule.uppercase && input.type === "text") {
			input.addEventListener("input", () => {
				const caret = input.selectionStart;
				input.value = input.value.toUpperCase();
				input.setSelectionRange(caret, caret);
			});
		}

		input.addEventListener("input", () => {
			input.setCustomValidity("");
			clearFieldError(input);
		});

		/* Only on blur, and only once there is something to judge: flagging a field the
		   moment it is tabbed past would light up the whole form on the way down. An empty
		   required field is caught by the `invalid` handler below. */
		input.addEventListener("blur", () => {
			if (!input.value) return;
			showFieldError(input, refreshValidity(input, rule));
		});

		/* Fired by the browser's own pass over the form at submit — `reportValidityOfForms`
		   above is what makes HTMX run it — so an empty required field gets the same
		   paragraph as everything else, not just the bubble that disappears on the next click. */
		input.addEventListener("invalid", () => showFieldError(input, refreshValidity(input, rule)));
	}
}

applyRules(document);

/* DATE FIELDS */

/*
 * Each birth date is a text input the eight digits are typed into — the slashes are placed here,
 * as the field fills — with a `type="date"` lying invisible over the calendar icon at its right
 * whose only job is to open the platform picker.
 *
 * One `type="date"` and nothing else would be less code and was what stood here first. It
 * cannot work: on a phone that control refuses the keyboard outright, so a birthday forty years
 * back has to be travelled to through a spinner or a calendar rather than simply typed. So the
 * text input is the field — it carries the name, the rule, the errors and the posted value — and
 * the picker only writes into it.
 */

/* dd/mm/yyyy → yyyy-mm-dd, or "" when those digits are not a day on the calendar. */
function isoFromTyped (value) {
	const [d, m, y] = value.trim().split("/");
	if (!y) return "";
	const iso = `${y}-${m}-${d}`;
	const date = new Date(`${iso}T00:00:00`);
	if (Number.isNaN(date.getTime())) return "";
	/* `new Date` rolls 31/02 forward to 03/03 rather than refusing it, so the parts are read
	   back off the date it produced and compared with the ones that went in. */
	if (date.getMonth() + 1 !== Number(m) || date.getDate() !== Number(d)) return "";
	return iso;
}

function typedFromIso (iso) {
	const [y, m, d] = iso.split("-");
	return `${d}/${m}/${y}`;
}

const digitsIn = (value) => value.replace(/[^0-9]/g, "");

/* What the field looks like once filled, and what `.date-field__ghost` prints the missing tail of.
   Its slashes sit where `maskDate` puts them, which is what lets the two be sliced at one index. */
const DATE_TEMPLATE = "gg/mm/aaaa";

/* Eight digits in, `gg/mm/aaaa` out: the applicant types numbers and never a separator, and
   anything else — a pasted `10-12-1985`, a stray letter — loses everything but its digits. */
function maskDate (value) {
	const digits = digitsIn(value).slice(0, 8);
	return [digits.slice(0, 2), digits.slice(2, 4), digits.slice(4, 8)]
		.filter(part => part)
		.join("/");
}

/* Where the caret belongs after the mask has run: still after the same digit, which is not the
   same offset once a slash has been inserted in front of it. */
function caretAfterDigits (value, count) {
	if (count === 0) return 0;
	let seen = 0;
	for (let i = 0; i < value.length; i++) {
		if (value[i] !== "/") seen++;
		if (seen === count) return i + 1;
	}
	return value.length;
}

function initDateField (typed) {
	const rule = RULES.get(typed.name);
	const native = typed.parentElement.querySelector(".date-field__native");
	if (!rule || !native) return;

	/* The same window the field is judged against, so the picker cannot offer a day that would
	   then be refused: the eighteen-year rule shapes what it shows rather than only rejecting
	   what came out of it. */
	native.min = rule.min;
	native.max = rule.max;

	/*
	 * On a desktop, Chrome and Firefox open the picker from the calendar button drawn inside the
	 * control and from nowhere else — and `appearance: none`, which is what stops WebKit sizing
	 * this box back across the field, has taken that button away. So the click that lands here
	 * asks for the picker itself. On a phone the tap opens it regardless, and a picker already
	 * open ignores the second request.
	 */
	native.addEventListener("click", () => {
		try {
			native.showPicker();
		} catch {
			/* No `showPicker` (Safari before 16), or the call was not credited to a gesture. The
			   control's own click behaviour is then the only way in, and it has already run. */
		}
	});

	/*
	 * The grey `gg/mm/aaaa`, built here rather than in the template: it exists only because of the
	 * mask below, and the mask is what knows how much of it is left. Two spans, because the first
	 * repeats what has been typed in transparent ink to push the second along — see the CSS.
	 *
	 * `placeholder` is dropped as this replaces it, the two would otherwise print the same word
	 * over itself. It stays in the markup all the same: with this script not running, the field
	 * still says what it wants.
	 */
	const ghost = document.createElement("span");
	ghost.className = "date-field__ghost";
	ghost.setAttribute("aria-hidden", "true");
	const ghostTyped = document.createElement("span");
	ghostTyped.className = "date-field__ghost-typed";
	const ghostRest = document.createElement("span");
	ghost.append(ghostTyped, ghostRest);
	typed.parentElement.appendChild(ghost);
	typed.removeAttribute("placeholder");

	function syncGhost () {
		ghostTyped.textContent = typed.value;
		ghostRest.textContent = DATE_TEMPLATE.slice(typed.value.length);
	}

	syncGhost();

	typed.addEventListener("input", () => {
		const before = digitsIn(typed.value.slice(0, typed.selectionStart)).length;
		typed.value = maskDate(typed.value);
		const caret = caretAfterDigits(typed.value, before);
		typed.setSelectionRange(caret, caret);
		syncGhost();
		/* Keeps the picker opening on the month being typed rather than on today. Empty while
		   the date is incomplete, which is what leaves the picker on its own default. */
		native.value = isoFromTyped(typed.value);
	});

	native.addEventListener("change", () => {
		if (!native.value) return;
		typed.value = typedFromIso(native.value);
		/* A value set from script fires no event, and the `input` listener applyRules() left on
		   the field is what drops the previous message and its sticky custom validity. */
		typed.dispatchEvent(new Event("input", { bubbles: true }));
		/* Then judged at once, as a typed date is on blur: the window is the picker's too, so
		   what this can still catch is a field the applicant then leaves half corrected. */
		showFieldError(typed, refreshValidity(typed, rule));
	});
}

document.querySelectorAll(".date-field__typed").forEach(initDateField);

/* PHASE NAVIGATION */

function goToPhase (phase) {
	wizard.dataset.phase = String(phase);
	window.scrollTo({ top: wizard.offsetTop - 20, behavior: "smooth" });
	/* Canvases sized while hidden have zero width, so they must be measured again here. */
	pads.forEach(p => setTimeout(p.resize, 60));
}

wizard.addEventListener("click", (e) => {
	const target = e.target.closest("[data-goto-phase]");
	if (!target) return;

	const next = target.dataset.gotoPhase;
	/* Going forward runs the browser's own validation on phase 1 only. The certificate has to
	   have been *read*, not merely chosen: `picked` below is what gets posted, and until the
	   read lands there is nothing to send. */
	if (next === "2" && !picked) {
		alert(certificate.files.length
			? "Attendi il controllo della foto del certificato medico."
			: "Scegli prima la foto del certificato medico.");
		return;
	}
	goToPhase(next);
});

/* CERTIFICATE PICKER */

const certificate = document.getElementById("certificate");
const filepick = document.getElementById("certificate-filepick");
const preview = document.getElementById("certificate-preview");
const previewContent = document.getElementById("certificate-preview-content");
const previewName = document.getElementById("certificate-preview-name");
const certError = document.getElementById("certificate-error");
const certSize = document.getElementById("certificate_size");
const nextButton = wizard.querySelector(".wizard__next");

/* Must match MAX_CERTIFICATE_BYTES in src/handlers/enrollment.rs, which stays authoritative.
   Checked here so the refusal arrives at the step that holds the picker, instead of after the
   whole form, two signatures and a long upload — and so a photo above the body limit in
   main.rs never gets to be cut off mid-request. */
const MAX_CERTIFICATE_BYTES = 12 * 1024 * 1024;

/*
 * The bytes of the chosen file, read at the moment it was chosen, and what actually gets posted.
 *
 * Not an optimization. The `File` in a file input is a *reference* to a copy the operating system
 * holds, and it is read when the form is serialized — by which point this form has been filled
 * in and signed twice. On iOS that copy may be gone by then, or may never have been finished:
 * Safari hands over a photo it was still transcoding out of HEIC, or one iCloud had not
 * downloaded, and what leaves the phone is a valid header followed by nothing. Android's
 * pickers do the same with a Google Foto file that is still in the cloud. That is how the grey
 * half-images arrived — a well-formed upload of an ill-formed file, which is why nothing along
 * the way looked broken.
 *
 * Reading here does two things: the failure surfaces on the step that holds the picker, while
 * the applicant still has the photo in front of them, and what is posted is this copy rather
 * than a second reading of a file that may have moved on.
 */
let picked = null;

/* Which read an answer belongs to. A second pick while one is still in flight must not have its
   preview and its bytes overwritten by the first one finishing. */
let reading = 0;

function showPreview (file) {
	filepick.hidden = true;
	certError.hidden = true;

	/* Named, not yet weighed: the figure that goes under the preview is the length of the bytes
	   that could actually be read, which is what the change listener below establishes. */
	previewName.textContent = `${file.name} — controllo del file…`;
	previewContent.replaceChildren();

	if (file.type.startsWith("image/")) {
		const img = document.createElement("img");
		img.src = URL.createObjectURL(file);
		img.className = "wizard__preview-img";
		img.alt = file.name;
		previewContent.appendChild(img);
	} else {
		const icon = document.createElement("div");
		icon.className = "wizard__preview-pdf";
		icon.textContent = "PDF";
		previewContent.appendChild(icon);
	}

	preview.hidden = false;
	/* And the step cannot be left until those bytes are in hand. */
	nextButton.disabled = true;
}

function clearPreview () {
	const dt = new DataTransfer();
	certificate.files = dt.files;
	picked = null;
	/* Abandons whatever read is in flight, so it cannot resurrect a preview of a file the
	   applicant has just removed. */
	reading++;
	certSize.value = "";
	previewContent.replaceChildren();
	previewName.textContent = "";
	preview.hidden = true;
	filepick.hidden = false;
	nextButton.disabled = true;
}

/* Whether the file matches the input's own `accept` list.

   The browser enforces that attribute in the file picker and nowhere else: a dropped file
   bypasses it entirely, and desktop pickers offer an "All files" escape hatch. So the list is
   read back off the DOM and applied to those paths too, rather than restated here — the
   markup stays the only place it lives, and adding a format there cannot leave this behind.

   A file whose type the browser cannot name is let through on purpose: some Android WebViews
   report nothing for an ordinary JPEG. This is a nudge, not the check — `validation::file_type`
   reads the actual bytes on the server, which is the only party that can tell a renamed HEIC
   from the JPEG it claims to be. */

/* The types an Android picker reaches for when it has nothing to say: a declaration of "some
   bytes" is not a claim about the format, and `enrollment.rs` exists in its current shape
   because an ordinary JPEG arrives announced like this. Refusing them here would block, on the
   step before the form, files the server goes on to accept. */
const OPAQUE_TYPES = ["application/octet-stream", "application/binary", "binary/octet-stream", "*/*"];

/* Spellings of an accepted format that the `accept` list cannot carry. `image/jpg` is not a
   registered MIME type — `image/jpeg` is — but several Android pickers report it anyway, and
   turning away a JPEG over how it was spelled is the same mistake as trusting the envelope. */
const TYPE_ALIASES = {
	"image/jpg": "image/jpeg",
	"image/pjpeg": "image/jpeg",
	"image/x-png": "image/png",
};

function accepted (file) {
	const declared = (file.type || "").toLowerCase();
	if (!declared || OPAQUE_TYPES.includes(declared)) return true;

	const type = TYPE_ALIASES[declared] || declared;
	return certificate.accept.split(",").some((pattern) => {
		pattern = pattern.trim().toLowerCase();
		if (!pattern) return false;
		if (pattern.endsWith("/*")) return type.startsWith(pattern.slice(0, -1));
		return type === pattern;
	});
}

certificate.addEventListener("change", async () => {
	const file = certificate.files[0];
	if (!file) {
		clearPreview();
		return;
	}
	/* Both refusals happen here rather than on submit: the certificate is on phase 1, so the
	   server's rejection would otherwise arrive after two signatures and a full form. */
	if (file.size > MAX_CERTIFICATE_BYTES) {
		clearPreview();
		/* Same sentence as `oversize_message` in enrollment.rs — the server keeps the limit,
		   this only delivers it sooner. */
		certError.textContent = `Il file pesa ${(file.size / (1024 * 1024)).toFixed(0)} MB: `
			+ `il massimo è ${MAX_CERTIFICATE_BYTES / (1024 * 1024)} MB. `
			+ "Scatta la foto a una risoluzione più bassa, oppure carica il PDF.";
		certError.hidden = false;
		return;
	}
	if (!accepted(file)) {
		clearPreview();
		certError.textContent = /heic|heif/i.test(file.type)
			? "Le foto in formato HEIC non si aprono sul computer di chi le riceve. "
			  + "Esportala in JPG, oppure carica un PDF."
			: "Puoi caricare una foto JPG o PNG, oppure un PDF. Questo file è di un altro tipo.";
		certError.hidden = false;
		return;
	}

	showPreview(file);

	/* On a 12 MB photo the read takes a moment, and until it lands there is nothing to post.
	   `turn` is what makes a second pick during that moment win over the first. */
	const turn = ++reading;

	let bytes;
	try {
		bytes = await file.arrayBuffer();
	} catch {
		if (turn !== reading) return;
		unreadable("Non è stato possibile leggere il file dal telefono.");
		return;
	}
	if (turn !== reading) return;

	/* Nothing came back at all: there is nothing to post, and nothing to judge either, so this
	   is the same dead end as a read that threw. */
	if (!bytes.byteLength) {
		unreadable("Il telefono non ha fornito il file.");
		return;
	}

	/* A copy of its own, so what is posted no longer depends on the original still being there.
	   The type is carried across as the browser reported it — the bytes are what the server
	   judges, and an empty or wrong claim is not this script's to correct.

	   Kept even when it came back *short* of `file.size`, deliberately. That disagreement is
	   between what a file says it weighs and the bytes behind it, and only one of the two is
	   evidence: the server walks the bytes and turns away a photo that stops mid-image, whatever
	   any size claim said. Refusing here on the claim would risk turning away a picker quirk
	   instead, and an applicant who cannot get off phase 1 does not enrol at all — worse than a
	   refusal that arrives later and is right. The figure posted below is the length actually
	   read, so a file that came up short is still there to be seen in the logs. */
	picked = new File([bytes], file.name, { type: file.type });
	certSize.value = bytes.byteLength;
	previewName.textContent = `${file.name} (${(bytes.byteLength / (1024 * 1024)).toFixed(1)} MB)`;
	nextButton.disabled = false;
});

/* The file could not be read whole. Both causes have the same way out, and it is not obvious
   enough to leave unsaid: the photo has to be on the phone, not in the cloud account behind it. */
function unreadable (cause) {
	clearPreview();
	certError.textContent = `${cause} Se la foto è su iCloud o su Google Foto, aprila prima `
		+ "nell'app per scaricarla sul telefono, poi riprova — oppure scattane una nuova.";
	certError.hidden = false;
}

document.getElementById("certificate-remove").addEventListener("click", clearPreview);

/* Drag-and-drop onto the picker area */
filepick.addEventListener("dragover", (e) => {
	e.preventDefault();
	filepick.classList.add("wizard__filepick--dragover");
});

filepick.addEventListener("dragleave", (e) => {
	if (!filepick.contains(e.relatedTarget))
		filepick.classList.remove("wizard__filepick--dragover");
});

filepick.addEventListener("drop", (e) => {
	e.preventDefault();
	filepick.classList.remove("wizard__filepick--dragover");
	const files = e.dataTransfer.files;
	if (files.length > 1) {
		certError.textContent = "Puoi caricare solo 1 file alla volta.";
		certError.hidden = false;
		return;
	}
	if (files.length === 1) {
		const dt = new DataTransfer();
		dt.items.add(files[0]);
		certificate.files = dt.files;
		certificate.dispatchEvent(new Event("change"));
	}
});

/* EMERGENCY CONTACTS */

/* Must match MAX_EMERGENCY_CONTACTS in src/handlers/enrollment.rs, which is authoritative:
   this only spares the user from filling in a row the server would reject. */
const MAX_CONTACTS = 4;

const contactList = document.getElementById("contact-list");
const addContact = document.getElementById("add-contact");
const contactMax = document.getElementById("contact-max");

function syncContactLimit () {
	const atLimit = contactList.querySelectorAll(".contact-row").length >= MAX_CONTACTS;
	addContact.hidden = atLimit;
	contactMax.hidden = !atLimit;
}

/* Delegated, so it also catches rows HTMX appends after page load. */
contactList.addEventListener("click", (e) => {
	const remove = e.target.closest("[data-remove-contact]");
	if (!remove) return;
	remove.closest(".contact-row").remove();
	syncContactLimit();
});

/* Fires after HTMX has appended the new row, so the count is already up to date and the
   row's four inputs are in the DOM waiting for their rules. */
contactList.addEventListener("htmx:afterSwap", () => {
	syncContactLimit();
	applyRules(contactList);
});

syncContactLimit();

/* SIGNATURE PADS */

/*
 * A pad keeps its strokes, not its pixels, and what counts as signed is measured rather than
 * remembered.
 *
 * Both halves of that are the same bug, and it reached the association twice: two membership
 * documents arrived with the autonomy signature drawn and the mandatory one an empty line. The
 * pad used to report a boolean set on the first pointer move — which a finger dragged across the
 * box on its way to scrolling the page sets just as readily, since `.signature-box` carries
 * `touch-action: none` — and it used to preserve the strokes around the wipe that assigning
 * `width` causes by copying the *pixels*. That only works while the pixels are still there to
 * copy: a browser that had already dropped the backing store (iOS does that under memory
 * pressure, and this page also holds the preview of a photo of up to 12 MB) handed back an empty
 * copy, and the flag survived it. Either way an untouched-looking canvas serialized to a
 * perfectly well-formed PNG, which passed every check on the way in and printed as a blank line.
 *
 * So the points are the record, in fractions of the box so they survive a rotation or a
 * re-measure, the canvas is re-rendered from them whenever it may have been wiped — including
 * immediately before it is serialized, the one moment that has to be right — and the ink in that
 * bitmap is counted against the same threshold the server counts against. See
 * `validation::signature`.
 */
function initPad (canvasId, inputId, errorId) {
	const canvas = document.getElementById(canvasId);
	if (!canvas) return null;
	const ctx = canvas.getContext("2d");
	const error = document.getElementById(errorId);
	/* The sentence the markup carries is for a pad nobody touched. The rule's own hint covers
	   the other case — a mark too small to be a signature — and is the same sentence the server
	   answers with, so a pad refused here reads like one refused on submit. */
	const untouched = error.textContent;
	const rule = RULES.get(inputId);
	/* Same share of the box `signature::MIN_INK_RATIO` requires, handed over in the rules JSON
	   so it is not written twice. The fallback matches it: a page whose rule went missing must
	   not start accepting blank pads. */
	const minInkRatio = rule?.minInkRatio ?? 0.003;

	/* One entry per stroke, each a list of `{x, y}` in fractions of the box. */
	const strokes = [];
	let drawing = false;

	function style () {
		ctx.lineWidth = 2;
		ctx.lineCap = "round";
		ctx.lineJoin = "round";
	}

	/* Point coordinates in CSS pixels, which is what the scaled context draws in. */
	const cssX = (p) => p.x * canvas.offsetWidth;
	const cssY = (p) => p.y * canvas.offsetHeight;

	/* Draws every stroke again, from the points. Cheap, and the only way back from a canvas
	   whose contents the browser decided it did not need. */
	function render () {
		if (canvas.offsetWidth === 0) return;
		ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight);
		style();
		for (const stroke of strokes) {
			ctx.beginPath();
			ctx.moveTo(cssX(stroke[0]), cssY(stroke[0]));
			for (const point of stroke.slice(1)) ctx.lineTo(cssX(point), cssY(point));
			/* A stroke of one point is a tap, which `lineCap: round` only draws as a dot if the
			   path goes somewhere. */
			if (stroke.length === 1) ctx.lineTo(cssX(stroke[0]), cssY(stroke[0]));
			ctx.stroke();
		}
	}

	/*
	 * Assigning `width` or `height` wipes the canvas — even when the number written is the one
	 * already there — and this runs on every phase change, so an unchanged box is left alone.
	 * Either way the strokes are drawn again afterwards: nothing here depends on what was in
	 * the canvas a moment ago.
	 */
	function resize () {
		if (canvas.offsetWidth === 0) return;
		const ratio = Math.max(window.devicePixelRatio || 1, 1);
		const width = Math.round(canvas.offsetWidth * ratio);
		const height = Math.round(canvas.offsetHeight * ratio);

		if (canvas.width !== width || canvas.height !== height) {
			canvas.width = width;
			canvas.height = height;
			/* The wipe also resets the transform, so this scale does not stack up across
			   calls. */
			ctx.scale(ratio, ratio);
		}
		render();
	}

	setTimeout(resize, 100);

	const pos = (e) => {
		const r = canvas.getBoundingClientRect();
		const touch = e.touches ? e.touches[0] : e;
		return { x: (touch.clientX - r.left) / r.width, y: (touch.clientY - r.top) / r.height };
	};

	const start = (e) => {
		drawing = true;
		strokes.push([pos(e)]);
		style();
	};

	const draw = (e) => {
		if (!drawing) return;
		const stroke = strokes[strokes.length - 1];
		const from = stroke[stroke.length - 1];
		const to = pos(e);
		stroke.push(to);

		/* The segment alone, not the whole pad: a full render on every pointer move would grow
		   heavier with each stroke. `render` is for the moments the canvas was wiped. */
		ctx.beginPath();
		ctx.moveTo(cssX(from), cssY(from));
		ctx.lineTo(cssX(to), cssY(to));
		ctx.stroke();

		error.hidden = true;
		if (e.cancelable) e.preventDefault();
	};

	const end = () => {
		drawing = false;
	};

	canvas.addEventListener("mousedown", start);
	canvas.addEventListener("mousemove", draw);
	window.addEventListener("mouseup", end);
	canvas.addEventListener("touchstart", start, { passive: false });
	canvas.addEventListener("touchmove", draw, { passive: false });
	window.addEventListener("touchend", end);

	/*
	 * The share of the pad that is painted, read off the bitmap that is about to be posted.
	 *
	 * Alpha and nothing else: the pad is never given a background, so transparency *is* the
	 * empty box. The floor mirrors `INK_ALPHA` in src/validation/signature.rs — the strokes are
	 * antialiased, so their edges arrive as partial alpha.
	 */
	function inkRatio () {
		if (!canvas.width || !canvas.height) return 0;
		const { data } = ctx.getImageData(0, 0, canvas.width, canvas.height);
		let ink = 0;
		for (let i = 3; i < data.length; i += 4) if (data[i] >= 32) ink++;
		return ink / (canvas.width * canvas.height);
	}

	return {
		resize,
		error,
		clear () {
			strokes.length = 0;
			ctx.clearRect(0, 0, canvas.width, canvas.height);
			document.getElementById(inputId).value = "";
		},
		/*
		 * Serializes the pad and says whether it is a signature. The hidden input is emptied
		 * when it is not: a value left over from an earlier attempt would otherwise be posted
		 * for a box that has since been cleared.
		 */
		save () {
			/* Re-drawn first, so the bytes measured below and the bytes posted are the strokes
			   the applicant actually made — whatever the browser did with the canvas since. */
			render();
			const signed = inkRatio() >= minInkRatio;
			document.getElementById(inputId).value = signed ? canvas.toDataURL("image/png") : "";
			if (!signed) error.textContent = strokes.length ? rule?.hint || untouched : untouched;
			return signed;
		},
	};
}

const mainPad = initPad("signature-pad", "signature", "signature-error");
const autonomyPad = initPad("autonomy-signature-pad", "autonomy_signature",
	"autonomy-signature-error");
const pads = [mainPad, autonomyPad].filter(Boolean);

document.querySelectorAll("button[data-clear]").forEach(btn => {
	btn.addEventListener("click", () => {
		if (btn.dataset.clear === "signature-pad") mainPad?.clear();
		if (btn.dataset.clear === "autonomy-signature-pad") autonomyPad?.clear();
	});
});

/* CONDITIONAL SECTIONS */

/*
 * CSS hides the minor and the autonomy blocks until their checkbox is ticked, so
 * their fields cannot carry `required` in the markup: a required field the user cannot see
 * would block the submission with a message the browser has nowhere to show. They get the
 * attribute here, while they are on screen, and lose it again when the section closes.
 */
const isMinor = document.getElementById("is_minor");
const commuteAlone = document.getElementById("commute_alone");
const minorSection = document.querySelector(".section-minor");
const autonomySection = document.querySelector(".section-autonomy");

/* `data-server-required` is excluded: the province fields are mandatory, but the sentence
   they need when empty names EE and only the server has it. See `Kind::Choice`. `tabindex="-1"`
   is the date picker, which posts nothing and must never be made required: it is invisible, so
   the browser would refuse the submission with a message it has nowhere to show. */
const CONDITIONAL_FIELD = "input:not([type=checkbox]):not([type=hidden])"
	+ ":not([data-server-required]):not([tabindex=\"-1\"])";
const autonomyFields = [...autonomySection.querySelectorAll(CONDITIONAL_FIELD)];
const minorFields = [...minorSection.querySelectorAll(CONDITIONAL_FIELD)]
	.filter(field => !autonomySection.contains(field));

/* Drops what the last attempt said about a field that is on its way off screen. The message is
   the visible half; `setCustomValidity` is the other, and it is sticky — left set, it keeps a
   field nobody can see, and nobody has to fill in, refusing the submission. */
function release (field) {
	field.setCustomValidity("");
	clearFieldError(field);
}

function syncConditionalSections () {
	/* The autonomy block sits inside the minor one, so it only counts when both are ticked. */
	const wantsAutonomy = isMinor.checked && commuteAlone.checked;
	minorFields.forEach(field => field.toggleAttribute("required", isMinor.checked));
	autonomyFields.forEach(field => field.toggleAttribute("required", wantsAutonomy));

	if (!wantsAutonomy && autonomyPad) autonomyPad.error.hidden = true;
	/* A message left standing inside a section that has been closed would come back with it. */
	if (!isMinor.checked) minorFields.forEach(release);
	if (!wantsAutonomy) autonomyFields.forEach(release);
	/* The autonomy canvas starts hidden, so it needs measuring once revealed. */
	setTimeout(() => autonomyPad?.resize(), 50);
}

isMinor.addEventListener("change", syncConditionalSections);
commuteAlone.addEventListener("change", syncConditionalSections);
syncConditionalSections();

/* SUBMIT */

function fail (error) {
	error.hidden = false;
	error.scrollIntoView({ behavior: "smooth", block: "center" });
	return false;
}

/*
 * Everything the browser cannot validate on its own. The fields the two toggles reveal are
 * left to constraint validation (see `syncConditionalSections`); the canvases and the two
 * consent checkboxes are not form controls, so they are checked here.
 */
function canSubmit () {
	/* Its own message lives on phase 1, next to the picker, so the wizard has to go back there
	   for it to be seen. Without the bytes there is nothing to send — and the file input's own
	   copy, which is what the form would fall back to, is precisely what must not be trusted. */
	if (!picked) {
		certError.textContent = "Manca la foto del certificato medico: scegliila di nuovo.";
		goToPhase(1);
		return fail(certError);
	}

	if (!(consentStatute.checked && consentPrivacy.checked)) return fail(consentError);

	/* `save` is what serializes each pad, and it answers whether what it serialized is a
	   signature rather than whether the box was touched. Without this the form went out with a
	   blank PNG in it, which is the whole reason the count exists. */
	if (mainPad && !mainPad.save()) return fail(mainPad.error);

	if (isMinor.checked && commuteAlone.checked) {
		if (autonomyPad && !autonomyPad.save()) return fail(autonomyPad.error);
	} else {
		/* A toggle ticked and then unticked must not leave its signature behind. */
		document.getElementById("autonomy_signature").value = "";
	}

	return true;
}

form.addEventListener("htmx:configRequest", (evt) => {
	if (evt.detail.elt !== form) return;

	if (!canSubmit()) {
		evt.preventDefault();
		return;
	}

	/*
	 * HTMX snapshots the form values *before* firing this event, so writing to the hidden
	 * inputs above are not enough — the freshly serialized signatures have to be pushed
	 * into the outgoing parameters by hand.
	 */
	evt.detail.parameters["signature"] = document.getElementById("signature").value;
	evt.detail.parameters["autonomy_signature"] = document.getElementById("autonomy_signature").value;

	/* The part HTMX just built for the file input is a fresh reading of the picked file, taken
	   now — long after the photo was chosen, and on iOS possibly of a copy that no longer exists
	   whole. Replaced with the bytes read on phase 1, which `canSubmit` has already established
	   are there. */
	evt.detail.parameters.set("certificate", picked);
});

/*
 * `hx-disabled-elt` on the form only covers the submit button, and the `htmx-indicator` span
 * inside it swaps the label for "Invio in corso" through CSS alone. Everything else on the
 * page has to be frozen from here: while the request is out the fields must not be edited and
 * the consent links must not open the Statute or the privacy notice in the dialog.
 *
 * `inert` is the guard — it kills clicks and takes the whole subtree out of the tab order —
 * and the class is what dims it. The wrapper is used rather than the wizard because the "back
 * to the home" link sits outside it and would otherwise stay clickable.
 */
const pageWrapper = document.querySelector(".main-wrapper");

function setSending (sending) {
	pageWrapper.inert = sending;
	pageWrapper.classList.toggle("main-wrapper--sending", sending);
}

form.addEventListener("htmx:beforeRequest", (evt) => {
	if (evt.detail.elt === form) setSending(true);
});

/* Covers the failures: the form is still on the page and keeps everything typed. */
form.addEventListener("htmx:afterRequest", (evt) => {
	if (evt.detail.elt === form) setSending(false);
});

/*
 * A successful send replaces the whole wizard (HX-Retarget + outerHTML), so the listener above
 * hangs off a detached form by then. The `enrollmentSent` trigger the response carries fires on
 * the body instead, which is what unfreezes the confirmation step. See the handler below.
 */

/* SERVER-SIDE REJECTION */

/*
 * The server refused the submission and sent, for each field, its name and the sentence
 * explaining it — the response body is empty on purpose. Nothing is listed anywhere: each
 * message is printed above its own field, the inputs are marked, and the wizard moves to the
 * step holding the first one and puts the cursor in it.
 *
 * A name may carry a `:index` suffix. That is how the server tells the emergency-contact
 * rows apart: they all post under the same four names, in DOM order.
 */
function inputNamed (reference) {
	const [name, index] = reference.split(":");
	const matches = form.querySelectorAll(`[name="${name}"]`);
	return matches[index === undefined ? 0 : Number(index)] || null;
}

function focusField (reference) {
	const input = inputNamed(reference);
	if (!input) return;

	/* The certificate is on the first step and everything else on the second, so the phase
	   comes from the input rather than from an assumption. */
	const phase = input.closest("[data-phase]");
	if (phase) goToPhase(phase.dataset.phase);

	/* After the smooth scroll goToPhase starts, or the two fight over the viewport. A
	   signature is a hidden input and cannot take focus, so its field is scrolled to instead. */
	setTimeout(() => {
		if (input.type === "hidden") {
			input.closest(".membership-form__field")
				?.scrollIntoView({ behavior: "smooth", block: "center" });
			return;
		}
		input.focus();
		input.scrollIntoView({ behavior: "smooth", block: "center" });
	}, 400);
}

body.addEventListener("enrollmentInvalid", (e) => {
	const errors = e.detail?.fields ?? [];

	clearAllFieldErrors();
	for (const error of errors) {
		const input = inputNamed(error.field);
		if (input) showFieldError(input, error.message);
	}

	if (errors.length) focusField(errors[0].field);
});

/* CONSENT CHECKBOXES */

const consentStatute = document.getElementById("consent-statute");
const consentPrivacy = document.getElementById("consent-privacy");
const consentError = document.getElementById("consent-error");
const submitBtn = form.querySelector("button[type=\"submit\"]");

function syncSubmitBtn () {
	submitBtn.disabled = !(consentStatute.checked && consentPrivacy.checked);
	if (!submitBtn.disabled) consentError.hidden = true;
}

consentStatute.addEventListener("change", syncSubmitBtn);
consentPrivacy.addEventListener("change", syncSubmitBtn);
/* Re-sync after HTMX re-enables elements following any sub-request (e.g., add contact) */
form.addEventListener("htmx:afterRequest", syncSubmitBtn);

/* ENTER KEY */

/*
 * Enter while filling in a field must never send the form — on a form this long it is far
 * too easy to fire the request halfway through — so it moves to the next control instead.
 * Buttons and links keep their native behavior: tabbing onto "Conferma e invia" and
 * pressing Enter still submits.
 */
/* The date pickers are left out along with the hidden inputs: they are already out of the tab
   order, and Enter should reach the next field rather than a calendar. */
const FOCUSABLE = "input:not([type=hidden]):not([tabindex=\"-1\"]), select, textarea, button,"
	+ " a[href]";

form.addEventListener("keydown", (e) => {
	if (e.key !== "Enter" || e.isComposing) return;
	if (e.target.matches("button, a[href], textarea")) return;

	e.preventDefault();

	/* `offsetParent` is null for the phase, and the sections CSS is currently hiding. */
	const fields = [...form.querySelectorAll(FOCUSABLE)]
		.filter(field => !field.disabled && field.offsetParent !== null);
	const current = fields.indexOf(e.target);
	if (current !== -1) fields[current + 1]?.focus();
});

/* Leaving midway loses everything typed, so warn unless the sending already succeeded. */
let submitted = false;
body.addEventListener("enrollmentSent", () => {
	submitted = true;
	/* The confirmation step is now on screen and its WhatsApp button has to be clickable. */
	setSending(false);
});
window.addEventListener("beforeunload", (e) => {
	if (submitted) return;
	if (!document.getElementById("applicant_email").value && !certificate.files.length) return;
	e.preventDefault();
});

/*
 * DEV ONLY — call window.__phase3() in the console to preview the confirmation step.
 *
 * The markup comes from the same partial a real submission returns, served by a route that
 * only exists in debug builds, so the preview cannot drift from what an applicant sees. The
 * response carries `HX-Trigger: enrollmentSent`, which lifts the unload guard above.
 */
window.__phase3 = () => htmx.ajax("GET", "/enrollment/preview-sent", {
	target: "#enrollment-wizard",
	swap: "outerHTML",
});
