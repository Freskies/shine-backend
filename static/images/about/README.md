# About — image dimensions

Images are displayed inside the collage with `object-fit: cover` (cropped to fill). Keep the subject centred — the crop
shifts slightly depending on the viewport.

Only the generated `about-N-<width>.webp` rungs live here; the high-quality masters sit in
`playground/about_masters/` and are never served. Edit a master, then regenerate the ladders:

```bash
python playground/generate_about_variants.py
```

| Master         | Collage slot        | Master size       | Rungs emitted            |
|----------------|---------------------|-------------------|--------------------------|
| `about-1.webp` | 47% wide × 43% tall | **800 × 620 px**  | 240, 360, 480, 640, 800  |
| `about-2.webp` | 52% wide × 40% tall | **900 × 580 px**  | 240, 360, 480, 640, 900  |
| `about-3.webp` | 77% wide × 50% tall | **1110 × 720 px** | 320, 480, 640, 880, 1110 |

The rungs are picked from the widest each slot is ever painted at, doubled for retina. If you change the collage
percentages in `index.css`, update both the ladders in the script and the `sizes` attributes in
`templates/showcase/index.html`.
