# The maths behind the fluid box and the wave divider

Reference for the two generators in this folder:

| Generator    | Playground                     | Consumes                    | Output lands in                                     |
|--------------|--------------------------------|-----------------------------|-----------------------------------------------------|
| Fluid box    | `fluid_box_playground.html`    | `clip-path: polygon(...)`   | `.blob-card` in `static/style.css`                  |
| Wave divider | `wave_divider_playground.html` | `background-image` SVG tile | `.wave-divider--top/--bottom` in `static/style.css` |

Both solve the same problem — **animate an organic shape so it loops with no visible jump** — but they solve it in
opposite ways. The fluid box is a *closed* curve animated by a scalar; the wave divider is an *open* curve that has to
tile against a copy of itself. Everything below follows from that distinction.

A shared rule of thumb: a loop is seamless only when the generating function is **periodic in the animated variable**,
and a tile is seamless only when the curve is **periodic in space**. Every constraint in this document is one of those
two in disguise.

---

## Part 1 — Fluid box

### 1.1 The base shape: a superellipse

Each of the `res` polygon vertices starts on a superellipse (Lamé curve), sampled at

$$\theta_i = \frac{2\pi i}{res}, \qquad i = 0, 1, \dots, res-1$$

With $R = 50 - \text{pad}$ (percent of the box) and $e = 2/\text{roundness}$:

$$x_i^{0} = 50 + R\,|\cos\theta_i|^{\,e}\operatorname{sgn} (\cos\theta_i)
\qquad y_i^{0} = 50 + R\,|\sin\theta_i|^{\,e}\operatorname{sgn} (\sin\theta_i)$$

The `sgn` factors are what extend the curve past the first quadrant: raising a negative cosine to a fractional power
would be undefined, so the code takes $|\cos|^e$ and re-applies the sign.

Eliminating $\theta$ gives the implicit form. Since $|x^0-50|/R = |\cos\theta|^e$ and likewise for $y$, raising both to
the power $2/e$ and summing the Pythagorean identity:

$$\left|\frac{x^0-50}{R}\right|^{n} + \left|\frac{y^0-50}{R}\right|^{n} = 1, \qquad n = \frac{2}{e} = \text{roundness}$$

So the `roundness` slider *is* the superellipse exponent $n$:

- $n = 2$ → circle (or ellipse in a non-square box)
- $n = 4$ → squircle
- $n \to \infty$ → square

The slider range 2–6 therefore spans circle → rounded rectangle, matching its label.

### 1.2 The animated displacement

Every vertex is then pushed along the **radial** direction $(\cos\theta_i, \sin\theta_i)$
by a time-varying scalar $S_i (a)$:

$$\begin{pmatrix} x_i (a) \\ y_i (a) \end{pmatrix} = \begin{pmatrix} x_i^{0} \\ y_i^{0} \end{pmatrix} + \text{amp}\,S_i (a)
\begin{pmatrix} \cos\theta_i \\ \sin\theta_i \end{pmatrix}$$

> **Subtlety.** $(\cos\theta,\sin\theta)$ is the radial direction of the *unit circle*,
> not the outward normal of the superellipse. The two coincide only when $n = 2$. For
> higher $n$ the wave slides slightly tangentially near the corners. It is visually
> harmless at these amplitudes, and it is why the code multiplies the same scalar into
> both coordinates instead of computing a real normal.

### 1.3 Two harmonics

$S_i (a)$ is the sum of two waves — one sine, one cosine — with independent frequencies and phases:

$$S_i (a) = 0.6\sin\!\big (f^t_1 a + f^s_1\theta_i^{\circ} + \varphi_1\big)

+ 0.4\cos\!\big (f^t_2 a + f^s_2\theta_i^{\circ} + \varphi_2\big)$$

where $a$ is the animated angle (the CSS `--a`), $f^t$ are **temporal** frequencies,
$f^s$ are **spatial** frequencies, and $\varphi$ are random phase offsets. `scrambleWaves()`
draws them as $f^t_{1,2} \in \{1,2\}$, $f^s_1 \in \{1,2,3\}$, $f^s_2 \in \{2,3,4,5\}$,
$\varphi \in [0^\circ, 360^\circ)$.

The weights $0.6 + 0.4 = 1$ are a deliberate normalisation: it makes
$|S_i (a)| \le 1$, so the peak displacement is exactly `amp` and never more. Section 1.5 depends on this.

### 1.4 Why every frequency must be an integer

This is the crux, and it is two separate conditions.

**Temporal — the animation must loop.** The keyframes drive $a: 0^\circ \to 360^\circ$. For the shape at the end to
equal the shape at the start we need
$S_i (a + 360^\circ) = S_i (a)$, i.e.

$$\sin\!\big (f^t_1 (a + 360^\circ) + \dots\big) = \sin\!\big (f^t_1 a + f^t_1\cdot 360^\circ + \dots\big)$$

