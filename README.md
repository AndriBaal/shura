# shura

shura is a safe, fast and [cross-platform](#Cross-platform) 2D component-based game framework written in rust. shura helps you to manage big games with a component system, scene managing and its group system. See the [features](#Features) section for all of shura's features.

shura is currently in an early beta version and many features most likely have some issues, so feedback is very welcome.

## Getting started

Get started by copying the [template](https://github.com/AndriBaal/shura/tree/main/examples/template). The template includes the library and has some additional config to make sure your game runs on all [supported platforms](#Cross-platform).

If you don't care for other platforms than PC you can just add the following to your `Cargo.toml`:

```
[dependencies]
shura = "*"
```

A good way to learn shura is through the provided examples or through reading the code [documentation](https://docs.rs/shura).

## Features

- Managing multiple independent scenes.

- Easy to use component system with a group system to ensure fast manageable 2D games in massive levels

- Group system that acts like a chunk system to organize components and manage big worlds

- Built in support for postprocessing of your renders

- Physics simulations directly implemented into the component system through [rapier](https://github.com/dimforge/rapier) (feature flag 'physics')

- Window Management with [winit](https://github.com/rust-windowing/winit)

- Cross-platform extendable rendering with [wgpu](https://github.com/gfx-rs/wgpu)

- Input handling for touch, mouse and keyboard and controller with [gilrs](https://gitlab.com/gilrs-project/gilrs) (feature flag 'gamepad')

- Text rendering with [wgpu_glyph](https://github.com/hecrj/wgpu_glyph) (feature flag 'text')

- Audio playback with [rodio](https://github.com/RustAudio/rodio) (feature flag 'audio')

- Easily create GUI's with [egui](https://github.com/emilk/egui) (feature flag 'gui')

## Future features:

- Tutorials and indepth documentation

- Lighting (with rapier2d)

- Debug grid and complete debug information

- Create models from rapier2d colliders / bodies

- Serialization

- Examples:

    - Computed Sprite

    - Groups

    - Postprocess (Improve)

## Safety:

The entire engine including the component system is completely without any unsafeties except some data conversion mandatory to pass data to the GPU.

## Cross-platform

shura is currently only tested on Windows 10 / 11, Linux, Android and on the web with WASM. macOS and iOS are currently untested, but are likely to work.

All examples are running on said platforms, but not every example has support for mobile.

The [template](https://github.com/AndriBaal/shura/tree/main/examples/template) uses [run-wasm](https://github.com/rukai/cargo-run-wasm) to run on the web and [cargo-apk](https://github.com/rust-mobile/cargo-apk) to run on android.

### Android

When compiling for android make sure the following is added in the Cargo.toml:

```
[lib]
crate-type = ["cdylib"]
path = "src/main.rs"
```

## Run Examples

Checkout some [cross-platform](#Cross-platform) examples in the [examples](https://github.com/AndriBaal/shura/tree/main/examples) directory.

Run a postprocessing example (Located in /examples/postprocessing):

Native:
```
cargo run --release
```

Android (See [Android](#Android)) :
```
cargo apk run --release
```

Browser (Make sure that hardware acceleration is enabled)
```
http://3.71.15.62
```
Or build it yourself:
```
cargo run-wasm --release --package bunnymark
```
