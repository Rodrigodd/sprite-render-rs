use wasm_bindgen::JsCast;
use web_sys::{WebGlProgram, WebGlRenderingContext, WebGlShader, WebGlBuffer, WebGlTexture};
use web_sys::console;

use winit::platform::web::WindowExtWebSys;

use winit::{
    window::Window,
    window::WindowBuilder,
    event_loop::EventLoop,
};

use std::mem;
use std::str;
use std::collections::HashMap;
use std::io::{ self, Write };


use crate::common::*;
use crate::SpriteRender;

const SPRITE_VERTEX_STRIDE: usize = mem::size_of::<f32>() * 6;

const VERTEX_SHADER_SOURCE: &str = r#"
attribute vec3 position;
attribute vec2 uv;
attribute vec4 aColor;
attribute float aTexture;

uniform mat3 view;

varying vec4 color;
varying vec2 TexCoord;
varying float textureIndex;

void main() {
    gl_Position = vec4(position*view, 1.0);
    gl_Position.y *= -1.0;
    color = aColor;
    TexCoord = uv;
    textureIndex = aTexture;
}
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
precision mediump float;

uniform sampler2D text[MAX_TEXTURE_IMAGE_UNITS];

varying vec4 color;
varying vec2 TexCoord;
varying float textureIndex;

void main() {
    int t = int(textureIndex);
    vec4 textureColor;
    for (int i = 0; i < MAX_TEXTURE_IMAGE_UNITS; i++ ) {
        if (i == t) textureColor = texture2D(text[i], TexCoord);
    }
    
    if (textureColor.a == 0.0 || color.a == 0.0) {
        discard;
    }
    gl_FragColor = textureColor*color;
}
"#;

unsafe fn transmute_slice<T, U>(slice: &[T]) -> &[U] {
    debug_assert!(mem::align_of::<T>() % mem::size_of::<U>() == 0, "T alignment must be multiple of U alignment");
    debug_assert!(mem::size_of::<T>() % mem::size_of::<U>() == 0, "T size must be multiple of U size");
    std::slice::from_raw_parts(slice.as_ptr() as *const T as *const U, slice.len() * mem::size_of::<T>() / mem::size_of::<U>())
}

fn gl_check_error_(context: &WebGlRenderingContext, file: &str, line: u32, label: &str) -> u32 {
    let mut error_code = context.get_error();
    while error_code != WebGlRenderingContext::NO_ERROR {
        let error = match error_code {
            WebGlRenderingContext::INVALID_ENUM => "INVALID_ENUM",
            WebGlRenderingContext::INVALID_VALUE => "INVALID_VALUE",
            WebGlRenderingContext::INVALID_OPERATION => "INVALID_OPERATION",
            WebGlRenderingContext::OUT_OF_MEMORY => "OUT_OF_MEMORY",
            WebGlRenderingContext::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
            _ => "unknown GL error code"
        };

        console::error_1(&format!("[{}:{:4}] {}: {}", file, line, label, error).into());

        error_code = context.get_error();
    }
    error_code
}

macro_rules! gl_check_error {
    ($context:expr,$($arg:tt)*) => (
        gl_check_error_($context, file!(), line!(), &format!($($arg)*))
    )
}

