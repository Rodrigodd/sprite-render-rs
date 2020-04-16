use glutin::dpi::PhysicalSize;
use std::f32::consts::PI;

#[repr(C)]
#[derive(Default, Clone)]
pub struct SpriteInstance {
    pub transform: [f32; 4],
    pub uv_rect: [f32; 4],
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub texture: u32,
    pub _padding: [f32; 1],
}

/// The camera encapsulates the projection matrix, providing methods to move, 
/// rotate or scale the camera view
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
    pub fn new(size: PhysicalSize<u32>, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: height * size.width as f32 / size.height as f32,
            height,
            rotation: 0.0,
            screen_size: size,
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
                    0.0,    h, -self.y*h,
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
    /// Without it the view will be distorted
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.screen_size = size;
        self.set_height(self.height);
    }

    /// Converts a vector from the screen space (in pixels) to word space
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

    /// Set the view height, keeping the screen proportion
    pub fn set_height(&mut self, new_height: f32) {
        let new_width = new_height * self.screen_size.width as f32/self.screen_size.height as f32;
        self.height = new_height;
        self.width = new_width;
        self.dirty = true;
    }


    #[inline]
    /// get the position of the center of the view in world space
    pub fn get_position(&mut self) -> (f32, f32) {
        (self.x, self.y)
    }

    #[inline]
    /// set the position of the center of the view in the world space
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.dirty = true;
    }

    #[inline]
    /// move the view position in the world space
    pub fn move_view(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        self.dirty = true;
    }

    #[inline]
    /// Set the angle of rotation of the view, in counterclockwise radians
    pub fn set_view_rotation(&mut self, radians: f32) {
        self.rotation = radians.rem_euclid(2.0*PI);
        self.dirty = true;
    }

    #[inline]
    /// Rotate the view by same angle, in couterclowise radians
    pub fn rotate_view(&mut self, radians: f32) {
        self.rotation = (self.rotation + radians).rem_euclid(2.0*PI);
        self.dirty = true;
    }
}