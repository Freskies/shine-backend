# static/images/

Photographic and illustrative content, one folder per page section. Keeps `static/` from turning into a flat pile of
assets.

```
images/
  about/     # the three-shot collage in the "Chi Siamo" section of the home page
```

Not in here: favicons and the web app manifest icons (they must sit at fixed paths, so they stay in `static/`), the
partner logos (`static/featured-in-logos/`), and anything embedded into the PDF by `src/pdf/` — that is loaded with
`include_bytes!` at compile time and moving it breaks the build.