pub struct WebGLSpriteRender {
    context: WebGlRenderingContext,
    shader_program: WebGlProgram,
    textures: Vec<WebGlTexture>,
    buffer: WebGlBuffer,
    indice_buffer: WebGlBuffer,
    /// Buffer size in number of sprites
    buffer_size: u32,
    /// maps a texture to a texture unit
    texture_unit_map: HashMap<u32, u32>, 
    max_texture_units: i32,
}
impl WebGLSpriteRender {
    /// Get a WindowBuilder and a event_loop (for opengl support), and return a window and Self.
    pub fn new<T>(wb: WindowBuilder, event_loop: &EventLoop<T>) -> (Window, Self) {

        let window = wb.build(event_loop).unwrap();

        let canvas = window.canvas();

        let web_window = web_sys::window().unwrap();
        let document = web_window.document().unwrap();
        let body = document.body().unwrap();

        body.append_child(&canvas)
            .expect("Append canvas to HTML body");
        
        
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
        

        let context_options = js_sys::Object::new();
        js_sys::Reflect::set(&context_options, &"alpha".into(), &false.into()).unwrap();
        js_sys::Reflect::set(&context_options, &"premultipliedAlpha".into(), &false.into()).unwrap();

        let context = canvas
            .get_context_with_context_options("webgl", &context_options)
            .unwrap().unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        context.blend_func(WebGlRenderingContext::SRC_ALPHA, WebGlRenderingContext::ONE_MINUS_SRC_ALPHA);
        context.enable(WebGlRenderingContext::BLEND);
        
        let max_texture_units = context.get_parameter(WebGlRenderingContext::MAX_TEXTURE_IMAGE_UNITS)
            .unwrap().as_f64().unwrap() as i32;
        console::log_1(&format!("MAX_TEXTURE_IMAGE_UNITS: {}", max_texture_units).into());
        
        let vert_shader = Self::compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            VERTEX_SHADER_SOURCE,
        ).unwrap();
        let frag_shader = Self::compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            &(
                format!("#define MAX_TEXTURE_IMAGE_UNITS {}\n", max_texture_units)
                + FRAGMENT_SHADER_SOURCE
            ),
        ).unwrap();
        let shader_program = Self::link_program(&context, &vert_shader, &frag_shader).unwrap();
        context.use_program(Some(&shader_program));

        let indice_buffer = context.create_buffer().ok_or("failed to create buffer").unwrap();
        let buffer = context.create_buffer().ok_or("failed to create buffer").unwrap();
        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));

        let position = context.get_attrib_location(&shader_program, "position") as u32;
        context.vertex_attrib_pointer_with_i32(
            position, 2,
            WebGlRenderingContext::FLOAT, false,
            SPRITE_VERTEX_STRIDE as i32, 0
        );
        context.enable_vertex_attrib_array(position);

        let uv = context.get_attrib_location(&shader_program, "uv") as u32;
        context.vertex_attrib_pointer_with_i32(
            uv, 2,
            WebGlRenderingContext::FLOAT, false,
            SPRITE_VERTEX_STRIDE as i32, mem::size_of::<f32>() as i32 * 2
        );
        context.enable_vertex_attrib_array(uv);

        let a_color = context.get_attrib_location(&shader_program, "aColor") as u32;
        context.vertex_attrib_pointer_with_i32(
            a_color, 4,
            WebGlRenderingContext::UNSIGNED_BYTE, true,
            SPRITE_VERTEX_STRIDE as i32, mem::size_of::<f32>() as i32 * 4
        );
        context.enable_vertex_attrib_array(a_color);

        let a_texture = context.get_attrib_location(&shader_program, "aTexture") as u32;
        context.vertex_attrib_pointer_with_i32(
            a_texture, 1,
            WebGlRenderingContext::UNSIGNED_SHORT, false,
            SPRITE_VERTEX_STRIDE as i32, mem::size_of::<f32>() as i32 * 5
        );
        context.enable_vertex_attrib_array(a_texture);

        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);

        let mut sprite_render = Self {
            shader_program,
            context,
            buffer,
            indice_buffer,
            buffer_size: 0,
            textures: Vec::new(),
            texture_unit_map: HashMap::new(),
            max_texture_units,
        };
        let size = window.inner_size();
        sprite_render.resize(size.width, size.height);
    
        (
            window,
            sprite_render  
        )
    }

    fn compile_shader(
        context: &WebGlRenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, String> {
        let shader = context
            .create_shader(shader_type)
            .ok_or_else(|| String::from("Unable to create shader object"))?;
        context.shader_source(&shader, source);
        context.compile_shader(&shader);
    
        if context
            .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(context
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| String::from("Unknown error creating shader"))
                .replace("\\n", "\n")
            )
        }
    }

    fn link_program(
        context: &WebGlRenderingContext,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
    ) -> Result<WebGlProgram, String> {
        let program = context
            .create_program()
            .ok_or_else(|| String::from("Unable to create shader object"))?;
    
        context.attach_shader(&program, vert_shader);
        context.attach_shader(&program, frag_shader);
        context.link_program(&program);
    
        if context
            .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            Err(context
                .get_program_info_log(&program)
                .unwrap_or_else(|| String::from("Unknown error creating program object")))
        }
    }

    unsafe fn write_sprite<W: Write>(writer: &mut W, sprite: &SpriteInstance, texture: u16) -> io::Result<()> {
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
            - cos*width + sin*height + x,
            - sin*width - cos*height + y,
            u, v,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride
        
        // bottom right
        writer.write(&transmute_slice(&[
            cos*width + sin*height + x,
            sin*width - cos*height + y,
            u + w, v,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride
        
        // top left
        writer.write(&transmute_slice(&[
            - cos*width - sin*height + x,
            - sin*width + cos*height + y,
            u, v + h,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&texture.to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride

        // top right
        writer.write(&transmute_slice(&[
            cos*width - sin*height + x,
            sin*width + cos*height + y,
            u + w, v + h,
        ]))?;
        writer.write(&sprite.color)?;
        writer.write(&(texture as u16).to_ne_bytes())?;
        writer.write(&[0, 0])?; //complete the stride
        Ok(())
    }

    fn reallocate_instance_buffer(&mut self, size_need: usize) {
        let new_size = size_need.next_power_of_two();
        unsafe {
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.buffer));
            self.context.buffer_data_with_i32(
                WebGlRenderingContext::ARRAY_BUFFER,
                (new_size * SPRITE_VERTEX_STRIDE * 4) as i32,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );

            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);

            self.context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&self.indice_buffer));
            self.context.buffer_data_with_u8_array(
                WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
                transmute_slice(
                    &(0..(new_size * 6 * mem::size_of::<u16>()) as u16)
                        .map(|x| x/6*4 + [0u16, 1, 2, 1, 2, 3][x as usize % 6])
                        .collect::<Vec<u16>>()
                ),
                WebGlRenderingContext::DYNAMIC_DRAW,
            );

            self.context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, None);
            gl_check_error!(&self.context, "reallocate_instance_buffer({})", new_size * SPRITE_VERTEX_STRIDE * 4 as usize);
        }
        self.buffer_size = new_size as u32;
    }

}
impl SpriteRender for WebGLSpriteRender {
    /// Load a Texture in the GPU. if linear_filter is true, the texture will be sampled with linear filter applied.
    /// Pixel art don't use linear filter.
    fn load_texture(&mut self, width: u32, height: u32, data: &[u8], linear_filter: bool) -> u32 {
        self.context.active_texture(WebGlRenderingContext::TEXTURE0 + self.texture_unit_map.len() as u32);
        let texture = self.context.create_texture().unwrap();
        self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));
        self.context.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_WRAP_S,
            WebGlRenderingContext::CLAMP_TO_EDGE as i32
        );
        self.context.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_WRAP_T,
            WebGlRenderingContext::CLAMP_TO_EDGE as i32
        );
        self.context.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_MIN_FILTER,
            WebGlRenderingContext::LINEAR as i32
        );
        self.context.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_MAG_FILTER,
            if linear_filter {
                WebGlRenderingContext::LINEAR
            } else {
                WebGlRenderingContext::NEAREST
            } as i32
        );
        self.context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGlRenderingContext::TEXTURE_2D, 0,
            WebGlRenderingContext::RGBA as i32, width as i32, height as i32,
            0,
            WebGlRenderingContext::RGBA, WebGlRenderingContext::UNSIGNED_BYTE,
            Some(data)
        ).unwrap();
        self.textures.push(texture);
        self.textures.len() as u32
    }

    fn draw(&mut self, camera: &mut Camera, sprites: &[SpriteInstance]) {
        self.context.clear_color(0.0, 0.3, 0.0, 1.0);
        self.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        if sprites.len() > self.buffer_size as usize {
            self.reallocate_instance_buffer(sprites.len());
        }
        
        unsafe {
            let mut data: Vec<u8> = Vec::with_capacity(sprites.len() * SPRITE_VERTEX_STRIDE * 4);
            for sprite in sprites {
                let texture_unit = if let Some(t) = self.texture_unit_map.get(&sprite.texture) {
                    *t
                } else {
                    if self.texture_unit_map.len() == self.max_texture_units as usize {
                        unimplemented!("Split rendering in multiples draw calls when number of textures is greater than MAX_TEXTURE_IMAGE_UNITS is unimplemented.");
                    }
                    self.context.active_texture(WebGlRenderingContext::TEXTURE0 + self.texture_unit_map.len() as u32);
                    self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.textures[sprite.texture as usize - 1]));
                    self.texture_unit_map.insert(sprite.texture, self.texture_unit_map.len() as u32);
                    self.texture_unit_map.len() as u32-1
                };
                Self::write_sprite(&mut data, sprite, texture_unit as u16).unwrap();
            }
        
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.buffer));
            self.context.buffer_sub_data_with_i32_and_u8_array(
                WebGlRenderingContext::ARRAY_BUFFER, 0,
                &data
            );
        }

        // self.context.enable_vertex_attrib_array(0);
        // self.context.vertex_attrib_pointer_with_i32(0, 2, WebGlRenderingContext::FLOAT, false, SPRITE_VERTEX_STRIDE as i32, 0);
        // self.context.enable_vertex_attrib_array(1);
        // self.context.vertex_attrib_pointer_with_i32(1, 3, WebGlRenderingContext::FLOAT, false, SPRITE_VERTEX_STRIDE as i32, mem::size_of::<f32>() as i32 * 2);

        gl_check_error!(&self.context, "after write");

        self.context.uniform_matrix3fv_with_f32_array(
            self.context.get_uniform_location(&self.shader_program, "view").as_ref(),
            false, camera.view()
        );
        let text_units = (0..self.max_texture_units).collect::<Vec<i32>>();
        self.context.uniform1iv_with_i32_array(
            self.context.get_uniform_location(&self.shader_program, "text").as_ref(), 
            &text_units
        );

        self.context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&self.indice_buffer));

        gl_check_error!(&self.context, "pre draw");

        self.context.draw_elements_with_i32(
            WebGlRenderingContext::TRIANGLES,
            sprites.len() as i32 * 6,
            WebGlRenderingContext::UNSIGNED_SHORT,
            0,
        );
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);
        self.context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, None);
        gl_check_error!(&self.context, "end frame");
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.context.viewport(0, 0, width as i32, height as i32);
    }
}