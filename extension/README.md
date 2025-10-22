Simple chrome extension to highligh and fix Greek typos based on `grs`.

To build/update the wasm, you will need `wasm-pack`:

```
cargo install wasm-pack
```

Then, at the root: 

```
wasm-pack build crates/grs_wasm --target web --out-dir "../../extension/pkg"
```
