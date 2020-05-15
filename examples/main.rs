use sprite_render::{ default_render, SpriteInstance, Camera };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState, MouseScrollDelta, MouseButton  },
    dpi::{ LogicalSize, LogicalPosition, PhysicalPosition }
};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn run() {
        super::main();
    }
}

fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(800.0, 400.0));
    
    // create the SpriteRender
    let (window, mut render) = default_render(wb, &events_loop, false);
    let fruit_texture = {
        let image = image::open("examples/fruits.png").expect("File not Found!").to_rgba();
        render.load_texture(image.width(), image.height(), image.into_raw().as_slice(), true)
    };
    let jelly_texture =   {
        let image = image::open("examples/jelly.png").expect("File not Found!").to_rgba();
        render.load_texture(image.width(), image.height(), image.into_raw().as_slice(), true)
    };

    let mut camera = Camera::new(window.inner_size(), 2.0);

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut number_of_sprites = 100;
    let mut sprite_size = 0.2f32;

    let mut instances: Box<[SpriteInstance]> = vec![SpriteInstance::default(); 16384].into_boxed_slice();
    for i in (0..instances.len()).into_iter().rev() {

        const COLORS: &[[u8; 4]] = &include!("colors.txt");
        const SPRITE: &[[f32; 4]] = &[
            [0.0, 0.0, 1.0/3.0, 1.0/2.0],
            [1.0/3.0, 0.0, 1.0/3.0, 1.0/2.0],
            [2.0/3.0, 0.0, 1.0/3.0, 1.0/2.0],
            [0.0, 1.0/2.0, 1.0/3.0, 1.0/2.0],
            [1.0/3.0, 1.0/2.0, 1.0/3.0, 1.0/2.0],
        ];

        instances[i] = SpriteInstance::new(
            rng.gen_range(-1.0, 1.0),
            rng.gen_range(-1.0, 1.0),
            sprite_size,
            sprite_size,
            if rng.gen() { fruit_texture } else { jelly_texture },
            SPRITE[i%SPRITE.len()]
        ).with_color(COLORS[i%100]);

    }

    use std::time::{ Instant };
    let mut clock = Instant::now();
    let mut change_clock = Instant::now();
    let mut change_frame = 0;
    let mut frame_count = 0;
    let mut fps  = 60.0;
    
    let mut time = 0.0f32;
    let mut do_anim = true;
    let mut do_rotation = false;
    let mut view_size = 2.0;
    let mut dragging = false;
    let mut last_cursor_pos = PhysicalPosition{ x: 0.0f32, y:0.0 };
    let mut cursor_pos = PhysicalPosition{ x: 0.0f32, y:0.0 };

    // let mut frame_clock = Instant::now();
    
    events_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                    WindowEvent::MouseWheel {delta, ..} => match delta {
                        MouseScrollDelta::LineDelta(_, dy) => {
                            view_size *= 2.0f32.powf(-dy/3.0);
                            camera.set_height(view_size);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                        MouseScrollDelta::PixelDelta(LogicalPosition{ y, ..}) => {
                            view_size *= 2.0f32.powf(-y as f32/3.0);
                            camera.set_height(view_size);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state , ..} => {
                        dragging = state == ElementState::Pressed;
                    },
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        last_cursor_pos = cursor_pos;
                        cursor_pos.x = x as f32;
                        cursor_pos.y = y as f32;
                        if dragging {
                            let (dx,dy) = camera.vector_to_word_space(
                                last_cursor_pos.x - cursor_pos.x,
                                last_cursor_pos.y - cursor_pos.y,
                            );
                            camera.move_view(dx, dy);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    },
                    WindowEvent::KeyboardInput { input: KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    }, ..} => match key {
                        VirtualKeyCode::Right => if number_of_sprites < instances.len() - 100 {
                            number_of_sprites = number_of_sprites + 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Left => if number_of_sprites > 100 {
                            number_of_sprites = number_of_sprites - 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Up => {
                            sprite_size *= 1.1;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Down => {
                            sprite_size *= 1.0/1.1;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Space => {
                            do_anim = !do_anim;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::R => {
                            do_rotation = !do_rotation;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                        _ => ()
                    }
                    WindowEvent::Resized(size) => {
                        render.resize(size.width, size.height);
                        camera.resize(size);
                    }
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                if do_anim {
                    time += 1.0/180.0;
                    for i in 0..number_of_sprites {
                        let a = ((i + 1)*(i + 3) % 777) as f32 + time;
                        instances[i].set_angle(a);
                        instances[i].set_size(sprite_size, sprite_size);
                    }
                }
                if do_rotation {
                    camera.rotate_view(std::f32::consts::PI*2.0*(1.0/180.0)/30.0);
                }
                window.request_redraw();
            }
            
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // draw
                frame_count +=1;
                if frame_count % 60 == 0 {
                    let elapsed = clock.elapsed().as_secs_f32();
                    clock = Instant::now();
                    fps = 60.0/elapsed;
                    let mean_fps = (frame_count - change_frame) as f32/change_clock.elapsed().as_secs_f32();
                    window.set_title(&format!("SpriteRender | {:9.2} FPS ({:7.3} ms) | {} sprites with size {:.3} | mean: {:9.2} FPS ({:7.3} ms)",
                        fps, 1000.0 / fps,
                        number_of_sprites, sprite_size,
                        mean_fps, 1000.0 / mean_fps
                    ));
                }
                // let elapsed = frame_clock.elapsed().as_secs_f32();
                // println!("elapsed: {:5.2}, sleep: {:5.2}", elapsed*1000.0, (1.0/60.0 - elapsed)*1000.0);
                // frame_clock = Instant::now();
                // if elapsed < 1.0/60.0 {
                // std::thread::sleep(Duration::from_secs_f32(1.0/60.0));
                render.draw(&mut camera, &instances[0..number_of_sprites]);
            }
            _ => ()
        }
    });
}