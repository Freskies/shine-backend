/*
 * Generates the wave tile used by `.wave-divider` in static/css/index.css.
 *
 * The wave is emitted as a single SVG tile of fixed pixel width, applied as a repeating
 * *mask* (`mask-repeat: repeat-x`) over a solid fill. That is the whole point: a stretched
 * SVG would squeeze the crests on narrow screens, while a fixed tile keeps them the same
 * size everywhere and simply shows fewer of them.
 *
 * One tile serves both edges of a section. The tile fills upwards, which is what the bottom
 * divider needs, and `.wave-divider--top` mirrors it with `transform: scaleY(-1)` rather
 * than shipping a second asset.
 *
 * Seamlessness comes from a *periodic* Catmull-Rom spline: neighboring nodes wrap around
 * with a +tileWidth shift, so position and tangent both match at the seam and the repetition
 * is invisible. That matters more than it used to — the tile now drifts, so a mismatched
 * seam would be a defect travelling across the screen rather than a static one nobody sees.
 */

/*
 * The tile is a mask, so only its alpha is read: `currentColor` resolves against the SVG's
 * own document (black, fully opaque) and the visible colour comes from `--wave-fill` in
 * index.css. Baking a pastel in here would be discarded.
 */
const MASK_FILL = "currentColor";

/* Mirrors `height: calc(var(--wave-h) * 0.7)` in static/css/index.css. */
const PHONE_HEIGHT_FACTOR = 0.7;

let lastCss = "";
let lastSvg = "";

/* Deterministic PRNG, so a given seed always rebuilds the same wave. */
function mulberry32 (a) {
	return function () {
		a |= 0;
		a = a + 0x6D2B79F5 | 0;
		let t = Math.imul(a ^ a >>> 15, 1 | a);
		t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t;
		return ((t ^ t >>> 14) >>> 0) / 4294967296;
	};
}

function readParams () {
	const num = (id) => parseFloat(document.getElementById(id).value);
	const height = num("height");
	return {
		width: num("tile"),
		height,
		nodes: num("nodes"),
		jitter: num("jitter"),
		tension: num("tension"),
		seed: num("seed"),
		speed: num("speed"),
		/* Keep the crests inside the tile: the spline overshoots the nodes, hence the margin. */
		amplitude: Math.min(num("amp"), height / 2 - 6),
	};
}

function buildNodes ({ width, height, amplitude, nodes, jitter, seed }) {
	const rnd = mulberry32(seed);
	const step = width / nodes;
	const baseline = height / 2;
	const pts = [];

	for (let i = 0; i < nodes; i++) {
		/* Node 0 is pinned to x=0: the tile has to start exactly on its left edge. */
		const jx = i === 0 ? 0 : (rnd() * 2 - 1) * jitter * step * 0.45;
		const jy = (rnd() * 2 - 1) * amplitude;
		pts.push({ x: i * step + jx, y: baseline + jy });
	}
	/*
	 * Mirrored once, here, so every consumer below sees the shipped orientation: solid above
	 * the wave line, transparent below. Drawn the other way up the tile would need flipping
	 * at every use site instead of just on `--top`.
	 */
	return pts.map((q) => ({ x: q.x, y: height - q.y }));
}

function wavePath (pts, width, tension) {
	const n = pts.length;
	/* Wrapped lookup: index n maps back to node 0 shifted one tile to the right. */
	const P = (i) => {
		const k = ((i % n) + n) % n;
		return { x: pts[k].x + Math.floor(i / n) * width, y: pts[k].y };
	};
	const f = (v) => Math.round(v * 100) / 100;

	let d = `M${f(P(0).x)},${f(P(0).y)}`;
	for (let i = 0; i < n; i++) {
		const p0 = P(i - 1), p1 = P(i), p2 = P(i + 1), p3 = P(i + 2);
		const c1 = { x: p1.x + (p2.x - p0.x) / 6 * tension, y: p1.y + (p2.y - p0.y) / 6 * tension };
		const c2 = { x: p2.x - (p3.x - p1.x) / 6 * tension, y: p2.y - (p3.y - p1.y) / 6 * tension };
		d += ` C${f(c1.x)},${f(c1.y)} ${f(c2.x)},${f(c2.y)} ${f(p2.x)},${f(p2.y)}`;
	}
	return d;
}

function wavePathClosed (pts, { width, tension }) {
	/*
	 * The fill runs 1px past the viewBox, which clips it exactly on the boundary. That avoids
	 * an antialiased half-row along the flat edge when the tile is scaled to a fractional
	 * height, which would show as a hairline against the adjacent section.
	 */
	return wavePath(pts, width, tension) + ` L${width},-1 L0,-1 Z`;
}

/*
 * No `width`/`height` attributes, only a viewBox: the element is sized by `mask-size` in
 * index.css, and intrinsic dimensions here would just be one more thing to keep in sync.
 */
function buildSvg (pts, cfg, pretty) {
	const d = wavePathClosed(pts, cfg);
	const open = `<svg fill="${MASK_FILL}" xmlns="http://www.w3.org/2000/svg" `
		+ `viewBox="0 0 ${cfg.width} ${cfg.height}" preserveAspectRatio="none">`;

	return pretty
		? `<!--suppress LongLine -->\n${open}\n\t<path\n\t\td="${d}"/>\n</svg>\n`
		: `${open}<path d="${d}"/></svg>`;
}