which equals the original **iff $f^t_1 \in \mathbb{Z}$**. A fractional temporal frequency would make the animation snap
back at the end of every cycle.

**Spatial — the polygon must close.** Vertices $i = res-1$ and $i = 0$ are adjacent. Going once around, $\theta^\circ$
advances by $360^\circ$, so the spatial phase advances by
$f^s_1 \cdot 360^\circ$. The displacement matches across that seam **iff
$f^s_1 \in \mathbb{Z}$**. A fractional spatial frequency would leave a visible kink at
$\theta = 0$.

Both conditions hold because the code draws all four frequencies with `Math.floor()`.

### 1.5 Keeping the wave inside the box

`clip-path` clips: anything outside 0–100% is silently cut, which reads as a flat edge. The shape's furthest reach
is $R + \text{amp} = 50 - \text{pad} + \text{amp}$, and it must stay below 50%. Hence

$$\text{amp} \le \text{pad} - 0.5$$

enforced as `Math.min(requestedAmp, pad - 0.5)`, with 0.5% of slack. This bound is only valid because $|S_i| \le 1$
(§1.3).

### 1.6 The CSS mechanics

Three features carry this:

1. **`@property --a`** with `syntax: '<angle>'`. Without a registered type a custom property is an untyped token string
   and animates by discrete substitution. Declaring it an `<angle>` is what lets the browser interpolate it smoothly —
   the whole design hinges on this one declaration.
2. **Trigonometric `calc()`** — `sin()` / `cos()` inside `calc()` are evaluated per frame against the current `--a`. So
   all $res$ vertices are recomputed by the engine; the JS only emits the formula once.
3. **`@supports (width: calc(sin(1deg) * 1px))`** in `style.css` feature-tests item 2 and falls back to a plain
   `border-radius` when unsupported.

A vertex is emitted literally as

```css
calc(37.42% + (sin(var(--a) * 2 + 141.0deg) * 3.12%) - (cos(var(--a) * 1 + 88.0deg) * 1.44%))
```

`buildMathTerm()` emits the amplitude's magnitude and folds its sign into the `+`/`-` operator joining the terms, which
keeps the generated `calc()` to plain additive syntax. The result is algebraically just $A\cdot f (\cdot)$ with signed
$A$. Terms whose amplitude rounds below 0.01% are dropped, so vertices near the axes emit shorter expressions.

### 1.7 Parameters

| Slider          | Symbol | Meaning                                                  |
|-----------------|--------|----------------------------------------------------------|
| Forma Base      | $n$    | Superellipse exponent: 2 = circle, 6 ≈ rounded rectangle |
| Risoluzione     | $res$  | Polygon vertex count; cost is linear                     |
| Margine Interno | pad    | Inset in %; also the amplitude ceiling                   |
| Ampiezza Onde   | amp    | Peak radial displacement in %                            |
| Velocità        | —      | Duration of one $0^\circ \to 360^\circ$ sweep of `--a`   |

---

## Part 2 — Wave divider

### 2.1 Why a fixed tile and not a stretched SVG

The first version was one SVG spanning the viewport with `preserveAspectRatio="none"`. That makes the horizontal period
proportional to the viewport:

$$\lambda_{\text{stretched}} = \frac{W_{\text{viewport}}}{k} \quad (k \text{ crests in the artwork})$$

so a 390 px phone compressed the same crests into a third of the width — same count, each three times narrower. The wave
felt different on every device.

The fix is to make the period a **constant in px** and let the *count* vary:

$$\lambda = \frac{W_{\text{tile}}}{N/2} \quad\text{ (independent of viewport)}, \qquad \text{waves visible} = \frac{W_{\text{viewport}}}{W_{\text{tile}}}\cdot\frac{N}{2}$$

using $N/2$ because one full wave spans two nodes (a crest and a trough). With the shipped
defaults $W_{\text{tile}} = 1000$, $N = 4$:

| Viewport          | Waves visible |
|-------------------|---------------|
| 390 px (phone)    | $\approx 0.8$ |
| 1400 px (desktop) | $\approx 2.8$ |

Which is the stated goal: roughly one wave on a phone, a couple on desktop, at identical crest size.

### 2.2 Node placement

$N$ nodes over a tile of width $W$ and height $H$, with $\Delta = W/N$ and a deterministic PRNG (`mulberry32`) seeded by
the `seed` slider, drawing $r \in [0,1)$:

$$x_i = i\Delta + \underbrace{ (2r-1)\cdot\text{jitter}\cdot\Delta\cdot 0.45}_{j_i,\ \ j_0 = 0}, \qquad y_i = \frac{H}{2} + (2r-1)\,A$$

Two details matter.

