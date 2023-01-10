use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    io::{self, Write},
    mem,
    num::NonZeroU32,
    os::raw::c_void,
    ptr, str,
};

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, DisplayApiPreference, GetGlDisplay},
    prelude::{
        GlConfig, GlDisplay, NotCurrentGlContextSurfaceAccessor,
        PossiblyCurrentContextGlSurfaceAccessor, PossiblyCurrentGlContext,
    },
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::{Window, WindowId};

use crate::{common::*, Renderer, SpriteRender, Texture, TextureError, TextureFilter, TextureId};

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
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

pub struct GlRenderer<'a> {
    render: &'a mut GlSpriteRender,
}
impl<'a> Renderer for GlRenderer<'a> {
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
        let Some(res) = &mut self.render.shared_resources else {
            panic!("OpenGL context don't exist.")
        };

        log::trace!("draw {} sprites", sprites.len());
        if sprites.is_empty() {
            return self;
        }
        if sprites.len() > res.buffer_size as usize {
            res.reallocate_vertex_buffer(sprites.len());
        }

        res.texture_unit_map.clear();
        unsafe {
            let mut data: Vec<u8> = Vec::with_capacity(sprites.len() * SPRITE_VERTEX_STRIDE * 4);

            for sprite in sprites {
                let texture_unit = if let Some(t) = res.texture_unit_map.get(&sprite.texture) {
                    *t
                } else {
                    if res.texture_unit_map.len() == res.max_texture_units as usize {
                        unimplemented!("Split rendering in multiples draw calls when number of textures is greater than MAX_TEXTURE_IMAGE_UNITS is unimplemented.");
                    }
                    let unit = res.texture_unit_map.len() as u32;
                    gl::ActiveTexture(gl::TEXTURE0 + unit);
                    log::trace!("active texture {}", unit);
                    let texture = sprite.texture.0;
                    gl::BindTexture(gl::TEXTURE_2D, texture);
                    log::trace!("bind texture {}", sprite.texture);

                    res.texture_unit_map.insert(sprite.texture, unit);

                    unit
                };
                GlSpriteRender::write_sprite(&mut data, sprite, texture_unit as u16).unwrap();
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, res.vertex_buffer);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                data.len() as GLsizeiptr,
                data.as_ptr() as *const c_void,
            );
            log::trace!(
                "buffer subdata: len {}, buffer size {}",
                data.len(),
                res.buffer_size
            );

            // render
            gl::UseProgram(res.shader_program);
            let text_units = (0..res.max_texture_units).collect::<Vec<i32>>();
            gl::Uniform1iv(
                get_uniform_location(res.shader_program, "text"),
                16,
                text_units.as_ptr(),
            );
            gl::UniformMatrix3fv(
                get_uniform_location(res.shader_program, "view"),
                1,
                gl::FALSE,
                camera.view().as_ptr(),
            );

            let Some(res) = &self.render.shared_resources else {
            panic!("OpenGL context don't exist.")
        };

            if let Some(vao) = self.render.vao() {
                gl::BindVertexArray(vao);
            }
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, res.indice_buffer);
            gl_check_error!("draw arrays instanced");
            gl::DrawElements(
                gl::TRIANGLES,
                sprites.len() as i32 * 6,
                gl::UNSIGNED_SHORT,
                ptr::null(),
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            if self.render.vao().is_some() {
                gl::BindVertexArray(0);
            }

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
            .swap_buffers()
            .unwrap();
    }
}

#[derive(Debug)]
pub enum Error {
    Glutin(glutin::error::Error),
    /// Either width or height were zero.
    BadDimensions,
    /// glLString(GL_VERSION) returned null, or a invalid version string.
    CouldNotQueryVersion,
    /// OpenGL major version is smaller than 2.
    UnsupportedOpenGlVersion,
}
impl From<glutin::error::Error> for Error {
    fn from(value: glutin::error::Error) -> Self {
        Self::Glutin(value)
    }
}

