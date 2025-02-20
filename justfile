IPATH := "texts/dump_large.txt"

t:
  cargo run -rq {{IPATH}} --select ALL --statistics
  grs {{IPATH}} --select ALL --statistics

lint:
  cargo clippy --all --fix -- -Wclippy::all -Wclippy::pedantic -Wclippy::nursery
