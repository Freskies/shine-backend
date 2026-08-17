#set page(paper: "a4", margin: (x: 2.2cm, y: 2cm), numbering: "1")
#set text(size: 10pt, lang: "en")
#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => block(above: 1.6em, below: 0.9em)[
  #text(size: 15pt, weight: "bold", fill: rgb("#0b4f8a"))[#it]
]
#show heading.where(level: 2): it => block(above: 1.25em, below: 0.65em)[
  #text(size: 11.5pt, weight: "bold")[#it]
]
#show raw.where(block: false): it => box(
  fill: rgb("#f3f4f6"),
  inset: (x: 3pt),
  outset: (y: 3pt),
  radius: 2pt,
)[#it]
#show raw.where(block: true): it => block(
  fill: rgb("#f8f9fa"),
  stroke: 0.5pt + rgb("#d8dde2"),
  radius: 3pt,
  inset: 9pt,
  width: 100%,
)[#it]

// Callout used for the handful of "this is the subtle bit" remarks.
#let note(body) = block(
  fill: rgb("#fff8e6"),
  stroke: (left: 2.5pt + rgb("#e9a23b")),
  inset: 9pt,
  width: 100%,
  radius: (right: 3pt),
)[#body]

// Boxed result, for the two formulas that are the point of their section.
#let result(body) = align(center)[
  #block(stroke: 0.6pt + rgb("#0b4f8a"), radius: 4pt, inset: 11pt)[#body]
]

#let th(..cells) = table.header(..cells.pos().map(c => text(weight: "bold")[#c]))

#align(center)[
  #text(size: 19pt, weight: "bold")[The maths behind the fluid box \ and the wave divider]
  #v(2mm)
  #text(size: 9.5pt, fill: rgb("#555"))[
    Shine Parkour ASD — reference for the generators in `playground/`
  ]
]

#v(6mm)

#table(
  columns: (auto, auto, auto, 1fr),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  th[Generator][Playground][Produces][Lands in],
  [Fluid box],
  [`fluid_box_playground.html`],
  [`clip-path: polygon(...)`],

  [`.blob-card` in `static/style.css`], [Wave divider], [`wave_divider_playground.html`], [SVG background tile],
  [`.wave-divider--top/--bottom`],
)

#v(3mm)

Both generators solve the same problem — *animate an organic shape so that it loops with no visible jump* — but from
opposite directions. The fluid box is a #emph[closed] curve animated by a single scalar; the wave divider is an
#emph[open] curve that must tile against a copy of itself. Every constraint in this document follows from that
distinction.

The shared rule: an animation loops seamlessly only when the generating function is #emph[periodic in the animated
  variable], and a tile repeats seamlessly only when the curve is #emph[periodic in space]. Each condition below is one
of those two in disguise.

= Fluid box

== The base shape: a superellipse

Each of the #box[$N_"res"$] polygon vertices starts on a superellipse (Lamé curve), sampled at

$ theta_i = (2 pi i) / N_"res", quad i = 0, 1, dots, N_"res" - 1 $

With $R = 50 - "pad"$ (in percent of the box) and $e = 2 \/ "roundness"$:

$
  x_i^0 = 50 + R abs(cos theta_i)^e op("sgn")(cos theta_i)
  quad quad
  y_i^0 = 50 + R abs(sin theta_i)^e op("sgn")(sin theta_i)
$

The sign factors are what extend the curve beyond the first quadrant: raising a negative cosine to a fractional power is
undefined, so the code takes $abs(cos theta)^e$ and re-applies the sign afterwards.

Eliminating $theta$ gives the implicit form. Since $abs(x^0 - 50) \/ R = abs(cos theta)^e$ and likewise for $y$, raising
both to the power $2 \/ e$ and adding the Pythagorean identity yields

$ abs((x^0 - 50) / R)^n + abs((y^0 - 50) / R)^n = 1, quad n = 2/e = "roundness" $

So the #emph[Forma Base] slider #emph[is] the superellipse exponent $n$:

#table(
  columns: (auto, 1fr),
  stroke: none,
  inset: (x: 0pt, y: 3pt),
  [$n = 2$], [circle, or an ellipse in a non-square box],
  [$n = 4$], [squircle],
  [$n -> infinity$], [square],
)