struct Context<T> {
    context: T,
    surface: Surface<WindowSurface>,
    config: glutin::config::Config,
    /// It is None when OpenGL version is 2.0
    vao: Option<u32>,
}
impl<T> Context<T> {
    fn map<U, F: FnOnce(T, &Surface<WindowSurface>) -> glutin::error::Result<U>>(
        self,
        f: F,
    ) -> glutin::error::Result<Context<U>> {
        let Context {
            context,
            surface,
            config,
            vao,
        } = self;
        Ok(Context {
            context: f(context, &surface)?,
            surface,
            config,
            vao,
        })
    }
}
impl Context<NotCurrentContext> {
    fn make_current(self) -> glutin::error::Result<Context<PossiblyCurrentContext>> {
        self.map(|ctx, surface| ctx.make_current(surface))
    }
}
impl Context<PossiblyCurrentContext> {
    fn new(
        window: &Window,
        vsync: bool,
        shared: Option<&Context<PossiblyCurrentContext>>,
    ) -> Result<Self, Error> {
        let raw_window_handle = window.raw_window_handle();
        let raw_display_handle = window.raw_display_handle();

        #[cfg(target_os = "macos")]
        let preference = DisplayApiPreference::Cgl;
        #[cfg(target_os = "android")]
        let preference = DisplayApiPreference::Egl;
        #[cfg(target_os = "linux")]
        let preference = DisplayApiPreference::GlxThenEgl(Some(raw_window_handle));
        #[cfg(target_os = "windows")]
        let preference = DisplayApiPreference::WglThenEgl(Some(raw_window_handle));

        let display = unsafe { Display::new(raw_display_handle, preference)? };

        let template = ConfigTemplateBuilder::new()
            .compatible_with_native_window(raw_window_handle)
            .build();

        let config = {
            let configs = unsafe { display.find_configs(template)? };
            configs
                .reduce(|accum, config| {
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        };
        log::debug!("Picked config: {:?}", config);
        log::debug!("Picked a config with {} samples", config.num_samples());

        let display = config.display();

        let context_attributes = {
            let builder = ContextAttributesBuilder::new();
            let builder = if let Some(context) = shared {
                builder.with_sharing(&context.context)
            } else {
                builder
            };
            builder.build(Some(raw_window_handle))
        };

        let context = unsafe { display.create_context(&config, &context_attributes)? };

        let size = window.inner_size();
        let (width, height) = match (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            (Some(w), Some(h)) => (w, h),
            _ => return Err(Error::BadDimensions),
        };

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            width,
            height,
        );
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes)? };

        let context = context.make_current(&surface)?;

        if vsync {
            surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))?;
        }

        Ok(Context {
            context,
            surface,
            config,
            vao: None,
        })
    }

    fn make_current(&self) -> Result<(), glutin::error::Error> {
        self.context.make_current(&self.surface)
    }

    fn make_not_current(self) -> glutin::error::Result<Context<NotCurrentContext>> {
        self.map(|ctx, _| ctx.make_not_current())
    }

    fn swap_buffers(&self) -> glutin::error::Result<()> {
        self.surface.swap_buffers(&self.context)
    }
}

/// OpenGL resources that are created only once, and are shader by all OpenGL contexts.
struct SharedResources {
    /// The OpenGL object for the Shader.
    shader_program: u32,
    /// The OpenGL object for the Indice Buffer.
    indice_buffer: u32,
    /// The OpenGL object for the Vertex Buffer.
    vertex_buffer: u32,

    /// Buffer size in number of sprites
    buffer_size: u32,
    // Textures currently loaded in OpenGL. Are a tuple of  (id, width, height)
    textures: Vec<(TextureId, u32, u32)>,
    /// maps a texture to a texture unit
    texture_unit_map: HashMap<TextureId, u32>,
    /// The maximum number of Textures Units supported by the curretn OpenGL context.
    max_texture_units: i32,
}
impl SharedResources {
    fn reallocate_vertex_buffer(&mut self, size_need: usize) {
        let new_size = size_need.next_power_of_two();
        log::trace!("reallocating vertex buffer: size need {size_need}, new_size {new_size}");
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (new_size * SPRITE_VERTEX_STRIDE * 4) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            gl_check_error!("reallocate buffer to {}", new_size);

            let indices = (0..(new_size * 6) as u32)
                .map(|x| (x / 6 * 4) as u16 + [0u16, 1, 2, 1, 2, 3][x as usize % 6])
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
}

pub struct GlSpriteRender {
    vsync: bool,
    contexts: HashMap<WindowId, Option<Context<NotCurrentContext>>>,
    current_context: Option<(WindowId, Context<PossiblyCurrentContext>)>,
    major_version: u8,

