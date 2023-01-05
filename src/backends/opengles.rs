use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    io::{self, Write},
    marker::PhantomData,
    mem,
    ops::Deref,
    os::raw::{c_char, c_void},
    ptr, str,
};

use raw_gl_context::{Api, GlConfig, GlContext};
use winit::window::{Window, WindowId};

use crate::{common::*, Renderer, SpriteRender};

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gles_bindings.rs"));
}
use gl::types::*;

const SPRITE_VERTEX_STRIDE: usize = mem::size_of::<f32>() * 6;

const VERTEX_SHADER_SOURCE: &str = r#"
#version 100
attribute vec2 position;
attribute vec2 uv;
attribute vec4 aColor;
attribute float aTexture;

uniform mat3 view;

varying vec4 color;
varying vec2 TexCoord;
varying float textureIndex;

void main() {
    gl_Position = vec4((vec3(position, 1.0) * view).xy, 0.0, 1.0);
    gl_Position.y *= -1.0;
    color = aColor;
    TexCoord = uv;
    textureIndex = aTexture;
}
"#;

unsafe fn transmute_slice<T, U>(slice: &[T]) -> &[U] {
    debug_assert!(
        mem::align_of::<T>() % mem::size_of::<U>() == 0,
        "T alignment must be multiple of U alignment"
    );
    debug_assert!(
        mem::size_of::<T>() % mem::size_of::<U>() == 0,
        "T size must be multiple of U size"
    );
    std::slice::from_raw_parts(
        slice.as_ptr() as *const T as *const U,
        slice.len() * mem::size_of::<T>() / mem::size_of::<U>(),
    )
}

unsafe fn gl_check_error_(file: &str, line: u32, label: &str) -> u32 {
    let mut error_code = gl::GetError();
    while error_code != gl::NO_ERROR {
        let error = match error_code {
            gl::INVALID_ENUM => "INVALID_ENUM",
            gl::INVALID_VALUE => "INVALID_VALUE",
            gl::INVALID_OPERATION => "INVALID_OPERATION",
            gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
            gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
            _ => "unknown GL error code",
        };

        log::error!("[{}:{:4}] {}: {}", file, line, label, error);

        error_code = gl::GetError();
    }
    error_code
}

macro_rules! gl_check_error {
    ($($arg:tt)*) => (
        gl_check_error_(file!(), line!(), &format!($($arg)*))
    )
}

macro_rules! cstr {
    ($s:literal) => {
        (concat!($s, "\0").as_bytes().as_ptr() as *const GLchar)
    };
}

unsafe fn get_uniform_location(shader_program: u32, name: &str) -> i32 {
    let s = CString::new(name).unwrap();
    gl::GetUniformLocation(shader_program, s.as_ptr())
}

