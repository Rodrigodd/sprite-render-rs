use winit::dpi::PhysicalSize;
use std::f32::consts::PI;

#[derive(Default, Clone)]
pub struct SpriteInstance {
    pub scale: [f32; 2],
    pub angle: f32,
    pub uv_rect: [f32; 4],
    pub color: [u8; 4],
    pub pos: [f32; 2],
    pub texture: u32,
}
impl SpriteInstance {
    /// Create a new SpriteInstant with center in (x,y) and with the given width, height, texture and uv_rect.
    /// The default color is white ([255, 255, 255, 255]).
    pub fn new(x: f32, y: f32, width: f32, height: f32, texture: u32, uv_rect: [f32;4]) -> Self {
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
    pub fn new_height_prop(x: f32, y: f32, height: f32, texture: u32, uv_rect: [f32;4]) -> Self {
        let width = height*uv_rect[2]/uv_rect[3];
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
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.scale = [width, height];
    }

    /// set the height of the sprite, keeping the proportion width / height.
    pub fn set_heigh_prop(&mut self, height: f32) {
        let width = height * self.get_width() / self.get_height();
        self.scale = [width, height];
    }

    /// get the width of the sprite.
    pub fn get_width(&self) -> f32 {
        self.scale[0]
    }

    /// get the height of the sprite.
    pub fn get_height(&self) -> f32 {
        self.scale[1]
    }

    /// set the angle of rotation, in counterclokwise radians.
    pub fn set_angle(&mut self, angle: f32) {
        self.angle = angle;
    }

    /// set the angle of the sprite, in a functional way (get owership of the value, and return it modified).
    /// The angle of rotation is in counterclokwise radians.
    pub fn with_angle(mut self, angle: f32) -> Self{
        self.angle = angle;
        self
    }

    /// set position of the sprite.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.pos = [x, y];
    }

    /// get the x axis of the position of the sprite.
    pub fn get_x(&self) -> f32 {
        self.pos[0]
    }

    /// get the y axis of the position of the sprite.
    pub fn get_y(&self) -> f32 {
        self.pos[1]
    }

    /// set the color of the sprite, in the range 0 to 255, in the RGBA format.
    /// 
    /// if you want to write in hexadecimal, you can write `sprite.set_color(0xRRGGBBAAu32.to_be_bytes())`
    /// for example.
    pub fn set_color(&mut self, color: [u8; 4]) {
        self.color = color;
    }

    /// set the color of the sprite, in a functional way (get owership of the value, and return it modified).
    /// The color is in the range 0 to 255, in the RBGA format.
    /// 
    /// if you want to write in hexadecimal, you can write `sprite.with_color(0xRRGGBBAAu32.to_be_bytes())`
    /// for example.
    pub fn with_color(mut self, color: [u8; 4]) -> Self{
        self.set_color(color);
        self
    }

    pub fn set_uv_rect(&mut self, rect: [f32; 4]) {
        self.uv_rect = rect;
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

    screen_size: PhysicalSize<u32>,
    
    view_matrix: [f32; 9],
    dirty: bool,
}
impl Camera {
    /// Create a new camera in (0.0, 0.0) position, with zero rotation, and with the 
    /// give height. The width is calculated to keep the screen_size proportion.
    pub fn new(screen_size: PhysicalSize<u32>, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: height * screen_size.width as f32 / screen_size.height as f32,
            height,
            rotation: 0.0,
            screen_size,
            view_matrix: [1.0, 0.0, 0.0,   0.0, 1.0, 0.0,   0.0, 0.0, 1.0],
            dirty: true,
        }
    }

    pub(crate) fn view(&mut self) -> &[f32; 9] {
        if self.dirty {
            self.dirty = false;
            let w = 2.0/self.width;
            let h = 2.0/self.height;
            if self.rotation == 0.0 {
                self.view_matrix = [
                      w,  0.0, -self.x*w,
                    0.0,   h, -self.y*h,
                    0.0,  0.0, 1.0,
                ];
            } else {
                let cos = self.rotation.cos();
                let sin = self.rotation.sin();
                self.view_matrix = [
                     cos*w, sin*w, -(self.x*cos + self.y*sin)*w,
                    -sin*h, cos*h,  (self.x*sin - self.y*cos)*h,
                    0.0,  0.0, 1.0,
                ];
            }
        }
        &self.view_matrix
    }

    /// Update the screen ratio when the screen size change.
    /// If a resize happen, and this function not called, the view will be distorted.
    ///
    /// The new smaller view size will be equal to the last smaller view size,
    /// the other view size change to keep proportion.
    /// If you want another behaviour, change the view size using
    /// ```set_width```, ```set_height``` or ```set_minor_size``` function.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.screen_size = size;
        let side = self.width.min(self.height);
        if size.width > size.height {
            self.set_height(side);
        } else {
            self.set_width(side);
        }
    }

    /// Converts a position in the screen space (in pixels) to word space.
    pub fn position_to_word_space(&self, x: f32, y: f32) -> (f32, f32) {
        let x = (x - self.screen_size.width  as f32 / 2.0)*self.width/self.screen_size.width  as f32;
        let y = (y - self.screen_size.height as f32 / 2.0)*self.height/self.screen_size.height as f32;
        if self.rotation == 0.0 {
            (x + self.x, y + self.y)
        } else {
            let cos = self.rotation.cos();
            let sin = self.rotation.sin();
            (
                cos*x - sin*y + self.x,
                sin*x + cos*y + self.y
            )
        }
    }

    /// Converts a vector in the screen space (in pixels) to word space.
    pub fn vector_to_word_space(&self, x: f32, y: f32) -> (f32, f32) {
        // from screen to clip space: x' = x*2.0/sceen_width
        // during x' times camera matrix  x'' = cos*x'*self.width/2.0 = cos*x*self.width/self.screen_width
        let x = x*self.width/self.screen_size.width  as f32;
        let y = y*self.height/self.screen_size.height as f32;
        if self.rotation == 0.0 {
            (x, y)
        } else {
            let cos = self.rotation.cos();
            let sin = self.rotation.sin();
            (
                cos*x - sin*y,
                sin*x + cos*y
            )
        }
    }

    /// Set the view height or the view width, whichever is the smaller, keeping the screen proportion
    /// by calculating the other dimension.
    pub fn set_minor_size(&mut self, size: f32) {
        if self.screen_size.width > self.screen_size.height {
            self.set_height(size);
        } else {
            self.set_width(size);
        }
    }
    
    /// Set the view width, keeping the screen proportion by calculating the other dimension.
    pub fn set_width(&mut self, new_width: f32) {
        let new_height = new_width * self.screen_size.height as f32/self.screen_size.width as f32;
        self.width = new_width;
        self.height = new_height;
        self.dirty = true;
    }

    /// Set the view height, keeping the screen proportion by calculating the other dimension.
    pub fn set_height(&mut self, new_height: f32) {
        let new_width = new_height * self.screen_size.width as f32/self.screen_size.height as f32;
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
        self.rotation = radians.rem_euclid(2.0*PI);
        self.dirty = true;
    }

    #[inline]
    /// Rotate the view by some angle, in couterclowise radians.
    pub fn rotate_view(&mut self, radians: f32) {
        self.rotation = (self.rotation + radians).rem_euclid(2.0*PI);
        self.dirty = true;
    }
}