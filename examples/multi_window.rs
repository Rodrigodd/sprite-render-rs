use std::{collections::HashMap, fmt::Error, time::Instant};

use rand::Rng;
use sprite_render::{Camera, SpriteInstance, SpriteRender};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

struct Scene {
    instances: Box<[SpriteInstance]>,
    window: Window,
    camera: Camera,
    number_of_sprites: usize,
    sprite_size: f32,
    time: f32,
    do_anim: bool,
    do_rotation: bool,
    view_size: f32,
    dragging: bool,
    last_cursor_pos: PhysicalPosition<f32>,
    cursor_pos: PhysicalPosition<f32>,
}
impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), Error> {
        let mut builder = f.debug_struct("Scene");
        let _ = builder.field("number_of_sprites", &self.number_of_sprites);
        let _ = builder.field("sprite_size", &self.sprite_size);
        let _ = builder.field("time", &self.time);
        let _ = builder.field("do_anim", &self.do_anim);
        let _ = builder.field("do_rotation", &self.do_rotation);
        let _ = builder.field("view_size", &self.view_size);
        let _ = builder.field("dragging", &self.dragging);
        let _ = builder.field("last_cursor_pos", &self.last_cursor_pos);
        let _ = builder.field("cursor_pos", &self.cursor_pos);
        builder.finish()
    }
}
impl Scene {
    fn new(rng: &mut impl Rng, fruit_texture: u32, jelly_texture: u32, window: Window) -> Self {
        let camera = Camera::new(window.inner_size().width, window.inner_size().height, 2.0);
        let mut instances = vec![SpriteInstance::default(); 16384].into_boxed_slice();
        let sprite_size = 0.2;
        for i in (0..instances.len()).rev() {
            const COLORS: &[[u8; 4]] = &include!("colors.txt");
            const SPRITE: &[[f32; 4]] = &[
                [0.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
                [1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
                [2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
                [0.0, 1.0 / 2.0, 1.0 / 3.0, 1.0 / 2.0],
                [1.0 / 3.0, 1.0 / 2.0, 1.0 / 3.0, 1.0 / 2.0],
            ];

            instances[i] = SpriteInstance::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                sprite_size,
                sprite_size,
                if rng.gen() {
                    fruit_texture
                } else {
                    jelly_texture
                },
                SPRITE[i % SPRITE.len()],
            )
            .with_color(COLORS[i % 100]);
        }
        Self {
            camera,
            window,
            instances,
            time: 0.0,
            number_of_sprites: 100,
            sprite_size,
            do_anim: true,
            do_rotation: false,
            view_size: 2.0,
            dragging: false,
            last_cursor_pos: PhysicalPosition { x: 0.0f32, y: 0.0 },
            cursor_pos: PhysicalPosition { x: 0.0f32, y: 0.0 },
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(800.0, 400.0))
        .build(&event_loop)
        .unwrap();

    // create the SpriteRender
    let mut render = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "opengl")] {
                sprite_render::GLSpriteRender::new(&window, true).unwrap()
            } else {
                ()
            }
        }
    };

    let window_2 = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(800.0, 400.0))
        .build(&event_loop)
        .unwrap();
    render.add_window(&window_2);
    let fruit_texture = {
        let image = image::open("examples/fruits.png")
            .expect("File not Found!")
            .to_rgba8();
        render.new_texture(
            image.width(),
            image.height(),
            image.into_raw().as_slice(),
            true,
        )
    };
    let jelly_texture = {
        let image = image::open("examples/Jelly.png")
            .expect("File not Found!")
            .to_rgba8();
        render.new_texture(
            image.width(),
            image.height(),
            image.into_raw().as_slice(),
            true,
        )
    };

    let mut rng = rand::thread_rng();

    let mut clock = Instant::now();
    let mut change_clock = Instant::now();
    let mut change_frame = 0;
    let mut frame_count = 0;

    let mut scenes = HashMap::new();
    scenes.insert(
        window.id(),
        Scene::new(&mut rng, fruit_texture, jelly_texture, window),
    );
    scenes.insert(
        window_2.id(),
        Scene::new(&mut rng, fruit_texture, jelly_texture, window_2),
    );

    // let mut frame_clock = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, window_id } => {
                let scene = scenes.get_mut(&window_id).unwrap();
                match event {
                    WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                    WindowEvent::MouseWheel {delta, ..} => match delta {
                        MouseScrollDelta::LineDelta(_, dy) => {
                            scene.view_size *= 2.0f32.powf(-dy/3.0);
                            scene.camera.set_height(scene.view_size);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                        MouseScrollDelta::PixelDelta(PhysicalPosition{ y, ..}) => {
                            scene.view_size *= 2.0f32.powf(-y as f32/3.0);
                            scene.camera.set_height(scene.view_size);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state , ..} => {
                        scene.dragging = state == ElementState::Pressed;
                    },
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        scene.last_cursor_pos = scene.cursor_pos;
                        scene.cursor_pos.x = x as f32;
                        scene.cursor_pos.y = y as f32;
                        if scene.dragging {
                            let (dx,dy) = scene.camera.vector_to_word_space(
                                scene.last_cursor_pos.x - scene.cursor_pos.x,
                                scene.last_cursor_pos.y - scene.cursor_pos.y,
                            );
                            scene.camera.move_view(dx, dy);
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    },
                    WindowEvent::KeyboardInput { input: KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    }, ..} => match key {
                        VirtualKeyCode::Right => if scene.number_of_sprites < scene.instances.len() - 100 {
                            scene.number_of_sprites += 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Left => if scene.number_of_sprites > 100 {
                            scene.number_of_sprites -= 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Up => {
                            scene.sprite_size *= 1.1;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Down => {
                            scene.sprite_size *= 1.0/1.1;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::Space => {
                            scene.do_anim = !scene.do_anim;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        },
                        VirtualKeyCode::R => {
                            scene.do_rotation = !scene.do_rotation;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                        _ => ()
                    }
                    WindowEvent::Resized(size) => {
                        render.resize(window_id, size.width, size.height);
                        scene.camera.resize(size.width, size.height);
                    }
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                for (_window_id, scene) in scenes.iter_mut() {
                    if scene.do_anim {
                        scene.time += 1.0/180.0;
                        for i in 0..scene.number_of_sprites {
                            let a = ((i + 1)*(i + 3) % 777) as f32 + scene.time;
                            scene.instances[i].set_angle(a);
                            scene.instances[i].set_size(scene.sprite_size, scene.sprite_size);
                        }
                    }
                    if scene.do_rotation {
                        scene.camera.rotate_view(std::f32::consts::PI*2.0*(1.0/180.0)/30.0);
                    }
                    scene.window.request_redraw();
                }
            }
            Event::RedrawRequested(window_id) => {
                let scene = scenes.get_mut(&window_id).unwrap();
                // draw
                frame_count +=1;
                if frame_count % 60 == 0 {
                    let elapsed = clock.elapsed().as_secs_f32();
                    clock = Instant::now();
                    let fps = 60.0/elapsed;
                    let mean_fps = (frame_count - change_frame) as f32/change_clock.elapsed().as_secs_f32();
                    scene.window.set_title(&format!("SpriteRender | {:9.2} FPS ({:7.3} ms) | {} sprites with size {:.3} | mean: {:9.2} FPS ({:7.3} ms)",
                        fps, 1000.0 / fps,
                        scene.number_of_sprites, scene.sprite_size,
                        mean_fps, 1000.0 / mean_fps
                    ));
                }
                render.render(window_id)
                    .clear_screen(&[0.0f32, 0.0, 1.0, 1.0])
                    .draw_sprites(&mut scene.camera, &scene.instances[0..scene.number_of_sprites])
                    .finish();
            }
            _ => ()
        }
    });
}
