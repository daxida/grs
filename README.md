Greek rule-based spell checker.

Contains both a binary and a library. To run the binary:
```
cargo install --git https://github.com/daxida/grs
grs --help
```

For testing, a playground can be found [here](https://daxida.github.io/grs/).

The design is inspired (yet much simpler) by [ruff](https://github.com/astral-sh/ruff) and [spaCy](https://github.com/explosion/spaCy), and implements ideas of [grac](https://github.com/daxida/grac) and [greek-double-accents](https://github.com/daxida/greek-double-accents).

There is no stable API at the moment.

