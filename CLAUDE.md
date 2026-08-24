# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run       # dev server on 127.0.0.1:3000
cargo build
cargo test
cargo clippy
cargo fmt       # rustfmt.toml enforces hard_tabs = true
cargo check
```

## Architecture

Single Rust monolith for **Shine Parkour ASD** (Ravenna, Italy). Built with **Axum** + **Tokio**, **Askama** for
compile-time templating, and an in-process **Typst** compiler for PDF generation.

Only the public-facing "showcase" area is implemented. `handlers/admin.rs` and `handlers/instructors.rs` are empty
stubs. `db/models.rs` is also a stub — SQLite (via SQLx) and Hetzner S3 are planned but not yet in `Cargo.toml`.

### Request flow

```
main.rs (Router) → handlers/showcase.rs → HtmlTemplate<T> (render.rs) → Askama → Axum response
                                         → pdf/membership_2026_27/generator.rs → Typst → PDF bytes
```

Static files under `static/` are served by `tower-http::ServeDir` at `/static`.

### PDF generation (the most complex part)

Located in `src/pdf/membership_2026_27/`:

- `MembershipForm` in `templates.rs` is both a `serde::Deserialize` (Axum reads it from the POST body) and an
  `askama::Template` (renders the `.typ` Typst source with form data interpolated).
- `TypstCompiler` in `generator.rs` is a **`OnceLock` singleton** — it loads all system fonts via `fontdb` once per
  process lifetime (expensive; intentionally amortized).
- `InMemoryWorld` implements `typst::World`. It serves the rendered Typst markup as the `main` source and the two
  decoded signature images (base64 canvas data URLs from the browser) as virtual files `signature.png` /
  `signature2.png` — no disk I/O.
- Typst compilation runs inside `tokio::task::spawn_blocking` because it is CPU-bound.

### Askama dual-use (HTML + Typst)

Askama renders both HTML pages and the Typst markup template (`templates/pdf/membership_2026_27.typ`). The `.typ`
template uses `escape = "none"` so Typst `#` syntax passes through unmodified alongside `{{ variable }}` interpolations.

### Signature capture

The membership form captures two canvas signatures via JS. On submitting, each canvas is serialized to a PNG data URL
(`canvas.toDataURL()`) placed in a hidden `<input>`. The server strips the `data:image/png;base64,` prefix and
base64-decodes the rest before passing the bytes to `InMemoryWorld`.

### Template / asset layout

```
templates/
  base.html                        # base layout (Rubik font, htmx, style.css)
  showcase/                        # HTML pages
  pdf/membership_2026_27.typ       # Typst template for PDF
static/                            # served at /static
  style.css, htmx.min.js, logos/
```

HTMX is included but not yet actively used (no `hx-*` attributes in current templates).

## Language

The website UI and all user-facing content are in Italian. All code, variable names, comments, and commit messages must
be written in English.

## Configuration

None currently — server address (`127.0.0.1:3000`) and all paths are hard-coded in `src/main.rs`. No `.env`, no config
struct. When SQLite/S3 are added, they will need environment-based configuration.