    shared_resources: Option<SharedResources>,
}
impl GlSpriteRender {
    /// Get a WindowBuilder and a event_loop (for opengl support), and return a window and Self.
    pub fn new(window: &Window, vsync: bool) -> Result<Self, Error> {
        let mut sprite_render = Self {
            vsync,
            contexts: HashMap::new(),
            current_context: None,
            major_version: 0,
            shared_resources: None,
        };

        #[cfg(target_os = "android")]
        {
            // TODO: need to find a reliable way of detecting if Android's window raw_window_handle
            // is null.
            // This may panic in the future: https://github.com/rust-windowing/winit/issues/2482
            if window.inner_size() == (0, 0).into() {
                return Ok(sprite_render);
            }
        }

        sprite_render.create_context_and_resources(window)?;

        Ok(sprite_render)
    }

    /// Create the first context and resources that will be shared by all following contexts.
    fn create_context_and_resources(&mut self, window: &Window) -> Result<(), Error> {
        let mut context = Context::new(window, self.vsync, None)?;

        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            context
                .config
                .display()
                .get_proc_address(symbol.as_c_str())
                .cast()
        });

        fn get_gl_string(variant: gl::types::GLenum) -> Option<&'static CStr> {
            unsafe {
                let s = gl::GetString(variant);
                (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
            }
        }

        let major_version = if let Some(version) = get_gl_string(gl::VERSION) {
            log::info!("OpenGL Version {}", version.to_string_lossy());
            let Some((major_version, _)) = parse_version_number(version) else {
                return Err(Error::CouldNotQueryVersion)
            };
            if major_version < 2 {
                return Err(Error::UnsupportedOpenGlVersion);
            }
            major_version
        } else {
            return Err(Error::CouldNotQueryVersion);
        };

        if let Some(shaders_version) = get_gl_string(gl::SHADING_LANGUAGE_VERSION) {
            log::info!("Shaders version on {}", shaders_version.to_string_lossy());
        }
        if let Some(renderer) = get_gl_string(gl::RENDERER) {
            log::info!("Running on {}", renderer.to_string_lossy());
        }

        let mut max_texture_units = 0;
        unsafe {
            gl::GetIntegerv(gl::MAX_TEXTURE_IMAGE_UNITS, &mut max_texture_units);
        }
        log::info!("MAX_TEXTURE_IMAGE_UNITS: {}", max_texture_units);

        unsafe {
            Self::init_context();
        }

        let shared_resources = unsafe { Self::create_resources(max_texture_units) };

        context.vao = unsafe {
            Self::create_vao(
                shared_resources.shader_program,
                shared_resources.vertex_buffer,
                major_version,
            )
        };

        log::trace!("finished sprite-render creation");
        let mut contexts = HashMap::new();
        let window_id = window.id();
        contexts.insert(window_id, None);

        self.contexts = contexts;
        self.current_context = Some((window.id(), context));
        self.major_version = major_version;
        self.shared_resources = Some(shared_resources);

        let size = window.inner_size();
        self.resize(window.id(), size.width, size.height);

