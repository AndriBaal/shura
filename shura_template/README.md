# shura template

## Run:

### Native:
```
cargo run --release
```

### Android:
When compiling for android make sure the following criteria are met:
- lib configuration in cargo.toml is not commented out
- The android sdk and ndk path are set
- [cargo-apk](https://github.com/rust-mobile/cargo-apk) is installed.

There are scripts in the scripts folder that set up the android development environment for ubuntu or arch
```
cargo apk run --lib --release
```

### WASM:
Make sure that [wasm-server-runner](https://github.com/jakobhellermann/wasm-server-runner) is installed and up to date.

```
cargo build --release --target wasm32-unknown-unknown

wasm-server-runner ./target/wasm32-unknown-unknown/release/shura_template.wasm
```
