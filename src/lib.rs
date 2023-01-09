mod backends;
mod common;

pub use backends::*;
pub use common::*;
use winit::window::{Window, WindowId};

pub trait Renderer {
    fn clear_screen(&mut self, color: &[f32; 4]) -> &mut dyn Renderer;

    fn draw_sprites(
        &mut self,
        camera: &mut Camera,
        sprites: &[SpriteInstance],
    ) -> &mut dyn Renderer;

    fn finish(&mut self);
}

pub trait SpriteRender {
    fn add_window(&mut self, window: &Window);
    fn remove_window(&mut self, window_id: WindowId);
    /// Load a Texture in the GPU. The texture data must be RGBA, and therefore need have a length
    /// of width * height * 4. if linear_filter is true, the texture will be sampled with linear
    /// filter applied.  Pixel art don't use linear filter.
    fn new_texture(&mut self, width: u32, height: u32, data: &[u8], linear_filter: bool) -> u32;
    fn update_texture(&mut self, texture: u32, data: &[u8], sub_rect: Option<[u32; 4]>);
    fn resize_texture(&mut self, texture: u32, width: u32, height: u32, data: &[u8]);
    fn render<'a>(&'a mut self, window: WindowId) -> Box<dyn Renderer + 'a>;
    fn resize(&mut self, window: WindowId, width: u32, height: u32);
}

/// A implementation of SpriteRender that does nothing.
pub struct NoopSpriteRender;
/// A implementation of Renderer that does nothing.
struct NoopRenderer;

impl Renderer for NoopRenderer {
    fn clear_screen(&mut self, _: &[f32; 4]) -> &mut dyn Renderer {
        self
    }
    fn draw_sprites(&mut self, _: &mut Camera, _: &[SpriteInstance]) -> &mut dyn Renderer {
        self
    }
    fn finish(&mut self) {}
}

impl SpriteRender for NoopSpriteRender {
    fn add_window(&mut self, _window: &Window) {}
    fn remove_window(&mut self, _window_id: WindowId) {}

    fn new_texture(&mut self, _: u32, _: u32, _: &[u8], _: bool) -> u32 {
        0
    }
    fn update_texture(&mut self, _: u32, _: &[u8], _: Option<[u32; 4]>) {}
    fn resize_texture(&mut self, _: u32, _: u32, _: u32, _: &[u8]) {}

    fn render<'a>(&'a mut self, _window: WindowId) -> Box<dyn Renderer + 'a> {
        Box::new(NoopRenderer)
    }

    fn resize(&mut self, _window: WindowId, _width: u32, _height: u32) {}
}