        Ok(())
    }

    unsafe fn init_context() {
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Enable(gl::BLEND);
    }

    unsafe fn create_resources(max_texture_units: i32) -> SharedResources {
        log::trace!("compiling vert shader");
        let vert_shader = Self::compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER_SOURCE).unwrap();
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
        let [vertex_buffer, indice_buffer] = buffers;
        log::debug!("buffers: {} {}", vertex_buffer, indice_buffer);
        gl_check_error!("gen buffers");

        SharedResources {
            shader_program,
            indice_buffer,
            vertex_buffer,

            buffer_size: 0,

            textures: Vec::new(),
            texture_unit_map: HashMap::new(),
            max_texture_units,
        }
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
        writer.write_all(transmute_slice(&[
            -cos * width + sin * height + x,
            -sin * width - cos * height + y,
            u,
            v,
        ]))?;
        writer.write_all(&sprite.color)?;
        writer.write_all(&texture.to_ne_bytes())?;
        writer.write_all(&[0, 0])?; //complete the stride

        // bottom right
        writer.write_all(transmute_slice(&[
            cos * width + sin * height + x,
            sin * width - cos * height + y,
            u + w,
            v,
        ]))?;
        writer.write_all(&sprite.color)?;
        writer.write_all(&texture.to_ne_bytes())?;
        writer.write_all(&[0, 0])?; //complete the stride

        // top left
        writer.write_all(transmute_slice(&[
            -cos * width - sin * height + x,
            -sin * width + cos * height + y,
            u,
            v + h,
        ]))?;
        writer.write_all(&sprite.color)?;
        writer.write_all(&texture.to_ne_bytes())?;
        writer.write_all(&[0, 0])?; //complete the stride

        // top right
        writer.write_all(transmute_slice(&[
            cos * width - sin * height + x,
            sin * width + cos * height + y,
            u + w,
            v + h,
        ]))?;
        writer.write_all(&sprite.color)?;
        writer.write_all(&texture.to_ne_bytes())?;
        writer.write_all(&[0, 0])?; //complete the stride
        Ok(())
    }

    /// get vao from the current context
    fn vao(&self) -> Option<u32> {
        self.current_context.as_ref().unwrap().1.vao
    }

    unsafe fn create_vao(
        shader_program: u32,
        vertex_buffer: u32,
        major_version: u8,
    ) -> Option<u32> {
        let mut vao = None;
        if major_version > 2 {
            let mut vertex_array = 0;
            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);
            vao = Some(vertex_array);
        }

        log::trace!("setting attributes");
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

        let position = gl::GetAttribLocation(shader_program, cstr!("position")) as u32;
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
        if major_version > 2 {
            gl::BindVertexArray(0);
        }

        vao
    }

    fn set_current_context(&mut self, window_id: WindowId) -> Result<(), glutin::error::Error> {
        log::trace!("set current context to {:?}", window_id);
        let already_current = self
            .current_context
            .as_ref()
            .map_or(false, |x| x.0 == window_id);
        if !already_current {
            let previous_context = self.current_context.take();
            self.current_context = Some((
                window_id,
                self.contexts
                    .get_mut(&window_id)
                    .unwrap()
                    .take()
                    .unwrap()
                    .make_current()?,
            ));
            if let Some((window, context)) = previous_context {
                *self.contexts.get_mut(&window).unwrap() = Some(context.make_not_current()?);
            }
        } else {
            // Make the current context current again to be sure that it is.
            self.current_context.as_ref().unwrap().1.make_current()?;
        }
        Ok(())
    }
}

/// Parse a OpenGL version string "<major>.<minor><whatever..>" (where major and minor are sequences
/// of ascii digits) into tuple `(major, minor)`.
fn parse_version_number(version: &CStr) -> Option<(u8, u8)> {
    let bytes = version.to_bytes();
    let Some(start_pos) = bytes.iter().position(|x| x.is_ascii_digit()) else {
        return None;
    };
    let Some(( dot_pos , _)) = bytes
        .iter()
        .enumerate()
        .skip(start_pos)
        .find(|(_, x)| !x.is_ascii_digit())
    else {
        return None;
    };
    let end_pos = bytes
        .iter()
        .enumerate()
        .skip(dot_pos + 1)
        .find(|(_, x)| !x.is_ascii_digit())
        .map(|x| x.0)
        .unwrap_or(bytes.len());

    let major = std::str::from_utf8(&bytes[start_pos..dot_pos]).expect("is pure ascii");
    let minor = std::str::from_utf8(&bytes[dot_pos + 1..end_pos]).expect("is pure ascii");
    Some((major.parse().ok()?, minor.parse().ok()?))
}
impl SpriteRender for GlSpriteRender {
    fn add_window(&mut self, window: &Window) {
        log::trace!("add window {:?}", window.id());
        let window_id = window.id();

        // TODO: propagate errors
        let context = Context::new(
            window,
            self.vsync,
            self.current_context.as_ref().map(|x| &x.1),
        )
        .unwrap();

        self.contexts
            .insert(window_id, Some(context.make_not_current().unwrap()));
        self.set_current_context(window_id).unwrap();

        unsafe { Self::init_context() };

        self.current_context.as_mut().unwrap().1.vao = unsafe {
            let Some(res) = &self.shared_resources else {
                panic!("OpenGL context don't exist.")
            };
            Self::create_vao(res.shader_program, res.vertex_buffer, self.major_version)
        };
    }

    fn remove_window(&mut self, window_id: WindowId) {
        log::trace!("remove window {:?}", window_id);
        let mut context = self.contexts.remove(&window_id).flatten();
        if let Some((id, _)) = self.current_context.as_mut() {
            if *id == window_id {
                // TODO: propagate the error
                context = Some(
                    self.current_context
                        .take()
                        .unwrap()
                        .1
                        .make_not_current()
                        .unwrap(),
                );
            }
        }
        drop(context);
    }

