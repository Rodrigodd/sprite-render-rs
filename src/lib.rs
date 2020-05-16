mod common;
mod backends;

pub use common::*;
pub use backends::*;

use winit::{
    window::Window,
    window::WindowBuilder,
    event_loop::EventLoop,
};

pub trait SpriteRender  {
    /// Load a Texture in the GPU. if linear_filter is true, the texture will be sampled with linear filter applied.
    /// Pixel art don't use linear filter.
    fn load_texture(&mut self, width: u32, height: u32, data: &[u8], linear_filter: bool) -> u32;

    fn draw(&mut self, camera: &mut Camera, sprites: &[SpriteInstance]);

    fn resize(&mut self, width: u32, height: u32);
}

pub struct EmptySpriteRender;
impl SpriteRender for EmptySpriteRender {
    fn load_texture(&mut self, _: u32, _: u32, _: &[u8], _: bool) -> u32 { 0 }

    fn draw(&mut self, _: &mut Camera, _: &[SpriteInstance]) {}

    fn resize(&mut self, _: u32, _: u32) {}
}


/// create a SpriteRender with use the default backend by system:
/// - wasm: WebGL
/// - Windows: OpenGL
/// - Linux: OpenGL
/// - MacOS: OpenGL
pub fn default_render<T>(wb: WindowBuilder, event_loop: &EventLoop<T>, vsync: bool) 
-> (Window, Box<dyn SpriteRender>) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "opengl")] {
            let (window, render) = GLSpriteRender::new(wb, event_loop, vsync);
            (window, Box::new(render))
        } else if #[cfg(all(target_arch = "wasm32", feature = "webgl"))] {
            let (window, render) = WebGLSpriteRender::new(wb, event_loop);
            let _ = vsync;
            (window, Box::new(render))
        } else {
            let _ = vsync;
            (wb.build(event_loop).unwrap(), Box::new(EmptySpriteRender))
        }
    }
}