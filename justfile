t path="texts/dump_lg.txt":
  cargo run -rq {{path}} --select ALL --statistics
  grs {{path}} --select ALL --statistics

lint:
  cargo clippy --all --fix -- -Wclippy::all -Wclippy::pedantic -Wclippy::nursery