**$x_0$ is pinned to 0.** The tile must begin exactly on its left edge, otherwise the repeat would show a gap.

**The magic 0.45 guarantees monotonicity.** The curve must never fold back on itself, so
$x$ must strictly increase. Since $|j_i| \le 0.45\Delta$:

$$x_{i+1} - x_i = \Delta + j_{i+1} - j_i \ \ge\ \Delta - 0.45\Delta - 0.45\Delta = 0.1\Delta > 0$$

and at the wrap-around, $W - x_{N-1} \ge \Delta - 0.45\Delta = 0.55\Delta > 0$. Any factor
$\ge 0.5$ would allow nodes to cross and the wave to self-intersect.

Irregularity — the "less mathematical" look — comes entirely from $j_i$ and the random
$y_i$. At `jitter = 0`, nodes are evenly spaced and the wave approaches a plain sinusoid.

### 2.3 Catmull-Rom → cubic Bézier

SVG paths speak cubic Béziers; the interpolation is Catmull-Rom, which passes *through*
its control points. Converting one to the other:

A Catmull-Rom tangent at $p_1$, given neighbours $p_0$ and $p_2$, is

$$m_1 = \frac{p_2 - p_0}{2}$$

A cubic Hermite segment $(p_1, p_2, m_1, m_2)$ becomes a Bézier with controls

$$c_1 = p_1 + \frac{m_1}{3}, \qquad c_2 = p_2 - \frac{m_2}{3}$$

Substituting gives the form in the code, where $\tau$ is the tension slider:

$$\boxed{\;c_1 = p_1 + \tau\frac{p_2 - p_0}{6}, \qquad c_2 = p_2 - \tau\frac{p_3 - p_1}{6}\;}$$

The **6 is $3 \times 2$**: the 3 from the Hermite→Bézier conversion, the 2 from the Catmull-Rom tangent. It is not a
tuning constant.

$\tau = 1$ is standard Catmull-Rom. $\tau < 1$ tightens the curve toward straight lines;
$\tau > 1$ overshoots and can form loops, which is why the slider is capped at 1.6.

### 2.4 The periodic wrap, and why the seam is exact

Neighbour lookup wraps with a horizontal shift of one tile:

$$P (i) = \big (x_{i \bmod N} + \lfloor i/N \rfloor \cdot W,\ \ y_{i \bmod N}\big)$$

so $P (-1)$ is the last node moved one tile left, and $P (N)$ is the first node moved one tile right. Two consequences:

**$C^0$ — endpoints coincide.** The final segment ends at $P (N) = (x_0 + W, y_0) = (W, y_0)$, the same height the path
started at.

**$C^1$ — tangents coincide.** At the start, $m_0 = \tfrac{1}{2} (P (1) - P (-1))$; at the end,
$m_N = \tfrac{1}{2} (P (N{+}1) - P (N{-}1))$. Expanding with the shift:

$$m_0 = \frac{1}{2}\begin{pmatrix} x_1 - x_{N-1} + W \\ y_1 - y_{N-1}\end{pmatrix} = \frac{1}{2}\begin{pmatrix} (x_1 + W) - x_{N-1} \\ y_1 - y_{N-1}\end{pmatrix} = m_N$$

Identical, for any node set. So the join is smooth *by construction* — not by tuning numbers until it looks right.
Verified on the shipped tile (§2.8) to full float precision.

### 2.5 Closing the path

The open wave is turned into a fillable region by walking to the far corners:

- **Top divider** (solid below, green section underneath): `… L W,H L 0,H Z`
- **Bottom divider** (solid above): mirror every node, $y_i \mapsto H - y_i$, then
  `… L W,0 L 0,0 Z`

Mirroring rather than re-randomising keeps both edges of the section visibly related.

### 2.6 Amplitude ceiling

A Catmull-Rom curve overshoots its nodes, so the nodes alone must not sit near the edge:

$$A = \min\!\left (A_{\text{slider}},\ \frac{H}{2} - 6\right)$$

The 6 px is empirical headroom for overshoot. The playground additionally warns when the realised extremes come within 2
px of either edge.

### 2.7 Tiling and the animation loop

```css
.wave-divider {
	--wave-tile: 1000px;
	background-repeat: repeat-x;
	background-size: var(--wave-tile) 100%;
	animation: wave-drift 22s linear infinite;
}

@keyframes wave-drift {
	from {
		background-position: 0 0;
	}
	to {
		background-position: var(--wave-tile) 0;
	}
}
```

Scrolling by exactly one tile width maps the pattern onto itself, so frame $t = T$ is pixel-identical to $t = 0$ and the
loop is invisible. Scrolling by any other distance would jump.

**Vertical behaviour.** `background-size`'s second value is `100%`, so the tile's intrinsic height $H$ is stretched to
the element height $h$. On-screen amplitude scales as

