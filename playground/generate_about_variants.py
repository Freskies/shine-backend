#!/usr/bin/env python3
"""Regenerate the responsive WebP ladder for the "Chi Siamo" collage.

Masters live in `playground/about_masters/` and are never served. This script
downscales each one into the rungs listed in LADDERS and writes them to
`static/images/about/`, where `templates/showcase/index.html` picks them up
through `srcset`.

The rungs are chosen from the widest each slot is ever painted at (see the
`sizes` attributes in the template), doubled for retina. Run with:

    python playground/generate_about_variants.py
"""

import os
from PIL import Image

MASTERS = os.path.join(os.path.dirname(__file__), "about_masters")
OUT = os.path.join(os.path.dirname(__file__), "..", "static", "images", "about")

# One entry per collage shot: the widths to emit, narrowest first.
LADDERS = {
	"about-1": [240, 360, 480, 640, 800],
	"about-2": [240, 360, 480, 640, 900],
	"about-3": [320, 480, 640, 880, 1110],
}

# Decorative photos behind a clip-path, so a lower quality than the masters is
# invisible in place. `method=6` is the slowest, densest encoder setting.
QUALITY = 78
METHOD = 6


def main():
	for stem, widths in LADDERS.items():
		master = Image.open(os.path.join(MASTERS, f"{stem}.webp"))
		src_w, src_h = master.size

		for width in widths:
			if width > src_w:
				raise SystemExit(f"{stem}: rung {width}px exceeds master width {src_w}px")

			height = round(src_h * width / src_w)
			resized = master.resize((width, height), Image.LANCZOS)
			path = os.path.join(OUT, f"{stem}-{width}.webp")
			resized.save(path, "WEBP", quality=QUALITY, method=METHOD)
			print(f"{stem}-{width}.webp  {width}x{height}  {os.path.getsize(path) // 1024} KiB")


if __name__ == "__main__":
	main()
