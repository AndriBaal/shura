# shura

shura is a safe, fast and [cross-platform](#Cross-platform) 2D component-based game framework written in rust. shura helps you to manage big games with it's scene managing, group system and component system which is designed to easily share components between projects (therefore allowing for a ecosystem of components). shura has everything your 2D game needs such as physics, controller input, audio, easy extensible rendering, serializing, deserializing, text rendering, gui, animations and many more. See the [features](#Features) section for all of shura's features.

shura is currently in an early version and things might change in the future or don't work as intended. Feel free to create an issue if you encounter bugs, have feedback or have questions.

## Getting started

To see some examples head to the [/examples](https://github.com/AndriBaal/shura/tree/main/examples) directory or run `cargo run --release --example bunnymark`

Get started by copying the [template](https://github.com/AndriBaal/shura_template). The template includes the library, some introduction and has some additional configuration to make sure your game runs on all [supported platforms](#Cross-platform).

OR:

Add the following to your `Cargo.toml`:
```
[dependencies]
shura = "0.7.0"
```

A good way to learn shura is through the provided examples or through reading the code [documentation](https://docs.rs/shura).

## Features

- Managing multiple independent scenes.

- Easy to use component system with a group system to ensure fast manageable 2D games in massive levels

- EntityGroup system that acts like a chunk system to organize components and manage big worlds

- Support for postprocessing of your renders

- Easy to configure camera scaling, to ensure your game is responsive and works on all sort of screens

- Physics simulations directly implemented into the component system through [rapier](https://github.com/dimforge/rapier) (feature flag 'physics')

- Window Management with [winit](https://github.com/rust-windowing/winit)

- Cross-platform extendable rendering with [wgpu](https://github.com/gfx-rs/wgpu)

- Input handling for touch, mouse and keyboard and controller with [gilrs](https://gitlab.com/gilrs-project/gilrs) (feature flag 'gamepad')

- Text rendering inspired by [wgpu_text](https://github.com/Blatko1/wgpu-text) (feature flag 'text')

- Audio playback with [rodio](https://github.com/RustAudio/rodio) (feature flag 'audio')

- Easily create GUI's with [egui](https://github.com/emilk/egui) (feature flag 'gui')

- Serializing and serializing of scenes and groups with [serde](https://github.com/serde-rs/serde) and [bincode](https://github.com/bincode-org/bincode)

- Animations inspired by [bevy_tweening](https://github.com/djeedai/bevy_tweening) (feature flag 'animation')

- Logging on all Platforms with [env_logger](https://github.com/rust-cli/env_logger) and a modified verison of [wasm_logger](https://gitlab.com/limira-rs/wasm-logger) (feature flag 'log')

## Cross-platform

shura is currently only tested on Windows 10 / 11, Linux, Android and on the web with WASM. macOS and iOS are currently untested, but are likely to work.

The [template](https://github.com/AndriBaal/shura_template) uses [run-wasm](https://github.com/rukai/cargo-run-wasm) to run on the web and [cargo-apk](https://github.com/rust-mobile/cargo-apk) to run on android.

### Android

When compiling for android make sure the following is added in the Cargo.toml:

```
[lib]
crate-type = ["cdylib"]
path = "src/main.rs"
```
