t path="texts/dump_lg.txt":
  cargo run -rq {{path}} --select ALL --statistics
  grs {{path}} --select ALL --statistics

lint:
  cargo clippy --all-targets --all-features -- -W clippy::nursery -W clippy::pedantic -A clippy::must_use_candidate -A clippy::module_name_repetitions -A clippy::cast_precision_loss

