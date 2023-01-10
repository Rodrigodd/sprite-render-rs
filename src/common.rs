use std::f32::consts::PI;

use crate::TextureId;

#[derive(Clone, Debug)]
pub struct SpriteInstance {
    pub scale: [f32; 2],
    pub angle: f32,
    pub uv_rect: [f32; 4],
    pub color: [u8; 4],
    pub pos: [f32; 2],
    pub texture: TextureId,
}
impl Default for SpriteInstance {
    fn default() -> Self {
        Self {
            scale: [1.0; 2],
            angle: 0.0,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            color: [255; 4],
            pos: [0.0; 2],
            texture: TextureId::default(),
        }
    }
}
impl SpriteInstance {
    /// Create a new SpriteInstant with center in (x,y) and with the given width, height, texture and uv_rect.
    /// The default color is white ([255, 255, 255, 255]).
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture: TextureId,
        uv_rect: [f32; 4],
    ) -> Self {
        Self {
            scale: [width, height],
            angle: 0.0,
            uv_rect,
            color: [0xff; 4],
            pos: [x, y],
            texture,
        }
    }

    /// Create a new SpriteInstant with center in (x,y) and with the given height, texture and uv_rect.
    /// The width is calculated to keep the uv_rect proportion.
    /// The default color is white ([255, 255, 255, 255]).
    pub fn new_height_prop(
        x: f32,
        y: f32,
        height: f32,
        texture: TextureId,
        uv_rect: [f32; 4],
    ) -> Self {
        let width = height * uv_rect[2] / uv_rect[3];
        Self {
            scale: [width, height],
            angle: 0.0,
            uv_rect,
            color: [0xff; 4],
            pos: [x, y],
            texture,
        }
    }

    /// set the width and the height of the sprite.
    #[inline]
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.scale = [width, height];
    }

    /// set the height of the sprite, keeping the proportion width / height.
    #[inline]
    pub fn set_heigh_prop(&mut self, height: f32) {
        let width = height * self.get_width() / self.get_height();
        self.scale = [width, height];
    }

    /// get the width of the sprite.
    #[inline]
    pub fn get_width(&self) -> f32 {
        self.scale[0]
    }

    /// get the height of the sprite.
    #[inline]
    pub fn get_height(&self) -> f32 {
        self.scale[1]
    }

    /// set the angle of rotation, in counterclokwise radians.
    #[inline]
    pub fn set_angle(&mut self, angle: f32) {
        self.angle = angle;
    }

    /// set the angle of the sprite, in a functional way (get owership of the value, and return it modified).
    /// The angle of rotation is in counterclokwise radians.
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// set position of the sprite.
    #[inline]
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.pos = [x, y];
    }

    /// get the x axis of the position of the sprite.
    #[inline]
    pub fn get_x(&self) -> f32 {
        self.pos[0]
    }

    /// get the y axis of the position of the sprite.
    #[inline]
    pub fn get_y(&self) -> f32 {
        self.pos[1]
    }

    /// set the color of the sprite, in the range 0 to 255, in the RGBA format.
    ///
    /// if you want to write in hexadecimal, you can write `sprite.set_color(0xRRGGBBAAu32.to_be_bytes())`
    /// for example.
    #[inline]
    pub fn set_color(&mut self, color: [u8; 4]) {
        self.color = color;
    }

    /// set the color of the sprite, in a functional way (get owership of the value, and return it modified).
    /// The color is in the range 0 to 255, in the RBGA format.
    ///
    /// if you want to write in hexadecimal, you can write `sprite.with_color(0xRRGGBBAAu32.to_be_bytes())`
    /// for example.
    #[inline]
    pub fn with_color(mut self, color: [u8; 4]) -> Self {
        self.set_color(color);
        self
    }

    #[inline]
    pub fn get_uv_rect(&mut self) -> &[f32; 4] {
        &self.uv_rect
    }

    #[inline]
    pub fn set_uv_rect(&mut self, rect: [f32; 4]) {
        self.uv_rect = rect;
    }

    #[inline]
    pub fn with_uv_rect(mut self, rect: [f32; 4]) -> Self {
        self.uv_rect = rect;
        self
    }
}

/// The camera encapsulates the view matrix, providing methods to move,
/// rotate or scale the camera view.
pub struct Camera {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rotation: f32,