pub struct GlesRenderer<'a> {
    render: &'a mut GlesSpriteRender,
}
impl<'a> Renderer for GlesRenderer<'a> {
    fn clear_screen(&mut self, color: &[f32; 4]) -> &mut dyn Renderer {
        log::trace!(
            "clear screen to [{:5.3}, {:5.3}, {:5.3}, {:5.3}]",
            color[0],
            color[1],
            color[2],
            color[3]
        );
        unsafe {
            gl::ClearColor(color[0], color[1], color[2], color[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self
    }

    fn draw_sprites(
        &mut self,
        camera: &mut Camera,
        sprites: &[SpriteInstance],
    ) -> &mut dyn Renderer {
        log::trace!("draw {} sprites", sprites.len());
        if sprites.is_empty() {
            return self;
        }
        if sprites.len() > self.render.buffer_size as usize {
            self.render.reallocate_instance_buffer(sprites.len());
        }

        self.render.texture_unit_map.clear();
        unsafe {
            let mut data: Vec<u8> = Vec::with_capacity(sprites.len() * SPRITE_VERTEX_STRIDE * 4);

            for sprite in sprites {
                let texture_unit = if let Some(t) =
                    self.render.texture_unit_map.get(&sprite.texture)
                {
                    *t
                } else {
                    if self.render.texture_unit_map.len() == self.render.max_texture_units as usize
                    {
                        unimplemented!("Split rendering in multiples draw calls when number of textures is greater than MAX_TEXTURE_IMAGE_UNITS is unimplemented.");
                    }
                    gl::ActiveTexture(gl::TEXTURE0 + self.render.texture_unit_map.len() as u32);
                    log::trace!("active texture");
                    gl::BindTexture(gl::TEXTURE_2D, sprite.texture);
                    log::trace!("bind texture");
                    self.render
                        .texture_unit_map
                        .insert(sprite.texture, self.render.texture_unit_map.len() as u32);
                    self.render.texture_unit_map.len() as u32 - 1
                };
                GlesSpriteRender::write_sprite(&mut data, sprite, texture_unit as u16).unwrap();
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, self.render.buffer);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                data.len() as GLsizeiptr,
                data.as_ptr() as *const c_void,
            );
            log::trace!(
                "buffer subdata: len {}, buffer size {}",
                data.len(),
                self.render.buffer_size
            );

            // render
            log::debug!("bind program");
            gl::UseProgram(self.render.shader_program);
            let text_units = (0..self.render.max_texture_units).collect::<Vec<i32>>();
            log::debug!("write uniform");
            gl::Uniform1iv(
                get_uniform_location(self.render.shader_program, "text"),
                16,
                text_units.as_ptr(),
            );
            log::debug!("write uniform");
            gl::UniformMatrix3fv(
                get_uniform_location(self.render.shader_program, "view"),
                1,
                gl::FALSE,
                camera.view().as_ptr(),
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.render.indice_buffer);
            gl_check_error!("draw arrays instanced");
            gl::DrawElements(
                gl::TRIANGLES,
                sprites.len() as i32 * 6,
                gl::UNSIGNED_SHORT,
                ptr::null(),
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);

            gl_check_error!("end frame");
        }
        self
    }

    fn finish(&mut self) {
        log::trace!("finish");
        self.render
            .current_context
            .as_ref()
            .unwrap()
            .1
            .swap_buffers();
    }
}

trait ContextCurrentState {}
struct NotCurrent;
struct PossiblyCurrent;
impl ContextCurrentState for NotCurrent {}
impl ContextCurrentState for PossiblyCurrent {}

struct Context<T: ContextCurrentState> {
    context: GlContext,
    _p: PhantomData<T>,
}
impl Context<NotCurrent> {
    unsafe fn make_current(self) -> Context<PossiblyCurrent> {
        let Self { context, .. } = self;
        context.make_current();
        Context {
            context,
            _p: Default::default(),
        }
    }
}
impl Context<PossiblyCurrent> {
    unsafe fn treat_as_not_current(self) -> Context<NotCurrent> {
        let Self { context, .. } = self;
        Context {
            context,
            _p: Default::default(),
        }
    }

    unsafe fn make_not_current(self) -> Context<NotCurrent> {
        let Self { context, .. } = self;
        context.make_not_current();
        Context {
            context,
            _p: Default::default(),
        }
    }
}
impl<T: ContextCurrentState> Deref for Context<T> {
    type Target = GlContext;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

pub struct GlesSpriteRender {
    vsync: bool,
    contexts: HashMap<WindowId, Option<Context<NotCurrent>>>,
    current_context: Option<(WindowId, Context<PossiblyCurrent>)>,
    shader_program: u32,
    indice_buffer: u32,
    buffer: u32,
    /// Buffer size in number of sprites
    buffer_size: u32,
    textures: Vec<(u32, u32, u32)>, // id, width, height
    /// maps a texture to a texture unit
    texture_unit_map: HashMap<u32, u32>,
    max_texture_units: i32,
}
impl GlesSpriteRender {
    /// Get a WindowBuilder and a event_loop (for opengl support), and return a window and Self.
    // TODO: build a better error handling!!!!
    pub fn new(window: &Window, vsync: bool) -> Result<Self, String> {
        let config = GlConfig {
            vsync,
            version: (2, 0),
            api: Api::Gles,
            ..Default::default()
        };
        let context = unsafe {
            let context = GlContext::create(window, config).map_err(|x| format!("{:?}", x))?;
            context.make_current();
            context
        };

        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

        unsafe {
            let string = gl::GetString(gl::VERSION);
            let string = CStr::from_ptr(string as *const c_char);
            log::info!("OpenGL version: {}", string.to_str().unwrap());
        }

        unsafe {
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
        }

        let mut max_texture_units = 0;
        unsafe {
            gl::GetIntegerv(gl::MAX_TEXTURE_IMAGE_UNITS, &mut max_texture_units);
        }
        log::info!("MAX_TEXTURE_IMAGE_UNITS: {}", max_texture_units);

        let (shader_program, buffer, indice_buffer) = unsafe {
            log::trace!("compiling vert shader");
            let vert_shader =
                Self::compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER_SOURCE).unwrap();

            log::trace!("compiling vert shader");
            let frag_shader = Self::compile_shader(
                gl::FRAGMENT_SHADER,
                &format!(
                    r#"
#version 100
#define MAX_TEXTURE_IMAGE_UNITS {}
precision mediump float;

uniform sampler2D text[MAX_TEXTURE_IMAGE_UNITS];

varying vec4 color;
varying vec2 TexCoord;
varying float textureIndex;

void main() {{
    int t = int(textureIndex);
    vec4 textureColor;
    for (int i = 0; i < MAX_TEXTURE_IMAGE_UNITS; i++ ) {{
        if (i == t) textureColor = texture2D(text[i], TexCoord);
    }}
    
    if (textureColor.a == 0.0 || color.a == 0.0) {{
        discard;
    }}
    gl_FragColor = textureColor*color;
}}
"#,
                    max_texture_units,
                ),
            )
            .unwrap();

            log::trace!("linking shader");
            let shader_program = Self::link_program(vert_shader, frag_shader).unwrap();

            gl_check_error!("linked program");

            gl::UseProgram(shader_program);

            log::trace!("generating buffers");
            let mut buffers = [0; 2];
            gl::GenBuffers(2, buffers.as_mut_ptr() as *mut GLuint);
            let [buffer, indice_buffer] = buffers;
            log::debug!("buffers: {} {}", buffer, indice_buffer);

            gl_check_error!("gen buffers");

            log::trace!("setting attributes");
            gl::BindBuffer(gl::ARRAY_BUFFER, buffer);

            let position = gl::GetAttribLocation(shader_program, cstr!("position")) as u32;
            dbg!(position);
            gl_check_error!("get position attribute location");
            gl::VertexAttribPointer(
                position,
                2,
                gl::FLOAT,
                gl::FALSE,
                SPRITE_VERTEX_STRIDE as i32,
                ptr::null(),
            );
            gl_check_error!("position vertex attrib pointer");
            gl::EnableVertexAttribArray(position);
            gl_check_error!("position enable vertex attrib array");

            let uv = gl::GetAttribLocation(shader_program, cstr!("uv")) as u32;
            dbg!(uv);
            gl::VertexAttribPointer(
                uv,
                2,
                gl::FLOAT,
                gl::FALSE,
                SPRITE_VERTEX_STRIDE as i32,
                (mem::size_of::<f32>() * 2) as *const c_void,
            );
            gl::EnableVertexAttribArray(uv);

            let a_color = gl::GetAttribLocation(shader_program, cstr!("aColor")) as u32;
            gl::VertexAttribPointer(
                a_color,
                4,
                gl::UNSIGNED_BYTE,
                gl::TRUE,
                SPRITE_VERTEX_STRIDE as i32,
                (mem::size_of::<f32>() * 4) as *const c_void,
            );
            gl::EnableVertexAttribArray(a_color);

            let a_texture = gl::GetAttribLocation(shader_program, cstr!("aTexture")) as u32;
            gl::VertexAttribPointer(
                a_texture,
                1,
                gl::UNSIGNED_SHORT,
                gl::FALSE,
                SPRITE_VERTEX_STRIDE as i32,
                (mem::size_of::<f32>() * 5) as *const c_void,
            );
            gl::EnableVertexAttribArray(a_texture);

            gl_check_error!("set vertex attributes");

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            (shader_program, buffer, indice_buffer)
        };

        log::trace!("finished sprite-render creation");
        let mut contexts = HashMap::new();
        let window_id = window.id();
        contexts.insert(window_id, None);

        let mut sprite_render = Self {
            vsync,
            shader_program,
            contexts,
            current_context: Some((
                window_id,
                Context {
                    context,
                    _p: Default::default(),
                },
            )),
            buffer,
            indice_buffer,
            buffer_size: 0,
            textures: Vec::new(),
            texture_unit_map: HashMap::new(),
            max_texture_units,
        };
        let size = window.inner_size();
        sprite_render.resize(window.id(), size.width, size.height);

        Ok(sprite_render)
    }

    unsafe fn compile_shader(shader_type: u32, source: &str) -> Result<u32, String> {
        log::trace!("CreateShader");
        if !gl::CreateShader::is_loaded() {
            panic!("CreateShader is not loaded!!");
        }
        let shader = gl::CreateShader(shader_type);
        log::trace!("CString");
        let c_str = CString::new(source).unwrap();
        log::trace!("ShaderSoruce");
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        log::trace!("CompileShader");
        gl::CompileShader(shader);

        // Check for shader compilation errors
        let mut success = i32::from(gl::FALSE);
        log::trace!("GetShaderiv");
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == gl::FALSE as i32 {
            let mut len = 0;
            log::trace!("GetShaderiv(INFO_LOG_LENGTH)");
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            // len includes the NULL character
            let mut buffer = vec![0u8; len as usize];

            log::trace!("GetShaderInfoLog (len {})", len);
            gl::GetShaderInfoLog(shader, len, &mut len, buffer.as_mut_ptr() as *mut GLchar);

            log::trace!("DeleteShader");
            gl::DeleteShader(shader);

            let info_log = if len == 0 {
                String::from("Unknown error creating shader")
            } else {
                String::from_utf8_lossy(&buffer[0..len as usize]).into_owned()
            }
            .replace("\\n", "\n");

            log::error!(
                "failing compiling {} shader: {}",
                match shader_type {
                    gl::VERTEX_SHADER => "vertex",
                    gl::FRAGMENT_SHADER => "fragment",
                    _ => "unknown",
                },
                info_log
            );
            Err(info_log)
        } else {
            Ok(shader)
        }
    }

    unsafe fn link_program(vertex_shader: u32, fragment_shader: u32) -> Result<u32, String> {
        let shader_program = gl::CreateProgram();
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);
        gl::LinkProgram(shader_program);

        // Check for linking errors
        let mut success = i32::from(gl::FALSE);
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
        let result = if success != i32::from(gl::TRUE) {
            let mut len = 0;
            let mut info_log = [0u8; 512];
            gl::GetProgramInfoLog(
                shader_program,
                info_log.len() as i32,
                (&mut len) as *mut GLsizei,
                info_log.as_mut_ptr() as *mut GLchar,
            );
            let info_log = if len == 0 {
                String::from("Unknown error linking shader")
            } else {
                String::from_utf8_lossy(&info_log[0..len as usize]).into_owned()
            }
            .replace("\\n", "\n");
            Err(info_log)
        } else {
            Ok(shader_program)
        };

        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);

        result
    }