Its range of 2–6 therefore spans circle to rounded rectangle, matching the slider label.

== The animated displacement

Every vertex is then pushed along the #emph[radial] direction $(cos theta_i, sin theta_i)$ by a time-varying scalar
$S_i (a)$:

$ vec(x_i (a), y_i (a)) = vec(x_i^0, y_i^0) + "amp" dot S_i (a) vec(cos theta_i, sin theta_i) $

#note[
  *Subtlety.* $(cos theta, sin theta)$ is the radial direction of the #emph[unit circle], not the outward normal of the
  superellipse. The two coincide only at $n = 2$. For larger $n$ the wave slides slightly tangentially near the corners
  — visually harmless at these amplitudes, and the reason the code multiplies one scalar into both coordinates instead
  of computing a true normal.
]

== Two harmonics

$S_i (a)$ is the sum of two waves, one sine and one cosine, with independent frequencies and phases:

$
  S_i (a) = 0.6 sin(f_1^t a + f_1^s theta_i^circle.small + phi_1)
  + 0.4 cos(f_2^t a + f_2^s theta_i^circle.small + phi_2)
$

where $a$ is the animated angle (the CSS `--a`), the $f^t$ are #emph[temporal] frequencies, the $f^s$ are #emph[spatial]
frequencies and the $phi$ are random phase offsets. `scrambleWaves()` draws

$
  f_1^t, f_2^t in {1, 2}, quad f_1^s in {1, 2, 3}, quad f_2^s in {2, 3, 4, 5},
  quad phi in [0degree, 360degree)
$

The weights $0.6 + 0.4 = 1$ are a deliberate normalisation: they guarantee $abs(S_i (a)) <= 1$, so peak displacement is
exactly `amp` and never more. Section 1.5 depends on it.

== Why every frequency must be an integer

This is the crux, and it is two separate conditions that happen to have the same cure.

*Temporal — the animation must loop.* The keyframes drive $a: 0degree -> 360degree$. For the shape at the end to equal
the shape at the start we need $S_i (a + 360degree) = S_i (a)$, that is

$ sin(f_1^t (a + 360degree) + dots) = sin(f_1^t a + f_1^t dot 360degree + dots) $

which equals the original #emph[if and only if] $f_1^t in ZZ$. A fractional temporal frequency makes the animation snap
back at the end of every cycle.

*Spatial — the polygon must close.* Vertices $i = N_"res" - 1$ and $i = 0$ are adjacent. Going once around,
$theta^circle.small$ advances by $360degree$, so the spatial phase advances by $f_1^s dot 360degree$. The displacement
matches across that seam #emph[if and only if] $f_1^s in ZZ$. A fractional spatial frequency leaves a visible kink at
$theta = 0$.

Both hold because all four frequencies are drawn with `Math.floor()`.

== Keeping the wave inside the box

`clip-path` clips: anything beyond 0–100% is silently cut, which reads as a flat edge. The shape's furthest reach is
$R + "amp" = 50 - "pad" + "amp"$, and it has to stay under 50%, hence

$ "amp" <= "pad" - 0.5 $

enforced as `Math.min(requestedAmp, pad - 0.5)`, keeping 0.5% of slack. This bound is only valid because $abs(S_i) <= 1$
(§1.3).

== The CSS mechanics

Three platform features carry the whole effect.

+ *`@property --a`* declared with `syntax: '<angle>'`. Without a registered type a custom property is an untyped token
  string and animates by discrete substitution. Typing it as an angle is what allows smooth interpolation — the design
  hinges on this one declaration.
+ *Trigonometric `calc()`.* `sin()` and `cos()` inside `calc()` are re-evaluated every frame against the current `--a`,
  so the browser recomputes all $N_"res"$ vertices. The JavaScript emits the formula once and then does nothing.
+ *`@supports (width: calc(sin(1deg) * 1px))`* in `static/style.css` feature-tests item 2 and falls back to a plain
  `border-radius` where unsupported.

A vertex is emitted literally as:

```css
calc(37.42% + (sin(var(--a) * 2 + 141.0deg) * 3.12%) - (cos(var(--a) * 1 + 88.0deg) * 1.44%))
```

