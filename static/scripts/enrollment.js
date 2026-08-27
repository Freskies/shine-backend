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
	if (v.rangeUnderflow) return rule.minMessage || rule.hint;
	if (v.rangeOverflow) return rule.maxMessage || rule.hint;
	/* A date left half typed — a day and a month with no year — is `badInput`, and the hint a
	   date rule carries is about its window, which has nothing to say about that. */
	if (v.badInput && input.type === "date") return "Questa non è una data valida.";
	if (v.patternMismatch || v.typeMismatch || v.badInput) return rule.hint;
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
		   nothing for the browser to enforce and no `blur` to enforce it on; whether the pad
		   was drawn on is reported by `canSubmit` instead. */
		if (input.type === "hidden") continue;
		input.dataset.ruled = "1";

		if (rule.pattern) input.pattern = rule.pattern;
		if (rule.minLength) input.minLength = rule.minLength;
		if (rule.maxLength) input.maxLength = rule.maxLength;
		if (rule.min) input.min = rule.min;
		if (rule.max) input.max = rule.max;
		input.title = rule.hint;

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

/*
 * The two birth dates need nothing beyond the loop above. They are `type="date"` inputs, so the
 * digits can be typed straight in and the picker is the browser's own, and `min`/`max` from the
 * rule are enforced by constraint validation like every other field on the page — the
 * eighteen-year window included, with the sentence explaining the minor toggle as its message.
 */

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
	/* Going forward runs the browser's own validation on phase 1 only. */
	if (next === "2" && !document.getElementById("certificate").files.length) {
		alert("Scegli prima la foto del certificato medico.");
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
const nextButton = wizard.querySelector(".wizard__next");

function showPreview (file) {
	filepick.hidden = true;
	certError.hidden = true;

	previewName.textContent = `${file.name} (${(file.size / (1024 * 1024)).toFixed(1)} MB)`;
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
	nextButton.disabled = false;
}

function clearPreview () {
	const dt = new DataTransfer();
	certificate.files = dt.files;
	previewContent.replaceChildren();
	previewName.textContent = "";
	preview.hidden = true;
	filepick.hidden = false;
	nextButton.disabled = true;
}

certificate.addEventListener("change", () => {
	const file = certificate.files[0];
	if (!file) {
		clearPreview();
		return;
	}
	showPreview(file);
});

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

function initPad (canvasId, inputId, errorId) {
	const canvas = document.getElementById(canvasId);
	if (!canvas) return null;
	const ctx = canvas.getContext("2d");
	const error = document.getElementById(errorId);
	let drawing = false, drawn = false;

	/*
	 * Assigning `width` or `height` wipes the canvas — even when the number written is the
	 * one already there — and this runs on every phase change. That is how a refused
	 * submission used to erase a signature the hidden input was still carrying: the applicant
	 * saw an empty box and sent the stroke anyway. So an unchanged box is left alone, and a
	 * box that really did change gets its strokes drawn back.
	 */
	function resize () {
		if (canvas.offsetWidth === 0) return;
		const ratio = Math.max(window.devicePixelRatio || 1, 1);
		const width = canvas.offsetWidth * ratio;
		const height = canvas.offsetHeight * ratio;
		if (canvas.width === width && canvas.height === height) return;

		/* Copied before the wipe, because the canvas is the only place the strokes exist. */
		let previous = null;
		if (drawn) {
			previous = document.createElement("canvas");
			previous.width = canvas.width;
			previous.height = canvas.height;
			previous.getContext("2d").drawImage(canvas, 0, 0);
		}

		canvas.width = width;
		canvas.height = height;
		/* The wipe also resets the transform, so this scale does not stack up across calls. */
		ctx.scale(ratio, ratio);
		ctx.lineWidth = 2;
		ctx.lineCap = "round";
		/* In CSS pixels, since the context is scaled: the old box is stretched over the new
		   one, which is what a rotated phone should show. */
		if (previous) ctx.drawImage(previous, 0, 0, canvas.offsetWidth, canvas.offsetHeight);
	}

	setTimeout(resize, 100);

	const pos = (e) => {
		const r = canvas.getBoundingClientRect();
		const clientX = e.touches ? e.touches[0].clientX : e.clientX;
		const clientY = e.touches ? e.touches[0].clientY : e.clientY;
		return { x: clientX - r.left, y: clientY - r.top };
	};

	const start = (e) => {
		drawing = true;
		const p = pos(e);
		ctx.beginPath();
		ctx.moveTo(p.x, p.y);
	};

	const draw = (e) => {
		if (!drawing) return;
		const p = pos(e);
		ctx.lineTo(p.x, p.y);
		ctx.stroke();
		drawn = true;
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

	return {
		resize,
		error,
		clear () {
			ctx.clearRect(0, 0, canvas.width, canvas.height);
			drawn = false;
			document.getElementById(inputId).value = "";
		},
		save () {
			document.getElementById(inputId).value = drawn ? canvas.toDataURL("image/png") : "";
		},
		hasDrawn: () => drawn,
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
   they need when empty names EE and only the server has it. See `Kind::Choice`. */
const CONDITIONAL_FIELD =
	"input:not([type=checkbox]):not([type=hidden]):not([data-server-required])";
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
	if (!(consentStatute.checked && consentPrivacy.checked)) return fail(consentError);

	mainPad?.save();
	if (mainPad && !mainPad.hasDrawn()) return fail(mainPad.error);

	if (isMinor.checked && commuteAlone.checked) {
		autonomyPad?.save();
		if (autonomyPad && !autonomyPad.hasDrawn()) return fail(autonomyPad.error);
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
const FOCUSABLE = "input:not([type=hidden]), select, textarea, button, a[href]";

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
