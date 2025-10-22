# Build WASM pkg. Assumed to be runned at ROOT.
wasm outdir="extension":
  wasm-pack build crates/grs_wasm --target web --out-dir "../../{{outdir}}/pkg"

# Lint with clippy
clippy *args:
  cargo clippy --all-targets --all-features {{args}} -- -W clippy::nursery -W clippy::pedantic -A clippy::must_use_candidate -A clippy::module_name_repetitions -A clippy::cast_precision_loss

RULE := "ALL"

alias t := test

# Quick test - The first one is for warmup
test path="texts/dump_lg.txt":
  cargo run -rq check {{path}} --select {{RULE}} --statistics
  grs check {{path}} --select {{RULE}} --statistics
  cargo run -rq check {{path}} --select {{RULE}} --statistics

# Ripgrep relevant files for version
check-version:
  rg version \
    playground/package.json \
    extension/manifest.json \
    Cargo.toml crates/grs/Cargo.toml crates/grs_wasm/Cargo.toml
