/*
 * Generates the wave tiles used by `.wave-divider` in static/style.css.
 *
 * The wave is emitted as a single SVG tile of fixed pixel width, meant to be applied
 * as a repeating background (`background-repeat: repeat-x`). That is the whole point:
 * a stretched SVG would squeeze the crests on narrow screens, while a fixed tile keeps
 * them the same size everywhere and simply shows fewer of them.
 *
 * Seamlessness comes from a *periodic* Catmull-Rom spline: neighboring nodes wrap
 * around with a +tileWidth shift, so position and tangent both match at the seam and
 * the repetition is invisible.
 */

const WAVE_FILL = "#ecfdf5";

/* Mirrors `height: calc(var(--wave-h) * 0.7)` in static/style.css. */
const PHONE_HEIGHT_FACTOR = 0.7;

let lastCss = "";

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
	return pts;
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

/*
 * `flip` mirrors the wave vertically and fills upwards instead of downwards, which is
 * what the bottom divider needs: solid green above, page background below.
 */
function buildSvg (pts, { width, height, tension }, flip) {
	const p = flip ? pts.map((q) => ({ x: q.x, y: height - q.y })) : pts;
	/*
	 * The fill runs 1px past the viewBox, which clips it exactly on the boundary. That
	 * avoids an antialiased half-row along the flat edge when the tile is scaled to a
	 * fractional height, which would show as a hairline against the adjacent section.
	 */
	const close = flip ? ` L${width},-1 L0,-1 Z` : ` L${width},${height + 1} L0,${height + 1} Z`;
	const d = wavePath(p, width, tension) + close;
	return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" `
		+ `viewBox="0 0 ${width} ${height}" preserveAspectRatio="none">`
		+ `<path d="${d}" fill="${WAVE_FILL}"/></svg>`;
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
	const topUri = toDataUri(buildSvg(pts, cfg, false));
	const botUri = toDataUri(buildSvg(pts, cfg, true));

	document.querySelectorAll(".stage").forEach((stage) => {
		stage.style.setProperty("--tile-w", cfg.width + "px");
		stage.querySelector(".wave--top").style.backgroundImage = `url("${topUri}")`;
		stage.querySelector(".wave--bottom").style.backgroundImage = `url("${botUri}")`;
		/* Production shrinks the divider below 960px, so the phone column must too. */
		const height = stage.id === "stage-phone" ? cfg.height * PHONE_HEIGHT_FACTOR : cfg.height;
		stage.querySelectorAll(".wave").forEach((w) => {
			w.style.height = Math.round(height) + "px";
		});
	});

	reportSeam(pts, cfg);

	/*
	 * Data only — never behaviour. `background-repeat`, `background-size` and the
	 * `animation` live permanently in style.css; emitting them here invites pasting
	 * over them, which silently freezes the wave.
	 */
	lastCss = `/* ============================================================================\n`
		+ ` * GENERATED by playground/wave_divider_playground.html\n`
		+ ` * Replace this whole block and nothing else. It carries data only — the\n`
		+ ` * animation and tiling behaviour above must survive a regeneration.\n`
		+ ` * Seed ${cfg.seed}, ${cfg.nodes} nodes, amplitude ${cfg.amplitude}.\n`
		+ ` * ========================================================================= */\n`
		+ `.wave-divider {\n`
		+ `\t--wave-tile: ${cfg.width}px;\n`
		+ `\t--wave-h: ${cfg.height}px;\n`
		+ `\t--wave-speed: ${cfg.speed}s;\n`
		+ `}\n\n`
		+ `.wave-divider--top {\n\tbackground-image: url("${topUri}");\n}\n\n`
		+ `.wave-divider--bottom {\n\tbackground-image: url("${botUri}");\n}\n`
		+ `/* === END GENERATED === */\n`;
	document.getElementById("output").value = lastCss;
}

/*
 * Surfaces the two things that actually make a tile unusable: a seam that does not
 * line up, or crests clipped by the tile height.
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

function copyCode () {
	const btn = document.getElementById("copy-btn");
	navigator.clipboard.writeText(lastCss).then(() => {
		btn.innerText = "Copiato";
		setTimeout(() => {
			btn.innerText = "Copia Codice CSS";
		}, 2500);
	}).catch(() => {
		const out = document.getElementById("output");
		out.select();
		alert("Copia fallita, seleziona il codice dal riquadro qui sotto.");
	});
}

window.onload = () => {
	updateUI();
	applySpeed();
	generate();
};
