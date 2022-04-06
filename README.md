# Sprite Render

A rust crate for rendering textured rects, with multiple backends (only two flavors of OpenGL for now).

## Backends
- [x] (), a noop implementation (I should not have implemented that to the unit type...)
- [x] OpenGL 3.2
- [x] WebGl
- [ ] OpenGLES (for Android support)
- [ ] Directx11? Directx12? (I have windows at least)
- [ ] Vulkan? (My only device with vulkan support is a Android)
- [ ] Metal? (I don't have a Apple device)
- [ ] Wgpu? (I have write this to avoid the bloat of gfx-hal)

# Run a example

You need to enable the feature of a backend to run a example:

 ```shell
 cargo run --example main --features=opengl
 ```

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
