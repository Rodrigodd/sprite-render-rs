use glutin::{
    window::Window,
    window::WindowBuilder,
    event_loop::EventLoop,
    dpi::PhysicalSize,
};
use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::os::raw::c_void;
use std::ffi::CString;

const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aUv;

layout (location = 2) in vec4 aTransform;
layout (location = 3) in vec4 aUvRect;
layout (location = 4) in vec4 aColor;
layout (location = 5) in vec2 aOffset;
out vec3 color;
void main()
{
    // vec2 pos = mat2(aTransform.xy, aTransform.wz)*aPos;
    vec2 pos = aPos*mat2(aTransform.xy, aTransform.zw);
    gl_Position = vec4(pos.x + aOffset.x, pos.y + aOffset.y, 0.0, 1.0);
	color = aColor.xyz; // pass the color along to the fragment shader
}
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core
out vec4 FragColor;
in vec3 color;
void main()
{
   // Set the fragment color to the color passed from the vertex shader
   FragColor = vec4(color, 1.0);
}
"#;

pub struct SpriteRender {
    context: glutin::RawContext<glutin::PossiblyCurrent>,
    shader_program: u32,
    vao: u32,
}
impl SpriteRender {
    /// Get a WindowBuilder and a event_loop (for opengl support), and return a window and Self.
    pub fn new<T>(wb: WindowBuilder, event_loop: &EventLoop<T>) -> (Window, Self) {
        let (context, window) = unsafe {
                glutin::ContextBuilder::new()
                    .build_windowed(wb, event_loop)
                    .unwrap()
                    .split()
            };
    
        let context = unsafe {
            context.make_current().unwrap()
        };
    
        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);
    
        let (shader_program, vao) = unsafe {
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
            if success != i32::from(gl::TRUE) {
                gl::GetShaderInfoLog(
                    vertex_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
                    str::from_utf8(&info_log).unwrap()
                );
            }
    
            // Fragment shader
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_str_frag = CString::new(FRAGMENT_SHADER_SOURCE.as_bytes()).unwrap();
            gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);
    
            // Check for shader compilation errors
            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success != i32::from(gl::TRUE) {
                gl::GetShaderInfoLog(
                    fragment_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
                    str::from_utf8(&info_log).unwrap()
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
                println!(
                    "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
                    str::from_utf8(&info_log).unwrap()
                );
            }
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
    
            // Set up vao and vbos
            let vertices: [f32; 16] = [
                // bottom left
                -0.5, -0.5,   0.0, 1.0,
    
                // bottom right
                 0.5, -0.5,   1.0, 1.0,
    
                // top left
                -0.5,  0.5,   0.0, 0.0,

                // top right
                 0.5,  0.5,   1.0, 0.0,
            ];

            #[repr(C)]
            #[derive(Default)]
            struct Instance {
                transform: [f32; 4],
                uv_rect: [f32; 4],
                color: [f32; 4],
                pos: [f32; 2],
            }


            // create instance buffer
            let mut instances: [Instance; 100] = mem::zeroed();
            let mut i = 0;
            for y in -5..5 {
                for x in -5..5 {
                    // let a = i as f32 * std::f32::consts::PI / 4.0;
                    let a = ((i  +1)*(i + 3)) as f32;
                    let r = 0.1;

                    const COLORS: &[[f32; 4]] = &include!("colors.txt");

                    instances[i] = Instance {
                        transform: [ a.cos()*r, a.sin()*r,
                                    -a.sin()*r, a.cos()*r],
                        uv_rect: [0.0, 0.0, 1.0, 1.0],
                        color: COLORS[i],
                        pos: [x as f32 / 5.0 + 0.1,
                              y as f32 / 5.0 + 0.1],
                    };
                    i += 1;
                }
            }

            let mut instance_buffer = 0;

            gl::GenBuffers(1, &mut instance_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, instance_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (instances.len() * mem::size_of::<Instance>()) as GLsizeiptr,
                &instances[0] as *const _ as *const c_void,
                gl::STATIC_DRAW
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            

            // create vertex buffer
            let mut vertex_buffer = 0;
            let mut vao = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vertex_buffer);
            
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &vertices[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,

            );
            const VERTEX_STRIDE: GLsizei = 4 * mem::size_of::<GLfloat>() as GLsizei;
            const INSTANCE_STRIDE: GLsizei = 14 * mem::size_of::<GLfloat>() as GLsizei;
            
            // 0, 1 are per vertex
            gl::EnableVertexAttribArray(0); // aPos
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                VERTEX_STRIDE,
                ptr::null(),
            );

            gl::EnableVertexAttribArray(1); // aUv
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                VERTEX_STRIDE,
                (2 * mem::size_of::<GLfloat>()) as *const c_void
            );

            // 2, 3, 4 are per instance
            gl::BindBuffer(gl::ARRAY_BUFFER, instance_buffer);

            gl::EnableVertexAttribArray(2); // aTransform
            gl::VertexAttribPointer(
                2,
                4,
                gl::FLOAT,
                gl::FALSE,
                INSTANCE_STRIDE,
                0 as *const c_void
            );
            gl::VertexAttribDivisor(2, 1);

            gl::EnableVertexAttribArray(3); // aUvRect
            gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                INSTANCE_STRIDE,
                (4 * mem::size_of::<GLfloat>()) as *const c_void
            );
            gl::VertexAttribDivisor(3, 1);

            gl::EnableVertexAttribArray(4); // aColor
            gl::VertexAttribPointer(
                4,
                4,
                gl::FLOAT,
                gl::FALSE,
                INSTANCE_STRIDE,
                (8 * mem::size_of::<GLfloat>()) as *const c_void
            );
            gl::VertexAttribDivisor(4, 1);

            gl::EnableVertexAttribArray(5); // aOffset
            gl::VertexAttribPointer(
                5,
                2,
                gl::FLOAT,
                gl::FALSE,
                INSTANCE_STRIDE,
                (12 * mem::size_of::<GLfloat>()) as *const c_void
            );
            gl::VertexAttribDivisor(5, 1);
    
    
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
    
            // Wireframe
            // gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
    
            (shader_program, vao)
        };
    
        (
            window,
            SpriteRender {
                shader_program,
                vao,
                context,
            }
        )
    }

    pub fn draw(&self) {
        unsafe {
            gl::ClearColor(0.39, 0.58, 0.92, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.shader_program);
            gl::BindVertexArray(self.vao);
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, 100);
        }

        self.context.swap_buffers().unwrap();
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        self.context.resize(size);
    }
}