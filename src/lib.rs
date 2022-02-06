mod backends;
mod common;

pub use backends::*;
pub use common::*;
use winit::{event_loop::EventLoopWindowTarget, window::{Window, WindowBuilder, WindowId}};

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
    fn add_window<T: 'static>(&mut self, window_builder: WindowBuilder, event_loop: &EventLoopWindowTarget<T>) -> Window;
    fn remove_window(&mut self, window: &Window);
    /// Load a Texture in the GPU. The texture data must be RGBA, and therefore need have a length
    /// of width * height * 4. if linear_filter is true, the texture will be sampled with linear
    /// filter applied.  Pixel art don't use linear filter.
    fn new_texture(&mut self, width: u32, height: u32, data: &[u8], linear_filter: bool) -> u32;
    fn update_texture(&mut self, texture: u32, data: &[u8], sub_rect: Option<[u32; 4]>);
    fn resize_texture(&mut self, width: u32, height: u32, texture: u32, data: &[u8]);
    fn render<'a>(&'a mut self, window: WindowId) -> Box<dyn Renderer + 'a>;
    fn resize(&mut self, window: WindowId, width: u32, height: u32);
}
impl Renderer for () {
    fn clear_screen(&mut self, _: &[f32; 4]) -> &mut dyn Renderer {
        self
    }
    fn draw_sprites(&mut self, _: &mut Camera, _: &[SpriteInstance]) -> &mut dyn Renderer {
        self
    }
    fn finish(&mut self) {}
}

impl SpriteRender for () {
    fn add_window<T: 'static>(&mut self, window_builder: WindowBuilder, event_loop: &EventLoopWindowTarget<T>) -> Window {
        window_builder.build(event_loop).unwrap()
    }
    fn remove_window(&mut self, _window: &Window) {}

    fn new_texture(&mut self, _: u32, _: u32, _: &[u8], _: bool) -> u32 {
        0
    }
    fn update_texture(&mut self, _: u32, _: &[u8], _: Option<[u32; 4]>) {}
    fn resize_texture(&mut self, _: u32, _: u32, _: u32, _: &[u8]) {}

    fn render<'a>(&'a mut self, _window: WindowId) -> Box<dyn Renderer + 'a> {
        Box::new(())
    }

    fn resize(&mut self, _window: WindowId, _width: u32, _height: u32) {}
}
