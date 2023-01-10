mod backends;
mod common;

pub use backends::*;
pub use common::*;
use winit::window::{Window, WindowId};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, Default)]
pub struct TextureId(pub u32);
impl TextureId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}
impl std::fmt::Display for TextureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Error generate in [SpriteRender::new_texture].
#[derive(Debug)]
pub enum TextureError {
    /// The length of `data` does not match the expected from its width, height and `TextureFormat`.
    InvalidLength,
    /// The underline Renderer Context does not exist.
    RendererContextDontExist,
}

/// The format representation used by `data`.
pub enum TextureFormat {
    /// The RGBA8888 format.
    ///
    /// Each pixel of the texture is represented by 4 bytes, each one representing the channels Red,
    /// Green, Blue and Alpha, in that order. The colors are in the sRGB color space.
    ///
    /// The total size of `data` in bytes must be `width * height * 4`.
    Rgba8888,
}

/// The type of interpolation used when sampling the texture.
pub enum TextureFilter {
    /// Use the nearest sample.
    ///
    /// This make the texture look pixelated.
    Nearest,
    /// Interpolate linear between nearests sample.
    Linear,
}

/// A Texture to be loaded in [SpriteRender].
pub struct Texture<'a> {
    id: TextureId,
    width: u32,
    height: u32,
    format: TextureFormat,
    filter: TextureFilter,
    data: Option<&'a [u8]>,
}
impl<'a> Texture<'a> {
    /// Creates a new Texture, with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            id: TextureId(u32::max_value()),
            width,
            height,
            format: TextureFormat::Rgba8888,
            filter: TextureFilter::Linear,
            data: None,
        }
    }

    /// Sets a stable id for the texture.
    ///
    /// Used for replacing a existing texture or recreating it on context loss.
    pub fn id(mut self, id: TextureId) -> Self {
        self.id = id;
        self
    }

    /// Set the `TextureFormat` of `data`.
    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the `TexureFilter` when sampling.
    pub fn filter(mut self, filter: TextureFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the `data` of the Texture.
    ///
    /// If this method is not called, the texture will contain undefined data.
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.data = Some(data);
        self
    }

    /// Create this texture in the given [SpriteRender].
    ///
    /// Same as calling `sprite_render.new_texture(self)`.
    pub fn create(self, sprite_render: &mut dyn SpriteRender) -> Result<TextureId, TextureError> {
        sprite_render.new_texture(self)
    }
}

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
    /// Load a Texture in the GPU.
    ///
    /// The texture data must be RGBA, and therefore need have a length
    /// of width * height * 4. if linear_filter is true, the texture will be sampled with linear
    /// filter applied.  Pixel art don't use linear filter.
    fn new_texture(&mut self, texture: Texture) -> Result<TextureId, TextureError>;
    fn update_texture(
        &mut self,
        texture: TextureId,
        data: Option<&[u8]>,
        sub_rect: Option<[u32; 4]>,
    ) -> Result<(), TextureError>;
    fn render<'a>(&'a mut self, window: WindowId) -> Box<dyn Renderer + 'a>;
    fn resize(&mut self, window: WindowId, width: u32, height: u32);

    /// Resume the given window.
    ///
    /// Only used on Android. Allows recreating the Rendering context when it is lost.
    fn resume(&mut self, window: &Window);

    /// Suspends the rendering.
    ///
    /// Deletes all Rendering resources.
    fn suspend(&mut self);
}

/// A implementation of SpriteRender that does nothing.
///
/// None of its methods returns a Error.
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

    fn new_texture(&mut self, _: Texture) -> Result<TextureId, TextureError> {
        Ok(TextureId(0))
    }
    fn update_texture(
        &mut self,
        _: TextureId,
        _: Option<&[u8]>,
        _: Option<[u32; 4]>,
    ) -> Result<(), TextureError> {
        Ok(())
    }

    fn render<'a>(&'a mut self, _window: WindowId) -> Box<dyn Renderer + 'a> {
        Box::new(NoopRenderer)
    }

    fn resize(&mut self, _window: WindowId, _width: u32, _height: u32) {}

    fn resume(&mut self, _: &Window) {}

    fn suspend(&mut self) {}
}
