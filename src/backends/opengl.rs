use gl::types::*;
use raw_gl_context::{GlConfig, GlContext};
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::str;
use std::{collections::HashMap, ops::Deref};
use winit::window::{Window, WindowId};

use crate::common::*;
use crate::{Renderer, SpriteRender};

const VERTEX_STRIDE: GLsizei = 4 * mem::size_of::<GLfloat>() as GLsizei;
const INSTANCE_STRIDE: GLsizei = 16 * mem::size_of::<GLfloat>() as GLsizei;
const VERTICES: [f32; 16] = [
    // bottom left
    -0.5, -0.5, 0.0, 0.0, // bottom right
    0.5, -0.5, 1.0, 0.0, // top left
    -0.5, 0.5, 0.0, 1.0, // top right
    0.5, 0.5, 1.0, 1.0,
];

const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aUv;

layout (location = 2) in vec2 aSize;
layout (location = 3) in float aAngle;
layout (location = 4) in vec4 aUvRect;
layout (location = 5) in vec4 aColor;
layout (location = 6) in vec2 aOffset;
layout (location = 7) in int aTextureIndex;

uniform mat3 view;

out vec4 color;
out vec2 TexCoord;
flat out int TextureIndex;

void main()
{
    float s = sin(aAngle);
	float c = cos(aAngle);
	mat2 m = mat2(c, s, -s, c);
    vec2 pos = aOffset + m*(aPos*aSize);
    gl_Position = vec4((vec3(pos, 1.0) * view).xy, 0.0, 1.0);
    gl_Position.y *= -1.0;
    color = aColor;
    TexCoord = aUvRect.xy + aUv*aUvRect.zw;
    TextureIndex = aTextureIndex;
}
"#;

