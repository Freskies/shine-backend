"use strict";

/*
 * Enrolment wizard: phase navigation, signature pads, emergency-contact removal and the
 * pre-submit checks.
 *
 * The two phases are sections of a single form, toggled by `data-phase` on the wrapper.
 * They are never fetched or removed because a file input cannot be repopulated from
 * script: dropping phase 1 from the DOM would lose the chosen photo.
 */

const body = document.querySelector("body");
const wizard = document.getElementById("enrollment-wizard");
const form = document.getElementById("enrollment-form");

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

/* Fires after HTMX has appended the new row, so the count is already up to date. */
contactList.addEventListener("htmx:afterSwap", syncContactLimit);

syncContactLimit();

/* SIGNATURE PADS */

function initPad (canvasId, inputId) {
	const canvas = document.getElementById(canvasId);
	if (!canvas) return null;
	const ctx = canvas.getContext("2d");
	let drawing = false, drawn = false;

	function resize () {
		if (canvas.offsetWidth === 0) return;
		const ratio = Math.max(window.devicePixelRatio || 1, 1);
		canvas.width = canvas.offsetWidth * ratio;
		canvas.height = canvas.offsetHeight * ratio;
		ctx.scale(ratio, ratio);
		ctx.lineWidth = 2;
		ctx.lineCap = "round";
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

const mainPad = initPad("signature-pad", "signature");
const autonomyPad = initPad("autonomy-signature-pad", "autonomy_signature");
const pads = [mainPad, autonomyPad].filter(Boolean);

document.querySelectorAll("button[data-clear]").forEach(btn => {
	btn.addEventListener("click", () => {
		if (btn.dataset.clear === "signature-pad") mainPad?.clear();
		if (btn.dataset.clear === "autonomy-signature-pad") autonomyPad?.clear();
	});
});

/* The autonomy canvas starts hidden, so it needs measuring once revealed. */
["is_minor", "commute_alone"].forEach(id => {
	document.getElementById(id)?.addEventListener("change",
		() => setTimeout(() => autonomyPad?.resize(), 50));
});

/* SUBMIT */

const SUBMIT_LABEL = "Conferma e invia";
const SUBMIT_LOADING_HTML = `Invio in corso <span class="loading-dots" aria-hidden="true"><span>.</span><span>.</span><span>.</span></span>`;

form.addEventListener("htmx:configRequest", (evt) => {
	if (evt.detail.elt !== form) return;

	const isMinor = document.getElementById("is_minor")?.checked;
	const commuteAlone = document.getElementById("commute_alone")?.checked;

	mainPad?.save();
	if (!mainPad?.hasDrawn()) {
		alert("Manca la firma: tracciala nel riquadro prima di inviare.");
		evt.preventDefault();
		return;
	}

	if (isMinor && commuteAlone) {
		autonomyPad?.save();
		if (!autonomyPad?.hasDrawn()) {
			alert("Hai indicato che il minore fa il tragitto da solo: serve anche la seconda firma.");
			evt.preventDefault();
			return;
		}
	} else {
		document.getElementById("autonomy_signature").value = "";
	}

	/*
	 * HTMX snapshots the form values *before* firing this event, so writing to the hidden
	 * inputs above are not enough — the freshly serialized signatures have to be pushed
	 * into the outgoing parameters by hand.
	 */
	evt.detail.parameters["signature"] = document.getElementById("signature").value;
	evt.detail.parameters["autonomy_signature"] = document.getElementById("autonomy_signature").value;

	submitBtn.innerHTML = SUBMIT_LOADING_HTML;
	submitBtn.disabled = true;
});

/* Restore button label if the server returned an error (on success the whole wizard is replaced). */
form.addEventListener("htmx:afterRequest", (evt) => {
	if (evt.detail.elt !== form) return;
	submitBtn.textContent = SUBMIT_LABEL;
});

/* CONSENT CHECKBOXES */

const consentStatute = document.getElementById("consent-statute");
const consentPrivacy = document.getElementById("consent-privacy");
const submitBtn = form.querySelector('button[type="submit"]');

function syncSubmitBtn () {
	submitBtn.disabled = !(consentStatute.checked && consentPrivacy.checked);
}

consentStatute.addEventListener("change", syncSubmitBtn);
consentPrivacy.addEventListener("change", syncSubmitBtn);
/* Re-sync after HTMX re-enables elements following any sub-request (e.g. add contact) */
form.addEventListener("htmx:afterRequest", syncSubmitBtn);

/* Leaving midway loses everything typed, so warn unless the sending already succeeded. */
let submitted = false;
body.addEventListener("enrollmentSent", () => {
	submitted = true;
});
window.addEventListener("beforeunload", (e) => {
	if (submitted) return;
	if (!document.getElementById("applicant_email").value && !certificate.files.length) return;
	e.preventDefault();
});

/* DEV ONLY — call window.__phase3() in the console to preview the confirmation step. */
window.__phase3 = () => {
	document.getElementById("enrollment-wizard").outerHTML =
		`<div class="wizard" data-phase="3">
			<ol class="wizard__steps" aria-label="Avanzamento">
				<li class="wizard__stepper" data-step="1">Certificato</li>
				<li class="wizard__stepper" data-step="2">I tuoi dati</li>
				<li class="wizard__stepper" data-step="3">Conferma</li>
			</ol>
			<div class="wizard__done">
				<p class="wizard__step-count">Passo 3 di 3</p>
				<h2 class="heading-secondary">Ci siamo quasi!</h2>
				<p class="wizard__lead">
					I tuoi dati sono stati registrati. Ti abbiamo inviato una copia di riepilogo a
					<strong>test@example.com</strong>, con il modulo di tesseramento in allegato.
				</p>
				<p class="wizard__lead">
					Manca solo un passaggio: apri WhatsApp e mandaci il messaggio già pronto, così sappiamo che
					possiamo procedere.
				</p>
				<p class="wizard__notice wizard__notice--warn">
					[Anteprima — il pulsante WhatsApp non è attivo in questa modalità]
				</p>
				<p class="wizard__hint">Non usi WhatsApp? Va bene comunque: abbiamo già tutto, ti scriviamo noi.</p>
				<a href="/" class="wizard__back-home">Torna alla home</a>
			</div>
		</div>`;
	submitted = true;
};
