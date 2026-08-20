/*
 * Enrolment wizard: phase navigation, signature pads, emergency-contact removal and the
 * pre-submit checks.
 *
 * The two phases are sections of a single form, toggled by `data-phase` on the wrapper.
 * They are never fetched or removed, because a file input cannot be repopulated from
 * script: dropping phase 1 from the DOM would lose the chosen photo.
 */

const wizard = document.getElementById("enrollment-wizard");
const form = document.getElementById("enrollment-form");

/* ---------------------------------------------------------------- */
/* PHASE NAVIGATION                                                 */
/* ---------------------------------------------------------------- */

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

/* ---------------------------------------------------------------- */
/* CERTIFICATE PICKER                                               */
/* ---------------------------------------------------------------- */

const certificate = document.getElementById("certificate");
const chosen = document.getElementById("certificate-chosen");
const nextButton = wizard.querySelector(".wizard__next");

certificate.addEventListener("change", () => {
	const file = certificate.files[0];
	if (!file) {
		chosen.textContent = "";
		nextButton.disabled = true;
		return;
	}
	const megabytes = (file.size / (1024 * 1024)).toFixed(1);
	chosen.textContent = `Selezionato: ${file.name} (${megabytes} MB)`;
	nextButton.disabled = false;
});

/* ---------------------------------------------------------------- */
/* EMERGENCY CONTACTS                                               */
/* ---------------------------------------------------------------- */

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

/* ---------------------------------------------------------------- */
/* SIGNATURE PADS                                                   */
/* ---------------------------------------------------------------- */

function initPad (canvasId, inputId) {
	const canvas = document.getElementById(canvasId);
	if (!canvas) return null;
	const ctx = canvas.getContext("2d");
	let drawing = false, hasDrawn = false;

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
		hasDrawn = true;
		if (e.cancelable) e.preventDefault();
	};

	const end = () => drawing = false;

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
			hasDrawn = false;
			document.getElementById(inputId).value = "";
		},
		save () {
			document.getElementById(inputId).value = hasDrawn ? canvas.toDataURL("image/png") : "";
		},
		hasDrawn: () => hasDrawn,
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

/* ---------------------------------------------------------------- */
/* SUBMIT                                                           */
/* ---------------------------------------------------------------- */

form.addEventListener("htmx:configRequest", (evt) => {
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
	 * inputs above is not enough — the freshly serialised signatures have to be pushed
	 * into the outgoing parameters by hand.
	 */
	evt.detail.parameters["signature"] = document.getElementById("signature").value;
	evt.detail.parameters["autonomy_signature"] = document.getElementById("autonomy_signature").value;
});

/* Leaving mid-way loses everything typed, so warn unless the send already succeeded. */
let submitted = false;
document.body.addEventListener("enrollmentSent", () => submitted = true);
window.addEventListener("beforeunload", (e) => {
	if (submitted) return;
	if (!document.getElementById("applicant_email").value && !certificate.files.length) return;
	e.preventDefault();
});
