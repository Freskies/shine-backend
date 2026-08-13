let currentClipPath = "";
let waveData = {};

function updateUI () {
	["round", "res", "pad", "amp", "speed"].forEach(id => {
		document.getElementById("val-" + id).innerText = document.getElementById(id).value;
	});
}

function updateSpeed () {
	document.getElementById("box").style.setProperty("--speed", document.getElementById("speed").value + "s");
}

function scrambleWaves () {
	waveData = {
		tf1: Math.floor(Math.random() * 2) + 1,
		tf2: Math.floor(Math.random() * 2) + 1,
		sf1: Math.floor(Math.random() * 3) + 1,
		sf2: Math.floor(Math.random() * 4) + 2,
		ph1: Math.floor(Math.random() * 360),
		ph2: Math.floor(Math.random() * 360),
	};
	generateForm();
}

// MODIFICATO: L'unità finale della stringa è ora % invece di px
function buildMathTerm (func, timeFreq, phaseDeg, amplitude) {
	if (Math.abs(amplitude) < 0.01) return "";
	let sign = amplitude >= 0 ? "+" : "-";
	return `${sign} (${func}(var(--a) * ${timeFreq} + ${phaseDeg.toFixed(1)}deg) * ${Math.abs(amplitude).toFixed(2)}%)`;
}

function generateForm () {
	const res = parseInt(document.getElementById("res").value);
	const pad = parseInt(document.getElementById("pad").value);

	// MODIFICATO: Preleviamo l'ampiezza e applichiamo una rete di sicurezza
	let requestedAmp = parseFloat(document.getElementById("amp").value);
	// SISTEMA ANTI-TAGLIO: L'onda non può mai superare lo spazio di padding (con uno scarto di 0.5% per sicurezza)
	const amp = Math.min(requestedAmp, pad - 0.5);

	const roundness = parseFloat(document.getElementById("round").value);

	let p = [];
	const e = 2 / roundness;

	for (let i = 0; i < res; i++) {
		let theta = (i / res) * Math.PI * 2;
		let thetaDeg = (i / res) * 360;

		let cosT = Math.cos(theta);
		let sinT = Math.sin(theta);

		let maxDist = 50 - pad;
		let x0 = 50 + maxDist * Math.pow(Math.abs(cosT), e) * Math.sign(cosT);
		let y0 = 50 + maxDist * Math.pow(Math.abs(sinT), e) * Math.sign(sinT);

		let nx = cosT;
		let ny = sinT;

		let amp1X = amp * 0.6 * nx;
		let amp1Y = amp * 0.6 * ny;
		let amp2X = amp * 0.4 * nx;
		let amp2Y = amp * 0.4 * ny;

		let phase1 = thetaDeg * waveData.sf1 + waveData.ph1;
		let phase2 = thetaDeg * waveData.sf2 + waveData.ph2;

		let termX1 = buildMathTerm("sin", waveData.tf1, phase1, amp1X);
		let termX2 = buildMathTerm("cos", waveData.tf2, phase2, amp2X);

		let termY1 = buildMathTerm("sin", waveData.tf1, phase1, amp1Y);
		let termY2 = buildMathTerm("cos", waveData.tf2, phase2, amp2Y);

		let pointX = `calc(${x0.toFixed(2)}% ${termX1} ${termX2})`;
		let pointY = `calc(${y0.toFixed(2)}% ${termY1} ${termY2})`;

		p.push(`${pointX} ${pointY}`);
	}

	currentClipPath = `polygon(\n  ${p.join(",\n  ")}\n)`;
	document.getElementById("box").style.clipPath = currentClipPath;
}

function copyCode () {
	const btn = document.getElementById("copy-btn");
	const textToCopy = `clip-path: ${currentClipPath};`;

	navigator.clipboard.writeText(textToCopy).then(() => {
		btn.innerText = "Copiato";
		btn.style.backgroundColor = "#e9c46a";
		btn.style.color = "#333";
		btn.style.borderColor = "#e9c46a";

		setTimeout(() => {
			btn.innerText = "Copia Codice CSS";
			btn.style.backgroundColor = "transparent";
			btn.style.color = "var(--accent)";
			btn.style.borderColor = "var(--accent)";
		}, 2500);
	}).catch(_ => {
		const fallback = document.getElementById("fallback-output");
		fallback.style.display = "block";
		fallback.value = textToCopy;
		fallback.select();
		alert("Copia fallita, copia il codice dal riquadro in basso.");
	});
}

window.onload = () => {
	updateUI();
	updateSpeed();
	scrambleWaves();
};