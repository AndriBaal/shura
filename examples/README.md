# shura template

## Run:

### Native:
```
cargo run --release --bin desktop
```

### Android:
When compiling for android make sure the "desktop" configuration is commented out and the android configuration is not.

Make sure that [cargo-apk](https://github.com/rust-mobile/cargo-apk) is installed.
```
cargo apk run --release
```

### WASM:
Make sure that [wasm-server-runner](https://github.com/jakobhellermann/wasm-server-runner) is installed.

```
cargo build --release --target wasm32-unknown-unknown --bin desktop

wasm-server-runner ./target/wasm32-unknown-unknown/release/desktop.wasm

```