unsafe fn gl_check_error_(file: &str, line: u32, label: &str) -> u32 {
    let mut error_code = gl::GetError();
    while error_code != gl::NO_ERROR {
        let error = match error_code {
            gl::INVALID_ENUM => "INVALID_ENUM",
            gl::INVALID_VALUE => "INVALID_VALUE",
            gl::INVALID_OPERATION => "INVALID_OPERATION",
            gl::STACK_OVERFLOW => "STACK_OVERFLOW",
            gl::STACK_UNDERFLOW => "STACK_UNDERFLOW",
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

unsafe fn get_uniform_location(shader_program: u32, name: &str) -> i32 {
    let s = CString::new(name).unwrap();
    gl::GetUniformLocation(shader_program, s.as_ptr())
}

pub struct GLRenderer<'a> {
    render: &'a mut GLSpriteRender,
}
impl<'a> Renderer for GLRenderer<'a> {
    fn clear_screen(&mut self, color: &[f32; 4]) -> &mut dyn Renderer {
        log::trace!("clear screen to [{:5.3}, {:5.3}, {:5.3}, {:5.3}]", color[0], color[1], color[2], color[3]);
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
        if sprites.len() > self.render.instance_buffer_size as usize {
            self.render.reallocate_instance_buffer(sprites.len());
        }

        self.render.texture_unit_map.clear();
        unsafe {
            log::debug!("bind buffer {:x}", self.render.instance_buffer);
            // copy sprites into gpu memory
            gl::BindBuffer(gl::ARRAY_BUFFER, self.render.instance_buffer);
            loop {
                log::debug!("map buffer range");
                let mut cursor = gl::MapBufferRange(
                    gl::ARRAY_BUFFER,
                    0,
                    (sprites.len() * INSTANCE_STRIDE as usize) as GLsizeiptr,
                    gl::MAP_WRITE_BIT,
                ) as *mut u8;

                for sprite in sprites {
                    let texture_unit = if let Some(t) =
                        self.render.texture_unit_map.get(&sprite.texture)
                    {
                        *t
                    } else {
                        if self.render.texture_unit_map.len()
                            == self.render.max_texture_units as usize
                        {
                            unimplemented!("Split rendering in multiples draw calls when number of textures is greater than MAX_TEXTURE_IMAGE_UNITS is unimplemented.");
                        }
                        gl::ActiveTexture(gl::TEXTURE0 + self.render.texture_unit_map.len() as u32);
                        gl::BindTexture(gl::TEXTURE_2D, sprite.texture);
                        self.render
                            .texture_unit_map
                            .insert(sprite.texture, self.render.texture_unit_map.len() as u32);
                        self.render.texture_unit_map.len() as u32 - 1
                    };
                    ptr::copy_nonoverlapping(
                        mem::transmute(sprite),
                        cursor,
                        mem::size_of::<SpriteInstance>(),
                    );
                    ptr::copy_nonoverlapping(
                        mem::transmute(&texture_unit),
                        cursor.add(memoffset::offset_of!(SpriteInstance, texture)),
                        mem::size_of::<u32>(),
                    );
                    cursor = cursor.add(INSTANCE_STRIDE as usize);
                }

            log::debug!("unmap buffer");
                let mut b = gl::UnmapBuffer(gl::ARRAY_BUFFER) == gl::TRUE;
                b = gl::NO_ERROR
                    != gl_check_error!(
                        "instance_buffer_write({})",
                        (sprites.len() * mem::size_of::<SpriteInstance>()) as GLsizeiptr
                    )
                    || b;
                if b {
                    break;
                }
            }
            self.render.texture_unit_map.clear();
            log::debug!("unbind buffer {:x}", self.render.instance_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
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
            log::debug!("bind vertex");
            gl::BindVertexArray(self.render.vao());
            gl_check_error!("draw arrays instanced");
            log::debug!("draw");
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, sprites.len() as i32);

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
    vao: u32,
    _p: PhantomData<T>,
}
impl Context<NotCurrent> {
    unsafe fn make_current(self) -> Context<PossiblyCurrent> {
        let Self { context, vao, .. } = self;
        context.make_current();
        Context {
            context,
            vao,
            _p: Default::default(),
        }
    }
}
impl Context<PossiblyCurrent> {
    unsafe fn treat_as_not_current(self) -> Context<NotCurrent> {
        let Self { context, vao, .. } = self;
        Context {
            context,
            vao,
            _p: Default::default(),
        }
    }

    unsafe fn make_not_current(self) -> Context<NotCurrent> {
        let Self { context, vao, .. } = self;
        context.make_not_current();
        Context {
            context,
            vao,
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

pub struct GLSpriteRender {
    vsync: bool,
    contexts: HashMap<WindowId, Option<Context<NotCurrent>>>,
    current_context: Option<(WindowId, Context<PossiblyCurrent>)>,
    shader_program: u32,
    vertex_buffer: u32,
    instance_buffer: u32,
    instance_buffer_size: u32,
    textures: Vec<(u32, u32, u32)>, // id, width, height
    /// maps a texture to a texture unit
    texture_unit_map: HashMap<u32, u32>,
    max_texture_units: i32,
}
impl GLSpriteRender {
    /// Get a WindowBuilder and a event_loop (for opengl support), and return a window and Self.
    // TODO: build a better error handling!!!!
    pub fn new(window: &Window, vsync: bool) -> Result<Self, String> {
        let config = GlConfig {
            vsync,
            ..Default::default()
        };
        let context = unsafe {
            let context = GlContext::create(window, config).map_err(|x| format!("{:?}", x))?;
            context.make_current();
            context
        };

        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

        // unsafe {
        //     let mut num_extensions = 0;
        //     gl::GetIntegerv(gl::NUM_EXTENSIONS, &mut num_extensions);
        //     println!("extensions({}):", num_extensions);
        //     for i in 0..num_extensions as u32 {
        //         let string = gl::GetStringi(gl::EXTENSIONS, i);
        //         let string = CStr::from_ptr(string as *const i8);
        //         println!("{}", string.to_str().unwrap());
        //     }
        // }
        unsafe {
            let string = gl::GetString(gl::VERSION);
            let string = CStr::from_ptr(string as *const i8);
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

        let (shader_program, vao, instance_buffer, vertex_buffer) = unsafe {
            // Setup shader compilation checks
            let mut success = i32::from(gl::FALSE);
            let mut info_log = Vec::with_capacity(512);
            info_log.set_len(512 - 1); // -1 to skip trialing null character

            // Vertex shader
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let c_str_vert = CString::new(VERTEX_SHADER_SOURCE.as_bytes()).unwrap();
            gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
            gl::CompileShader(vertex_shader);

            // Check for shader compilation errors
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                let mut len = 0;
                gl::GetShaderInfoLog(
                    vertex_shader,
                    info_log.len() as i32,
                    &mut len,
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
                    str::from_utf8(&info_log[0..len as usize]).unwrap()
                );
            }

            // Fragment shader
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_str_frag = CString::new(
                format!(
                    r#"
#version 330 core
#define MAX_TEXTURE_IMAGE_UNITS {MAX_TEXTURE_IMAGE_UNITS}
out vec4 FragColor;

in vec4 color;
in vec2 TexCoord;
flat in int TextureIndex;

uniform sampler2D text[MAX_TEXTURE_IMAGE_UNITS];

void main()
{{
    vec4 textureColor;
    {textureColor} // textureColor = texture(text[TextureIndex], TexCoord);
    if (textureColor.a == 0.0 || color.a == 0.0) {{
        discard;
    }}
    FragColor = color*textureColor;
}}
"#,
                    MAX_TEXTURE_IMAGE_UNITS = max_texture_units,
                    textureColor = (0..max_texture_units)
                        .map(|i| format!("if (TextureIndex == {i}) textureColor = texture(text[{i}], TexCoord); \nelse ", i = i))
                        .chain(std::iter::once("textureColor = vec4(0.0);".to_string()))
                        .fold(String::new(), |a, b| a + &b)
                )
                .as_bytes(),
            )
            .unwrap();

            gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);

            // Check for shader compilation errors
            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success != i32::from(gl::TRUE) {
                let mut len = 0;
                gl::GetShaderInfoLog(
                    fragment_shader,
                    info_log.len() as i32,
                    &mut len,
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
                    str::from_utf8(&info_log[0..len as usize]).unwrap()
                );
            }

            // Link Shaders
            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader);
            gl::AttachShader(shader_program, fragment_shader);
            gl::LinkProgram(shader_program);

            // Check for linking errors
            gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
            if success != i32::from(gl::TRUE) {
                gl::GetProgramInfoLog(
                    shader_program,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
                    &String::from_utf8_lossy(&info_log)
                );
            }
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            // Set up vao and vbos

            // create instance buffer

            let mut instance_buffer = 0;

            gl::GenBuffers(1, &mut instance_buffer);

            // create vertex buffer
            let mut vertex_buffer = 0;

            gl::GenBuffers(1, &mut vertex_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTICES.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &VERTICES[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            let vao = Self::create_vao(vertex_buffer, instance_buffer);

            // Wireframe
            // gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);

            (shader_program, vao, instance_buffer, vertex_buffer)
        };

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
                    vao,
                    _p: Default::default(),
                },
            )),
            vertex_buffer,
            instance_buffer,
            instance_buffer_size: 0,
            textures: Vec::new(),
            texture_unit_map: HashMap::new(),
            max_texture_units,
        };
        let size = window.inner_size();
        sprite_render.resize(window.id(), size.width, size.height);

        Ok(sprite_render)
    }

    fn reallocate_instance_buffer(&mut self, size_need: usize) {
        let new_size = size_need.next_power_of_two();
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.instance_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (new_size * INSTANCE_STRIDE as usize) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            gl_check_error!(
                "reallocate_instance_buffer({})",
                (new_size * INSTANCE_STRIDE as usize) as GLsizeiptr
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
        self.instance_buffer_size = new_size as u32;
    }

    /// get vao from the current context
    fn vao(&self) -> u32 {
        self.current_context.as_ref().unwrap().1.vao
    }

    unsafe fn create_vao(vertex_buffer: u32, instance_buffer: u32) -> u32 {
        let mut vao = 0;

        gl::GenVertexArrays(1, &mut vao);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

        // 0, 1 are per vertex
        gl::EnableVertexAttribArray(0); // aPos
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, VERTEX_STRIDE, ptr::null());

        gl::EnableVertexAttribArray(1); // aUv
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            VERTEX_STRIDE,
            (mem::size_of::<f32>() * 2) as *const c_void,
        );

        // 2, 3, 4 are per instance
        gl::BindBuffer(gl::ARRAY_BUFFER, instance_buffer);

        gl::EnableVertexAttribArray(2); // aSize
        gl::VertexAttribPointer(
            2,
            2,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, scale) as *const c_void,
        );
        gl::VertexAttribDivisor(2, 1);

        gl::EnableVertexAttribArray(3); // aAngle
        gl::VertexAttribPointer(
            3,
            1,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, angle) as *const c_void,
        );
        gl::VertexAttribDivisor(3, 1);

        gl::EnableVertexAttribArray(4); // aUvRect
        gl::VertexAttribPointer(
            4,
            4,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, uv_rect) as *const c_void,
        );
        gl::VertexAttribDivisor(4, 1);

        gl::EnableVertexAttribArray(5); // aColor
        gl::VertexAttribPointer(
            5,
            4,
            gl::UNSIGNED_BYTE,
            gl::TRUE,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, color) as *const c_void,
        );
        gl::VertexAttribDivisor(5, 1);

        gl::EnableVertexAttribArray(6); // aOffset
        gl::VertexAttribPointer(
            6,
            2,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, pos) as *const c_void,
        );
        gl::VertexAttribDivisor(6, 1);

        gl::EnableVertexAttribArray(7); // aTextureIndex
        gl::VertexAttribIPointer(
            7,
            1,
            gl::INT,
            INSTANCE_STRIDE,
            memoffset::offset_of!(SpriteInstance, texture) as *const c_void,
        );
        gl::VertexAttribDivisor(7, 1);

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        vao
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
impl SpriteRender for GLSpriteRender {
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
        let context = unsafe {
            GlContext::create(window, config).unwrap()
        };

        let window_id = window.id();
        self.contexts.insert(
            window_id,
            Some(Context {
                context,
                vao: 0,
                _p: Default::default(),
            }),
        );

        self.set_current_context(window_id);

        unsafe {
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
        }

        self.current_context.as_mut().unwrap().1.vao =
            unsafe { Self::create_vao(self.vertex_buffer, self.instance_buffer) };
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
        Box::new(GLRenderer { render: self })
    }

    fn resize(&mut self, window_id: WindowId, width: u32, height: u32) {
        self.set_current_context(window_id);
        // self.current_context
        //     .as_ref()
        //     .unwrap()
        //     .1
        //     .resize(PhysicalSize::new(width, height));
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
        }
    }
}
