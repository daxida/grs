# Build WASM pkg/ dir
# Assumed to be runned at ROOT, and outputs relative at ROOT in a pkg folder
wasm outdir:
  wasm-pack build crates/grs_wasm --target web --out-dir "../../{{outdir}}/pkg"

clippy *args:
  cargo clippy --all-targets --all-features {{args}} -- -W clippy::nursery -W clippy::pedantic -A clippy::must_use_candidate -A clippy::module_name_repetitions -A clippy::cast_precision_loss

RULE := "ALL"

# Quick test - The first one is for warmup
t path="texts/dump_lg.txt":
  cargo run -rq check {{path}} --select {{RULE}} --statistics
  grs check {{path}} --select {{RULE}} --statistics
  cargo run -rq check {{path}} --select {{RULE}} --statistics