`buildMathTerm()` emits the amplitude's magnitude and folds its sign into the `+` or `-` joining the terms, keeping the
generated `calc()` to plain additive syntax; algebraically it is just $A dot f(dot)$ with signed $A$. Terms whose
amplitude rounds below 0.01% are dropped, so vertices near the axes produce shorter expressions.

== Parameters

#table(
  columns: (auto, auto, 1fr),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  th[Slider][Symbol][Meaning], [Forma Base], [$n$],
  [Superellipse exponent: 2 = circle, 6 ≈ rounded rectangle], [Risoluzione], [$N_"res"$],
  [Vertex count; cost grows linearly], [Margine Interno], ["pad"],
  [Inset in percent; doubles as the amplitude ceiling], [Ampiezza Onde], ["amp"],
  [Peak radial displacement in percent], [Velocità], [—],
  [Duration of one $0degree -> 360degree$ sweep of `--a`],
)

= Wave divider

== Why a fixed tile and not a stretched SVG

The first implementation was a single SVG spanning the viewport with `preserveAspectRatio="none"`. That makes the
horizontal period proportional to the viewport:

$ lambda_"stretched" = W_"viewport" / k quad (k "crests in the artwork") $

so a 390 px phone squeezed the same $k$ crests into a third of the width: identical count, each three times narrower.
The wave felt different on every device.

The fix is to hold the period constant in pixels and let the #emph[count] vary instead:

#result[
  $
    lambda = W_"tile" / (N \/ 2) quad "(independent of viewport)", quad quad
    "waves visible" = W_"viewport" / W_"tile" dot N/2
  $
]

using $N \/ 2$ because one full wave spans two nodes, a crest and a trough. With the shipped defaults $W_"tile" = 1000$
and $N = 4$:

#table(
  columns: (auto, auto),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  th[Viewport][Waves visible], [390 px (phone)],
  [$approx 0.8$], [1400 px (desktop)],
  [$approx 2.8$],
)

Which is the stated goal: about one wave on a phone, a couple on desktop, at identical crest size.

== Node placement

$N$ nodes across a tile of width $W$ and height $H$, with $Delta = W \/ N$ and a deterministic PRNG (`mulberry32`)
seeded by the slider, drawing $r in [0, 1)$:

$
  x_i = i Delta + underbrace((2r - 1) dot "jitter" dot Delta dot 0.45, j_i", " j_0 = 0),
  quad quad
  y_i = H/2 + (2r - 1) A
$

Two details matter.

*$x_0$ is pinned to zero.* The tile must begin exactly on its left edge, or the repetition shows a gap.

*The factor 0.45 is what guarantees monotonicity.* The curve must never fold back on itself, so $x$ has to strictly
increase. Since $abs(j_i) <= 0.45 Delta$:

$ x_(i+1) - x_i = Delta + j_(i+1) - j_i >= Delta - 0.45 Delta - 0.45 Delta = 0.1 Delta > 0 $

and at the wrap-around, $W - x_(N-1) >= Delta - 0.45 Delta = 0.55 Delta > 0$. Any factor $>= 0.5$ would let nodes cross
and the wave self-intersect.

The irregularity — the "less mathematical" look — comes entirely from $j_i$ and the random $y_i$. At $"jitter" = 0$ the
nodes are evenly spaced and the wave tends to a plain sinusoid.

== Catmull-Rom to cubic Bézier

SVG paths speak cubic Béziers, while the interpolation used here is Catmull-Rom, which passes #emph[through] its control
points. Converting between them:

a Catmull-Rom tangent at $p_1$, given neighbours $p_0$ and $p_2$, is

$ m_1 = (p_2 - p_0) / 2 $

and a cubic Hermite segment $(p_1, p_2, m_1, m_2)$ becomes a Bézier with controls

$ c_1 = p_1 + m_1 / 3, quad quad c_2 = p_2 - m_2 / 3 $

Substituting one into the other gives the form used in the code, with $tau$ as the tension slider:

#result[
  $ c_1 = p_1 + tau (p_2 - p_0) / 6, quad quad c_2 = p_2 - tau (p_3 - p_1) / 6 $
]

The #strong[6 is $3 times 2$]: the 3 from the Hermite-to-Bézier conversion, the 2 from the Catmull-Rom tangent. It is
not a tuning constant.