    /// Load a Texture in the GPU. if linear_filter is true, the texture will be sampled with linear filter applied.
    /// Pixel art don't use linear filter.
    fn new_texture(&mut self, texture: Texture) -> Result<TextureId, TextureError> {
        let Texture {
            width,
            height,
            format,
            filter,
            data,
        } = texture;

        log::trace!("new texture {width}x{height}");
        let Some(res) = &mut self.shared_resources else {
            log::error!("OpenGL context don't exist.");
            return Err(TextureError::RendererContextDontExist);
        };

        unsafe {
            let mut texture = 0;
            gl::ActiveTexture(gl::TEXTURE0 + res.texture_unit_map.len() as u32);
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                match filter {
                    TextureFilter::Nearest => gl::NEAREST,
                    TextureFilter::Linear => gl::LINEAR,
                } as i32,
            );

            let data_ptr = match data {
                Some(data) => {
                    if data.len() as u32 != width * height * 4 {
                        return Err(TextureError::InvalidLength);
                    }
                    data.as_ptr() as *const c_void
                }
                None => std::ptr::null::<c_void>(),
            };

            let (internalformat, format, type_) = match format {
                crate::TextureFormat::Rgba8888 => (gl::RGBA as i32, gl::RGBA, gl::UNSIGNED_BYTE),
            };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                internalformat,
                width as i32,
                height as i32,
                0,
                format,
                type_,
                data_ptr,
            );
            let texture = TextureId(texture);
            res.textures.push((texture, width, height));
            Ok(texture)
        }
    }

    fn update_texture(
        &mut self,
        texture: TextureId,
        data: Option<&[u8]>,
        sub_rect: Option<[u32; 4]>,
    ) -> Result<(), TextureError> {
        log::trace!("update texture {texture}");
        let Some(res) = &mut self.shared_resources else {
            log::error!("OpenGL context don't exist.");
            return Err(TextureError::RendererContextDontExist);
        };

        let rect = sub_rect.unwrap_or({
            let size = res
                .textures
                .iter()
                .find(|(id, _, _)| *id == texture)
                .unwrap();
            [0, 0, size.1, size.2]
        });
        let expected_len = (rect[2] * rect[3] * 4) as usize;

        let data_ptr = match data {
            Some(data) => {
                if data.len() != expected_len {
                    log::error!(
                        "expected data length was {}x{}x4={}, but receive a data of length {}",
                        rect[2],
                        rect[3],
                        expected_len,
                        data.len()
                    );
                    return Err(TextureError::InvalidLength);
                }
                data.as_ptr() as *const c_void
            }
            None => std::ptr::null::<c_void>(),
        };

        let texture = texture.0;

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
                data_ptr,
            );
        }

        Ok(())
    }

    fn resize_texture(
        &mut self,
        texture: TextureId,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
    ) -> Result<(), TextureError> {
        log::trace!("resize texture {texture}");
        let Some(_) = &mut self.shared_resources else {
            log::error!("OpenGL context don't exist.");
            return Err(TextureError::RendererContextDontExist);
        };

        let texture = texture.0;

        let data_ptr = match data {
            Some(data) => {
                if data.len() as u32 != width * height * 4 {
                    return Err(TextureError::InvalidLength);
                }
                data.as_ptr() as *const c_void
            }
            None => std::ptr::null::<c_void>(),
        };

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

        Ok(())
    }

    fn render<'a>(&'a mut self, window_id: WindowId) -> Box<dyn Renderer + 'a> {
        log::trace!("render {:?}", window_id);
        if self.shared_resources.is_none() {
            log::warn!("OpenGL context don't exist.");
            return Box::new(crate::NoopRenderer);
        }
        self.set_current_context(window_id).unwrap();
        Box::new(GlRenderer { render: self })
    }

    fn resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        log::trace!("resize {:?}", window_id);
        if self.shared_resources.is_none() {
            log::warn!("OpenGL context don't exist.");
            return;
        };
        self.set_current_context(window_id).unwrap();
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
        }
    }

    fn resume(&mut self, window: &Window) {
        self.create_context_and_resources(window).unwrap();
    }

    fn suspend(&mut self) {
        self.contexts.clear();
        self.current_context.take();
        self.major_version = 0;
        self.shared_resources = None;
    }
}
