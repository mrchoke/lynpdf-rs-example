# lynpdf-rs-example

Thai-first Rust web app that simulates a small checkout page and generates a Thai-language PDF receipt via lynpdf-rs.

## What it demonstrates

- Rust HTTP server with Axum
- Thai web UI form -> backend POST endpoint -> Thai PDF response
- Using `lynpdf-rs` from GitHub (before crates.io publish)

## Dependency source

This example points Cargo to GitHub:

```toml
lynpdf-rs = { git = "https://github.com/mrchoke/lynpdf-rs", branch = "main" }
```

If the repository is private, Cargo needs GitHub credentials. During local development you can temporarily switch to:

```toml
lynpdf-rs = { path = "../lynpdf-rs" }
```

## Run

```bash
cargo run
```

Open:

- http://127.0.0.1:3000

Fill the Thai UI form and submit to download a Thai receipt PDF.

## Font setup (recommended)

If your environment does not already have Thai-friendly fonts, set one of these:

- `LYNPDF_FONT_DIRS`
- `LYNPDF_RS_EXAMPLE_FONT_DIR`

Example:

```bash
export LYNPDF_FONT_DIRS="/path/to/fonts"
cargo run
```

If this repository is next to [lynpdf-rs](../lynpdf-rs), you can bootstrap local fonts with:

```bash
../lynpdf-rs/scripts/download-fonts.sh --dest ./fonts
export LYNPDF_FONT_DIRS="$(pwd)/fonts:$(pwd)/fonts/tlwg/otf"
cargo run
```

## Endpoints

- `GET /` -> checkout page
- `POST /receipt` -> returns `application/pdf`