$tau = 1$ is standard Catmull-Rom; $tau < 1$ tightens the curve towards straight lines, $tau > 1$ overshoots and can
form loops, which is why the slider stops at 1.6.

== The periodic wrap, and why the seam is exact

Neighbour lookup wraps around with a horizontal shift of one whole tile:

$ P(i) = (x_(i mod N) + floor(i \/ N) dot W, quad y_(i mod N)) $

so $P(-1)$ is the last node moved one tile left, and $P(N)$ is the first node moved one tile right. Two consequences
follow.

*$C^0$ — the endpoints coincide.* The final segment ends at $P(N) = (x_0 + W, y_0) = (W, y_0)$, the same height the path
started at.

*$C^1$ — the tangents coincide.* At the start $m_0 = 1/2 (P(1) - P(-1))$, at the end $m_N = 1/2 (P(N+1) - P(N-1))$.
Expanding both with the shift:

$
  m_0 = 1/2 vec(x_1 - x_(N-1) + W, y_1 - y_(N-1))
  = 1/2 vec((x_1 + W) - x_(N-1), y_1 - y_(N-1)) = m_N
$

Identical, for #emph[any] node set. The join is therefore smooth by construction, not by nudging numbers until it looks
right — verified to full float precision on the shipped tile in §2.8.

== Closing the path

The open wave becomes a fillable region by walking out to the far corners:

- *Top divider* (solid below, green section underneath): `… L W,H L 0,H Z`
- *Bottom divider* (solid above): mirror every node, $y_i arrow.r.bar H - y_i$, then
  `… L W,0 L 0,0 Z`

Mirroring instead of re-randomising keeps the two edges of the section visibly related.

== Amplitude ceiling

A Catmull-Rom curve overshoots its nodes, so the nodes themselves must not sit near the edge:

$ A = min(A_"slider", H/2 - 6) $

The 6 px is empirical headroom for that overshoot. The playground additionally warns when the realised extremes come
within 2 px of either edge.

== Tiling and the animation loop

```css
/* Behaviour — hand-written, must survive a regeneration. */
.wave-divider {
	width: 100%;
	height: calc(var(--wave-h) * 0.7);
	background-repeat: repeat-x;
	background-size: var(--wave-tile) 100%;
	background-position: 0 0;
	animation: wave-drift var(--wave-speed) linear infinite;
}
.wave-divider--bottom {
	animation-direction: reverse;
	animation-duration: calc(var(--wave-speed) * 1.3);
}
@keyframes wave-drift {
	from { background-position: 0 0; }
	to   { background-position: var(--wave-tile) 0; }
}
@media (min-width: 960px) {
	.wave-divider { height: var(--wave-h); }
}

/* Data — this is all the playground emits. */
.wave-divider {
	--wave-tile: 1000px;
	--wave-h: 110px;
	--wave-speed: 22s;
}
```

#note[
  *Keep data and behaviour apart.* The playground deliberately emits #emph[only] the three
  custom properties and the two `background-image` rules, wrapped in `GENERATED` markers.
  An earlier version also emitted a partial `.wave-divider` rule; pasting it over the
  hand-written one silently deleted `background-repeat`, `background-size` and `animation`,
  which froze the wave and left the stale tile winning the cascade. Data in, behaviour
  untouched.
]

Scrolling by exactly one tile width maps the pattern onto itself, so the frame at $t = T$ is pixel-identical to the one
at $t = 0$ and the loop is invisible. Any other distance would jump.

*Vertical behaviour.* The second value of `background-size` is `100%`, so the tile's intrinsic height $H$ is stretched
to the element height $h$. On-screen amplitude therefore scales as

$ A_"screen" = A dot h / H $

while the horizontal period is untouched. That is why the 65 px to 90 px breakpoint changes the divider's thickness
without disturbing the crest spacing of §2.1.

*Encoding.* The SVG is inlined as a `data:` URI. Only the characters that would terminate a CSS `url()` are
percent-encoded; spaces, commas, colons and slashes are restored afterwards so the stylesheet stays readable and
diffable. Note that `#ecfdf5` has to become `%23ecfdf5`, since a raw hash would start a URL fragment.

== Worked example: the shipped tile

The defaults $W = 1000$, $H = 90$, $N = 4$, $A = 17$, $"jitter" = 0.7$, $tau = 1$ and $"seed" = 55$ produce these nodes:

#table(
  columns: (auto, auto, auto),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  align: (center, right, right),
  th[$i$][$x_i$][$y_i$], [0], [0],
  [43.172937], [1], [308.844084],
  [52.048966], [2], [539.225813],
  [30.116432], [3], [706.066336],
  [45.108269],
)

First segment, with $p_0 = P(-1) = (706.066336 - 1000, 45.108269)$:

$
  c_1 = p_1 + (p_2 - p_0)/6
  = ((308.844084 + 293.933664)/6, quad 43.172937 + (52.048966 - 45.108269)/6)
  = (100.46, 44.33)
$

$
  c_2 = p_2 - (p_3 - p_1)/6
  = (308.844084 - 539.225813/6, quad 52.048966 - (30.116432 - 43.172937)/6)
  = (218.97, 54.23)
$

which matches the first curve command in `static/style.css`:

```
M0,43.17 C100.46,44.33 218.97,54.23 308.84,52.05 …
```

Seam figures for this tile: $y(0) = y(W) = 43.172936930321$ and $m_0 = m_N = (301.388874095283, 3.470348228933)$ — equal
as floating-point values, not merely close.

== Parameters

#table(
  columns: (auto, auto, 1fr),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  th[Slider][Symbol][Meaning], [Larghezza Onda], [$W$],
  [Tile width in px; sets the period. The viewport-independence knob], [Altezza], [$H$],
  [Tile height in px], [Nodi per Tile], [$N$],
  [Control points; $N \/ 2$ full waves per tile], [Ampiezza], [$A$],
  [Node deviation from the mid-line, clamped per §2.6], [Irregolarità], ["jitter"],
  [0 = evenly spaced (near-sinusoid), 1 = maximally staggered], [Morbidezza], [$tau$],
  [Catmull-Rom tension; above 1 risks loops], [Velocità], [$T$],
  [Duration of one full tile scroll], [Seed], [—],
  [PRNG seed; the same seed always rebuilds the same wave],
)

= Comparison

#table(
  columns: (auto, 1fr, 1fr),
  stroke: 0.4pt + rgb("#c8ced4"),
  inset: 6pt,
  th[][Fluid box][Wave divider], [Curve], [Closed, around a centre],
  [Open, horizontal], [Basis], [Superellipse + two harmonics],
  [Catmull-Rom through random nodes], [Randomness], [Frequencies and phases, per reload],
  [Node offsets, from a reproducible seed], [Animated by], [`--a`: $0degree -> 360degree$],
  [`background-position`: $0 -> W$], [Loops because], [Integer temporal frequency (§1.4)],
  [Scroll equals exactly one tile (§2.7)], [Seam closes because], [Integer spatial frequency (§1.4)],
  [Periodic Catmull-Rom wrap (§2.4)], [Evaluated by], [The CSS engine, every frame],
  [Once, baked into the SVG], [Responsive], [Percentages: scales with the box],
  [Fixed px: count varies, size does not],
)

The last two rows are the real trade-off. The fluid box recomputes $N_"res"$ vertices per frame — flexible, but it costs
CPU and only works where trigonometric `calc()` does. The wave divider is a static image that the compositor merely
slides, which is far cheaper, at the price of a regeneration pass whenever the shape changes.

= Regenerating

Open either playground straight in a browser — they are deliberately plain scripts, so `file://` works with no server —
move the sliders, then copy the generated CSS into `static/style.css`:

- *Fluid box*: replace the `clip-path` inside the `@supports` block of `.blob-card`.
- *Wave divider*: replace `--wave-tile`, `height`, and both `background-image` rules on `.wave-divider--top` and
  `.wave-divider--bottom`.

The wave divider's `seed` slider makes its output reproducible: recording the seed next to the generated CSS, as the
comment in `static/style.css` does, is enough to rebuild a wave exactly. The fluid box has no seed — its frequencies are
redrawn on every #emph[Genera Nuova Sinuosità], so the emitted `clip-path` is the only record of a given shape.

#note[
  Templates under `templates/` are compiled into the binary by Askama and need `cargo build` after every edit. Files
  under `static/` are plain assets: editing `static/style.css` takes effect on reload, with no rebuild.
]