    screen_size: (u32, u32),
    view_matrix: [f32; 9],
    dirty: bool,
}
impl Camera {
    /// Create a new camera with center in (0.0, 0.0) position, with zero rotation, and with the
    /// give height. The width is calculated to keep the screen_size proportion.
    pub fn new(screen_width: u32, screen_height: u32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: height * screen_width as f32 / screen_height as f32,
            height,
            rotation: 0.0,
            screen_size: (screen_width, screen_height),
            view_matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            dirty: true,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn view(&mut self) -> &[f32; 9] {
        if self.dirty {
            self.dirty = false;
            let w = 2.0 / self.width;
            let h = 2.0 / self.height;
            if self.rotation == 0.0 {
                self.view_matrix = [w, 0.0, -self.x * w, 0.0, h, -self.y * h, 0.0, 0.0, 1.0];
            } else {
                let cos = self.rotation.cos();
                let sin = self.rotation.sin();
                self.view_matrix = [
                    cos * w,
                    sin * w,
                    -(self.x * cos + self.y * sin) * w,
                    -sin * h,
                    cos * h,
                    (self.x * sin - self.y * cos) * h,
                    0.0,
                    0.0,
                    1.0,
                ];
            }
        }
        &self.view_matrix
    }

    /// Update the screen ratio when the screen size change. If a resize happen, and this function
    /// is not called, the view will be distorted.
    ///
    /// The smaller dimension of the view will be preserved, while the other dimension will change
    /// to keep the proportion. If you need another behaviour, use the functions [`set_width`],
    /// [`set_height`] or [`set_minor_size`], after calling this one.
    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.screen_size = (screen_width, screen_height);
        let side = self.width.min(self.height);
        if screen_width > screen_height {
            self.set_height(side);
        } else {
            self.set_width(side);
        }
    }

    /// Converts a position in the screen space (in pixels) to word space.
    pub fn position_to_word_space(&self, x: f32, y: f32) -> (f32, f32) {
        let x = (x - self.screen_size.0 as f32 / 2.0) * self.width / self.screen_size.0 as f32;
        let y = (y - self.screen_size.1 as f32 / 2.0) * self.height / self.screen_size.1 as f32;
        if self.rotation == 0.0 {
            (x + self.x, y + self.y)
        } else {
            let cos = self.rotation.cos();
            let sin = self.rotation.sin();
            (cos * x - sin * y + self.x, sin * x + cos * y + self.y)
        }
    }

    /// Converts a vector in the screen space (in pixels) to word space.
    pub fn vector_to_word_space(&self, x: f32, y: f32) -> (f32, f32) {
        // from screen to clip space: x' = x*2.0/sceen_width
        // during x' times camera matrix  x'' = cos*x'*self.width/2.0 = cos*x*self.width/self.screen_width
        let x = x * self.width / self.screen_size.0 as f32;
        let y = y * self.height / self.screen_size.1 as f32;
        if self.rotation == 0.0 {
            (x, y)
        } else {
            let cos = self.rotation.cos();
            let sin = self.rotation.sin();
            (cos * x - sin * y, sin * x + cos * y)
        }
    }

    /// Set the view height or the view width, whichever is the smallest, keeping the screen
    /// proportion by calculating the other dimension.
    pub fn set_minor_size(&mut self, size: f32) {
        if self.screen_size.0 > self.screen_size.1 {
            self.set_height(size);
        } else {
            self.set_width(size);
        }
    }

    /// Get the view width, in world space.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Set the view width, keeping the screen proportion by calculating the other dimension.
    pub fn set_width(&mut self, new_width: f32) {
        let new_height = new_width * self.screen_size.1 as f32 / self.screen_size.0 as f32;
        self.width = new_width;
        self.height = new_height;
        self.dirty = true;
    }

    /// Get the view height, in world space.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Set the view height, keeping the screen proportion by calculating the other dimension.
    pub fn set_height(&mut self, new_height: f32) {
        let new_width = new_height * self.screen_size.0 as f32 / self.screen_size.1 as f32;
        self.width = new_width;
        self.height = new_height;
        self.dirty = true;
    }

    #[inline]
    /// get the position of the center of the view in world space.
    pub fn get_position(&mut self) -> (f32, f32) {
        (self.x, self.y)
    }

    #[inline]
    /// set the position of the center of the view in the world space.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.dirty = true;
    }

    #[inline]
    /// move the view position in the world space.
    pub fn move_view(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        self.dirty = true;
    }

    #[inline]
    /// Set the angle of rotation of the view, in counterclockwise radians.
    pub fn set_view_rotation(&mut self, radians: f32) {
        self.rotation = radians.rem_euclid(2.0 * PI);
        self.dirty = true;
    }

    #[inline]
    /// Rotate the view by some angle, in couterclowise radians.
    pub fn rotate_view(&mut self, radians: f32) {
        self.rotation = (self.rotation + radians).rem_euclid(2.0 * PI);
        self.dirty = true;
    }

    #[inline]
    /// Scale the view size by some scalar.
    ///
    /// A value of 1.0 will keep the size, and a value greater than 1.0 will zoom out the camera.
    /// The position of the center of the camera in world space is preserved.
    pub fn scale_view(&mut self, scalar: f32) {
        let h = self.height();
        self.set_height(h * scalar);
    }

    #[inline]
    /// The `(width, height)` of the screen, in pixels.
    pub fn screen_size(&self) -> (u32, u32) {
        self.screen_size
    }
}