    unsafe fn write_sprite<W: Write>(
        writer: &mut W,
        sprite: &SpriteInstance,
        texture: u16,
    ) -> io::Result<()> {
        let cos = sprite.angle.cos();
        let sin = sprite.angle.sin();
        let width = sprite.get_width() / 2.0;
        let height = sprite.get_height() / 2.0;
        let x = sprite.get_x();
        let y = sprite.get_y();
        let u = sprite.uv_rect[0];
        let v = sprite.uv_rect[1];
        let w = sprite.uv_rect[2];
        let h = sprite.uv_rect[3];

        // bottom left
        writer.write(&transmute_slice(&[
            -cos * width + sin * height + x,
            -sin * width - cos * height + y,
            u,
            v,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride

        // bottom right
        writer.write(&transmute_slice(&[
            cos * width + sin * height + x,
            sin * width - cos * height + y,
            u + w,
            v,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride

        // top left
        writer.write(&transmute_slice(&[
            -cos * width - sin * height + x,
            -sin * width + cos * height + y,
            u,
            v + h,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride

        // top right
        writer.write(&transmute_slice(&[
            cos * width - sin * height + x,
            sin * width + cos * height + y,
            u + w,
            v + h,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&(texture as u16).to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride
        Ok(())
    }

    fn reallocate_instance_buffer(&mut self, size_need: usize) {
        let new_size = size_need.next_power_of_two();
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
            dbg!(self.buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (new_size * SPRITE_VERTEX_STRIDE * 4) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            gl_check_error!("reallocate buffer to {}", new_size);

            let indices = (0..(new_size * 6 * mem::size_of::<u16>()) as u16)
                .map(|x| x / 6 * 4 + [0u16, 1, 2, 1, 2, 3][x as usize % 6])
                .collect::<Vec<u16>>();

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.indice_buffer);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u16>()) as GLsizeiptr,
                &*indices as *const _ as *const c_void,
                gl::DYNAMIC_DRAW,
            );
            gl_check_error!("reallocate indice buffer to {}", new_size);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
        self.buffer_size = new_size as u32;
    }

    fn set_current_context(&mut self, window_id: WindowId) {
        let already_current = self
            .current_context
            .as_ref()
            .map_or(false, |x| x.0 == window_id);
        if !already_current {
            let previous_context = self.current_context.take();
            unsafe {
                self.current_context = Some((
                    window_id,
                    self.contexts
                        .get_mut(&window_id)
                        .unwrap()
                        .take()
                        .unwrap()
                        .make_current(),
                ));
            }
            if let Some((window, context)) = previous_context {
                unsafe {
                    *self.contexts.get_mut(&window).unwrap() = Some(context.treat_as_not_current());
                }
            }
        }
    }
}
impl SpriteRender for GlesSpriteRender {
    fn add_window(&mut self, window: &Window) {
        if self.contexts.contains_key(&window.id()) {
            log::warn!("Tried to add a window to SpriteRender twice");
            return;
        }

        let config = GlConfig {
            vsync: self.vsync,
            share: Some(&self.current_context.as_ref().unwrap().1.context),
            ..Default::default()
        };
        let context = unsafe { GlContext::create(window, config).unwrap() };

        let window_id = window.id();
        self.contexts.insert(
            window_id,
            Some(Context {
                context,
                _p: Default::default(),
            }),
        );

        self.set_current_context(window_id);

        unsafe {
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
        }
    }

    fn remove_window(&mut self, window_id: WindowId) {
        let mut context = self.contexts.remove(&window_id).flatten();
        if let Some((id, _)) = self.current_context.as_mut() {
            if *id == window_id {
                unsafe {
                    context = Some(self.current_context.take().unwrap().1.make_not_current());
                }
            }
        }
        drop(context);
    }

    /// Load a Texture in the GPU. if linear_filter is true, the texture will be sampled with linear filter applied.
    /// Pixel art don't use linear filter.
    fn new_texture(&mut self, width: u32, height: u32, data: &[u8], linear_filter: bool) -> u32 {
        log::trace!("new texture {width}x{height}");
        unsafe {
            let mut texture = 0;
            gl::ActiveTexture(gl::TEXTURE0 + self.texture_unit_map.len() as u32);
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                if linear_filter {
                    gl::LINEAR
                } else {
                    gl::NEAREST
                } as i32,
            );
            let data_ptr;
            if data.len() as u32 >= width * height * 4 {
                data_ptr = data.as_ptr() as *const c_void;
            } else {
                data_ptr = std::ptr::null::<c_void>();
            }
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data_ptr,
            );
            self.textures.push((texture, width, height));
            texture
        }
    }

    fn update_texture(&mut self, texture: u32, data: &[u8], sub_rect: Option<[u32; 4]>) {
        log::trace!("update texture {texture}");
        let rect = sub_rect.unwrap_or({
            let size = self
                .textures
                .iter()
                .find(|(id, _, _)| *id == texture)
                .unwrap();
            [0, 0, size.1, size.2]
        });
        let expected_len = (rect[2] * rect[3] * 4) as usize;
        assert!(
            data.len() == expected_len,
            "expected data length was {}x{}x4={}, but receive a data of length {}",
            rect[2],
            rect[3],
            expected_len,
            data.len()
        );
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                rect[0] as i32,
                rect[1] as i32,
                rect[2] as i32,
                rect[3] as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const c_void,
            );
        }
    }

    fn resize_texture(&mut self, texture: u32, width: u32, height: u32, data: &[u8]) {
        let data_ptr;
        if data.len() as u32 >= width * height * 4 {
            data_ptr = data.as_ptr() as *const c_void;
        } else {
            data_ptr = std::ptr::null::<c_void>();
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data_ptr,
            );
        }
    }

    fn render<'a>(&'a mut self, window: WindowId) -> Box<dyn Renderer + 'a> {
        self.set_current_context(window);
        Box::new(GlesRenderer { render: self })
    }

    fn resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        self.set_current_context(window_id);
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
        }
    }
}
