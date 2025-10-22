**G**reek **R**ule-based **S**pellchecker.

A playground can be found [here](https://daxida.github.io/grs/).

To install the binary:
```
cargo install --git https://github.com/daxida/grs
grs --help
```

A quick example:
```
echo "Oι μαγαζατορες. Να μην την δώσεις. Σε κανενανε!" > tmp.txt
grs check tmp.txt --select ALL

>>> MS : [*] Oι μαγαζατορες.
>>> RFN: [*] Να μην την δώσεις.
>>> MNA:     Σε κανενανε!
>>> Found 3 errors.
>>> [*] 2 fixable with the `--fix` option.
```

It also contains a library, used by the playground and the browser extension.

There is no stable API at the moment.

The design is inspired (yet much simpler) by [ruff](https://github.com/astral-sh/ruff) and [spaCy](https://github.com/explosion/spaCy), and implements ideas of [grac](https://github.com/daxida/grac) and [greek-double-accents](https://github.com/daxida/greek-double-accents).
