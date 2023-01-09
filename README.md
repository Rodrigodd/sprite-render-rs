# Sprite Render

A rust crate for rendering textured rects, with multiple backends (only two flavors of OpenGL for now).

## Backends
- [x] A noop implementation
- [x] OpenGL >2.0 (Including ES)
- [x] WebGl
- [ ] Directx11? Directx12? (I have windows at least)
- [ ] Vulkan? (My only device with Vulkan support is an Android)
- [ ] Metal? (I don't have an Apple device)
- [ ] Wgpu? (I have written this to avoid the bloat of gfx-hal)

# Run an example

You need to enable the feature of a backend to run an example:

 ```shell
 cargo run --example main --features=opengl
 ```

# Run example on Web

```shell
cargo build --example main --target=wasm32-unknown-unknown --features=webgl
wasm-bindgen ./target/wasm32-unknown-unknown/debug/examples/main.wasm --target web --no-typescript --out-dir ./dist
wasm-opt ./dist/main_bg.wasm -o ./dist/main_bg.wasm -O # (optional)

cargo build --example main --target=wasm32-unknown-unknown --features=webgl && wasm-bindgen ./target/wasm32-unknown-unknown/debug/examples/main.wasm --target web --no-typescript --out-dir ./dist && wasm-opt ./dist/main_bg.wasm -o ./dist/main_bg.wasm -O # (optional)
```

# Run example on Android

This library makes use of the ndk-rs crates, refer to that repo for more documentation.

Make an example compile to a library:

```
// add to Cargo.toml
[[example]]
name = "main"
crate-type = ["cdylib"]
path = "examples/main.rs"
``` 

And then run the example by executing `cargo apk run --example main --features=opengl`. 
Use `adb logcat sprite-render:I *:S RustStdoutStderr:D` to get the logs.


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