/* Escape only what would break a CSS url(), so the stylesheet stays diffable. */
function toDataUri (svg) {
	const encoded = encodeURIComponent(svg)
		.replace(/%20/g, " ")
		.replace(/%2C/g, ",")
		.replace(/%3D/g, "=")
		.replace(/%3A/g, ":")
		.replace(/%2F/g, "/");
	return `data:image/svg+xml,${encoded}`;
}

function updateUI () {
	["tile", "height", "nodes", "amp", "jitter", "tension", "speed", "seed"].forEach((id) => {
		document.getElementById("val-" + id).innerText = document.getElementById(id).value;
	});
}

function applySpeed () {
	const speed = document.getElementById("speed").value + "s";
	document.querySelectorAll(".stage").forEach((s) => s.style.setProperty("--speed", speed));
}

function randomSeed () {
	document.getElementById("seed").value = Math.floor(Math.random() * 200) + 1;
	updateUI();
	generate();
}

function generate () {
	const cfg = readParams();
	const pts = buildNodes(cfg);
	const maskUri = toDataUri(buildSvg(pts, cfg, false));

	document.querySelectorAll(".stage").forEach((stage) => {
		stage.style.setProperty("--tile-w", cfg.width + "px");
		/*
		 * One mask for both edges, exactly as in production — `.wave--top` mirrors it with
		 * scaleY(-1) instead of receiving a separately generated flipped tile.
		 */
		stage.style.setProperty("--wave-mask", `url("${maskUri}")`);
		/* Production shrinks the divider below 960px, so the phone column must too. */
		const height = stage.id === "stage-phone" ? cfg.height * PHONE_HEIGHT_FACTOR : cfg.height;
		stage.querySelectorAll(".wave").forEach((w) => {
			w.style.height = Math.round(height) + "px";
		});
	});

	reportSeam(pts, cfg);

	lastSvg = buildSvg(pts, cfg, true);

	/*
	 * Data only — never behaviour. The mask, the tiling and the `transform` drift live
	 * permanently in index.css; emitting them here invites pasting over them, and the last
	 * time that happened the keyframes came across interpolating `0 0` to `0 0` and the
	 * dividers sat still for weeks.
	 */
	lastCss = `/* ============================================================================\n`
		+ ` * GENERATED by playground/wave_divider_playground.html\n`
		+ ` * Replace these three custom properties and nothing else. This block carries\n`
		+ ` * data only — the mask, the tiling and the animation must survive a regeneration.\n`
		+ ` * Seed ${cfg.seed}, ${cfg.nodes} nodes, amplitude ${cfg.amplitude}, tension ${cfg.tension}.\n`
		+ ` * ========================================================================= */\n`
		+ `.wave-divider {\n`
		+ `\t--wave-tile: ${cfg.width}px;\n`
		+ `\t--wave-h: ${cfg.height}px;\n`
		+ `\t--wave-speed: ${cfg.speed}s;\n`
		+ `}\n`
		+ `/* === END GENERATED === */\n`;

	document.getElementById("output-css").value = lastCss;
	document.getElementById("output-svg").value = lastSvg;
}

/*
 * Surfaces the two things that actually make a tile unusable: a seam that does not line up,
 * or crests clipped by the tile height.
 */
function reportSeam (pts, cfg) {
	const ys = pts.map((p) => p.y);
	const lo = Math.min(...ys), hi = Math.max(...ys);
	/* One full wave spans two nodes: a crest plus a trough. */
	const wavesPerTile = cfg.nodes / 2;
	const wavesOnPhone = (390 / cfg.width * wavesPerTile).toFixed(1);
	const wavesOnDesktop = (1400 / cfg.width * wavesPerTile).toFixed(1);
	const clipped = lo < 2 || hi > cfg.height - 2;

	document.getElementById("seam-check").innerHTML =
		`<strong>Giunzione:</strong> continua per costruzione — il nodo iniziale e quello finale `
		+ `sono lo stesso punto (y = ${pts[0].y.toFixed(1)}) e le tangenti combaciano. `
		+ `<strong>Onde intere visibili:</strong> ~${wavesOnPhone} su telefono (390px), `
		+ `~${wavesOnDesktop} su desktop (1400px). `
		+ (clipped
			? `<strong style="color:#e9c46a">Attenzione:</strong> le creste sfiorano il bordo, abbassa l'ampiezza.`
			: `Creste entro l'altezza.`);
}

function copyToClipboard (buttonId, text, label) {
	const btn = document.getElementById(buttonId);
	navigator.clipboard.writeText(text).then(() => {
		btn.innerText = "Copiato";
		setTimeout(() => {
			btn.innerText = label;
		}, 2500);
	}).catch(() => {
		alert("Copia fallita, seleziona il codice dal riquadro qui sotto.");
	});
}

function copyCss () {
	copyToClipboard("copy-css-btn", lastCss, "Copia Blocco CSS");
}

function copySvg () {
	copyToClipboard("copy-svg-btn", lastSvg, "Copia File SVG");
}

window.onload = () => {
	updateUI();
	applySpeed();
	generate();
};