$$A_{\text{screen}} = A \cdot \frac{h}{H}$$

while the horizontal period is untouched. This is why the 65 px → 90 px breakpoint changes the divider's thickness
without disturbing the crest spacing of §2.1.

**Encoding.** The SVG is inlined as a `data:` URI. Only characters that would terminate a CSS `url()` are
percent-encoded; spaces, commas, `:` and `/` are restored afterwards so the stylesheet stays readable and diffable. Note
`#ecfdf5` must become `%23ecfdf5` — a raw `#`
would start a URL fragment.

### 2.8 Worked example — the shipped tile

Defaults $W = 1000$, $H = 90$, $N = 4$, $A = 17$, jitter $= 0.7$, $\tau = 1$, seed $= 55$
produce the nodes

| $i$ | $x_i$      | $y_i$     |
|-----|------------|-----------|
| 0   | 0          | 43.172937 |
| 1   | 308.844084 | 52.048966 |
| 2   | 539.225813 | 30.116432 |
| 3   | 706.066336 | 45.108269 |

First segment, $p_0 = P (-1) = (706.066336 - 1000,\ 45.108269)$:

$$c_1 = p_1 + \frac{p_2 - p_0}{6} = \left (\frac{308.844084 + 293.933664}{6},\ \ 43.172937 + \frac{52.048966 - 45.108269}{6}\right)
= (100.46,\ 44.33)$$

$$c_2 = p_2 - \frac{p_3 - p_1}{6} = \left (308.844084 - \frac{539.225813}{6},\ \ 52.048966 - \frac{30.116432 - 43.172937}{6}\right)
= (218.97,\ 54.23)$$

matching the first curve command in `static/style.css`:

```
M0,43.17 C100.46,44.33 218.97,54.23 308.84,52.05 …
```

Seam figures for this tile: $y (0) = y (W) = 43.172936930321$ and
$m_0 = m_N = (301.388874095283,\ 3.470348228933)$ — equal as floats, not merely close.

### 2.9 Parameters

| Slider         | Symbol | Meaning                                                            |
|----------------|--------|--------------------------------------------------------------------|
| Larghezza Onda | $W$    | Tile width in px — sets the period; the viewport-independence knob |
| Altezza        | $H$    | Tile height in px                                                  |
| Nodi per Tile  | $N$    | Control points; $N/2$ full waves per tile                          |
| Ampiezza       | $A$    | Node deviation from the mid-line, clamped by §2.6                  |
| Irregolarità   | jitter | 0 = evenly spaced (near-sinusoid), 1 = maximally staggered         |
| Morbidezza     | $\tau$ | Catmull-Rom tension; > 1 risks loops                               |
| Velocità       | $T$    | Duration of one full tile scroll                                   |
| Seed           | —      | PRNG seed; same seed always rebuilds the same wave                 |

---

## Comparison

|                     | Fluid box                          | Wave divider                           |
|---------------------|------------------------------------|----------------------------------------|
| Curve               | Closed, around a centre            | Open, horizontal                       |
| Basis               | Superellipse + 2 harmonics         | Catmull-Rom through random nodes       |
| Randomness          | Frequencies and phases, per reload | Node offsets, from a reproducible seed |
| Animated by         | `--a`: $0^\circ \to 360^\circ$     | `background-position`: $0 \to W$       |
| Loops because       | Integer temporal frequency (§1.4)  | Scroll equals exactly one tile (§2.7)  |
| Seam closes because | Integer spatial frequency (§1.4)   | Periodic Catmull-Rom wrap (§2.4)       |
| Evaluated by        | The CSS engine, every frame        | Once, baked into the SVG               |
| Responsive          | Percentages: scales with the box   | Fixed px: count varies, size does not  |

The last two rows are the real trade-off. The fluid box recomputes $res$ vertices per frame — flexible, but it costs CPU
and only works where trigonometric `calc()` does. The wave divider is a static image the compositor merely slides, which
is far cheaper, at the price of needing a regeneration pass whenever the shape changes.

---

## Regenerating

Open the playground directly in a browser — no server needed, they are plain scripts on purpose so `file://` works —
adjust the sliders, then copy the generated CSS into
`static/style.css`:

- **Fluid box** → replace the `clip-path` inside the `@supports` block of `.blob-card`
- **Wave divider** → replace `--wave-tile`, `height`, and both
  `.wave-divider--top/--bottom` `background-image` rules

The `seed` slider makes wave output reproducible: recording the seed alongside the generated CSS (as the comment in
`style.css` does) is enough to rebuild it exactly. The fluid box has no seed — its frequencies are redrawn on every
"Genera Nuova Sinuosità", so the emitted `clip-path` is the only record of a given shape.

> Templates are compiled into the binary by Askama, but these are static assets: editing
> `static/style.css` needs no rebuild, while editing anything under `templates/` does.
