# shura template

## Run:

### Native:
```
cargo run --release
```

### Android:
When compiling for android make sure the "desktop" configuration is commented out and the android configuration is not.

Make sure that [cargo-apk](https://github.com/rust-mobile/cargo-apk) is installed.
```
cargo apk run --release
```

### WASM:
```
cargo run-wasm --release --bin desktop
```