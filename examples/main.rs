use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use sprite_render::{Camera, SpriteInstance, SpriteRender};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, Touch, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::EventLoop,
    window::WindowBuilder,
};
#[cfg(target_arch = "wasm32")]
mod time {
    pub use wasm_timer::Instant;
}
#[cfg(target_arch = "wasm32")]
use time::Instant;

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(
        backtrace = "on",
        ndk_glue = "ndk_glue",
        logger(
            level = "trace",
            tag = "sprite-render",
            filter = "main,raw_gl_context::android"
        )
    )
)]
pub fn main() {
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    env_logger::init();
    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    log::info!("starting main example!!");

    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(800.0f32, 400.0));

    #[cfg(target_arch = "wasm32")]
    let wb = {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;

        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("main_canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        wb.with_canvas(Some(canvas))
    };

    let window = wb.build(&event_loop).unwrap();

    // create the SpriteRender
    let mut render: Box<dyn SpriteRender> = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "opengl")] {
                Box::new(sprite_render::GLSpriteRender::new(&window, true)
                         .unwrap_or_else(|x| panic!("{}", x)))
            } else if #[cfg(feature = "opengles")] {
                Box::new(())
            } else if #[cfg(all(target_arch = "wasm32", feature = "webgl"))] {
                Box::new(sprite_render::WebGLSpriteRender::new(&window))
            } else {
                log::warn!("No sprite-render backend was choosen. \
                           Enable one of them by enabling a feature, like `--features=opengl`");
                Box::new(())
            }
        }
    };

    let mut camera = Camera::new(window.inner_size().width, window.inner_size().height, 2.0);

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut number_of_sprites = 100;
    let mut sprite_size = 0.2f32;

    let mut instances: Box<[SpriteInstance]> =
        vec![SpriteInstance::default(); 16384].into_boxed_slice();
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
            0,
            SPRITE[i % SPRITE.len()],
        )
        .with_color(COLORS[i % 100]);
    }

    #[cfg(not(target_arch = "android"))]
    create_textures(&mut render, &mut instances);

    let mut clock = Instant::now();
    let mut change_clock = Instant::now();
    let mut change_frame = 0;
    let mut frame_count = 0;

    let mut time = 0.0f32;
    let mut do_anim = true;
    let mut do_rotation = false;
    let mut dragging = false;
    let mut cursor_pos = PhysicalPosition { x: 0.0f32, y: 0.0 };
    #[cfg(target_os = "android")]
    let mut touchs: HashMap<u64, PhysicalPosition<f64>> = HashMap::new();

    // let mut frame_clock = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        // log::debug!("event: {:?}", event);
        match event {
            #[cfg(target_os = "android")]
            Event::Resumed => {
                log::info!("creating sprite-render");
                render = sprite_render::GlesSpriteRender::new(&window, true)
                    .map(|x| Box::new(x) as Box<dyn SpriteRender>)
                    .unwrap_or_else(|x| {
                        log::error!("initializing sprite-render failed: {:?}", x);
                        Box::new(())
                    });
                create_textures(&mut render, &mut instances);
            }
            #[cfg(target_os = "android")]
            Event::Suspended => {
                log::info!("destroying sprite-render");
                render = Box::new(());
            }
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                WindowEvent::MouseWheel { delta, .. } => {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, dy) => dy as f32,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { y: dy, .. }) => {
                            dy as f32 / 133.33
                        }
                    };
                    let scale = 2.0f32.powf(-dy as f32 / 3.0);

                    let (w, h) = {
                        let (w, h) = camera.screen_size();
                        (w as f32, h as f32)
                    };
                    let dx = (cursor_pos.x - w / 2.0) * (1.0 / scale - 1.0);
                    let dy = (cursor_pos.y - h / 2.0) * (1.0 / scale - 1.0);

                    camera.scale_view(scale);
                    let (dx, dy) = camera.vector_to_word_space(dx, dy);
                    camera.move_view(dx, dy);
                    change_clock = Instant::now();
                    change_frame = frame_count;
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    dragging = state == ElementState::Pressed;
                }
                #[cfg(target_os = "android")]
                WindowEvent::Touch(Touch {
                    location,
                    id,
                    phase,
                    ..
                }) => {
                    let length = |touchs: &HashMap<u64, PhysicalPosition<f64>>| {
                        if touchs.len() == 2 {
                            let mut iter = touchs.iter();
                            let (_, a) = iter.next().unwrap();
                            let (_, b) = iter.next().unwrap();
                            let dx = a.x - b.x;
                            let dy = a.y - b.y;
                            (dx.powi(2) + dy.powi(2)).sqrt() as f32
                        } else {
                            1.0
                        }
                    };

                    let last_l;

                    match phase {
                        TouchPhase::Started => {
                            log::debug!("touch insert");
                            touchs.insert(id, location);
                            last_l = length(&touchs);
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            log::debug!("touch remove");
                            touchs.remove(&id);
                            last_l = length(&touchs);
                        }
                        TouchPhase::Moved => {
                            last_l = length(&touchs);
                            touchs.insert(id, location);
                        }
                    }

                    let curr_l = length(&touchs);

                    let mean = |touchs: &HashMap<u64, PhysicalPosition<f64>>| {
                        touchs.iter().fold((0.0, 0.0), |(mx, my), (_, p)| {
                            (
                                mx + p.x as f32 / touchs.len() as f32,
                                my + p.y as f32 / touchs.len() as f32,
                            )
                        })
                    };
                    let (w, h) = {
                        let (w, h) = camera.screen_size();
                        (w as f32, h as f32)
                    };
                    let (x, y) = mean(&touchs);
                    let scale = curr_l / last_l;
                    let tx = w / 2.0 + (x - w / 2.0) / scale;
                    let ty = h / 2.0 + (y - h / 2.0) / scale;

                    match phase {
                        TouchPhase::Started | TouchPhase::Ended | TouchPhase::Cancelled => {
                            log::debug!("touch {} {}", x, y);
                            cursor_pos.x = x as f32;
                            cursor_pos.y = y as f32;
                        }
                        _ => {}
                    }

                    match phase {
                        TouchPhase::Moved => {
                            let last_cursor_pos = cursor_pos;
                            if dragging {
                                let dx = last_cursor_pos.x - tx;
                                let dy = last_cursor_pos.y - ty;
                                let (dx, dy) = camera.vector_to_word_space(dx, dy);
                                log::debug!("dragg {} {}", dx, dy);
                                camera.move_view(dx, dy);
                                camera.scale_view(1.0 / scale as f32);
                                change_clock = Instant::now();
                                change_frame = frame_count;
                            }
                        }
                        _ => {}
                    }

                    dragging = touchs.len() > 0;

                    cursor_pos.x = x as f32;
                    cursor_pos.y = y as f32;
                }
                WindowEvent::CursorMoved {
                    position: PhysicalPosition { x, y },
                    ..
                } => {
                    let last_cursor_pos = cursor_pos;
                    cursor_pos.x = x as f32;
                    cursor_pos.y = y as f32;
                    if dragging {
                        let (dx, dy) = camera.vector_to_word_space(
                            last_cursor_pos.x - cursor_pos.x,
                            last_cursor_pos.y - cursor_pos.y,
                        );
                        camera.move_view(dx, dy);
                        change_clock = Instant::now();
                        change_frame = frame_count;
                    }
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match key {
                    VirtualKeyCode::Right => {
                        if number_of_sprites < instances.len() - 100 {
                            number_of_sprites += 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    }
                    VirtualKeyCode::Left => {
                        if number_of_sprites > 100 {
                            number_of_sprites -= 100;
                            change_clock = Instant::now();
                            change_frame = frame_count;
                        }
                    }
                    VirtualKeyCode::Up => {
                        sprite_size *= 1.1;
                        change_clock = Instant::now();
                        change_frame = frame_count;
                    }
                    VirtualKeyCode::Down => {
                        sprite_size *= 1.0 / 1.1;
                        change_clock = Instant::now();
                        change_frame = frame_count;
                    }
                    VirtualKeyCode::Space => {
                        do_anim = !do_anim;
                        change_clock = Instant::now();
                        change_frame = frame_count;
                    }
                    VirtualKeyCode::R => {
                        do_rotation = !do_rotation;
                        change_clock = Instant::now();
                        change_frame = frame_count;
                    }
                    _ => (),
                },
                WindowEvent::Resized(size) => {
                    render.resize(window_id, size.width, size.height);
                    camera.resize(size.width, size.height);
                }
                _ => (),
            },

            Event::MainEventsCleared => {
                // log::debug!("events cleared!!");
                if do_anim {
                    // log::debug!("anim!!");
                    time += 1.0 / 180.0;
                    for i in 0..number_of_sprites {
                        let a = ((i + 1) * (i + 3) % 777) as f32 + time;
                        instances[i].set_angle(a);
                        instances[i].set_size(sprite_size, sprite_size);
                    }
                }
                if do_rotation {
                    use std::f32::consts::PI;
                    camera.rotate_view(PI * 2.0 * (1.0 / 180.0) / 30.0);
                }
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                // draw
                // log::debug!("draw!!");
                frame_count += 1;
                if frame_count % 60 == 0 {
                    let elapsed = clock.elapsed().as_secs_f32();
                    clock = Instant::now();
                    let fps = 60.0 / elapsed;
                    let mean_fps =
                        (frame_count - change_frame) as f32 / change_clock.elapsed().as_secs_f32();
                    let title = format!(
                        "SpriteRender | {:9.2} FPS ({:7.3} ms) | \
                                        {} sprites with size {:.3} | mean: {:9.2} FPS ({:7.3} ms)",
                        fps,
                        1000.0 / fps,
                        number_of_sprites,
                        sprite_size,
                        mean_fps,
                        1000.0 / mean_fps
                    );
                    window.set_title(&title);
                    log::debug!("{}", title);
                }
                // let elapsed = frame_clock.elapsed().as_secs_f32();
                // println!("elapsed: {:5.2}, sleep: {:5.2}", elapsed*1000.0, (1.0/60.0 - elapsed)*1000.0);
                // frame_clock = Instant::now();
                // if elapsed < 1.0/60.0 {
                // std::thread::sleep(Duration::from_secs_f32(1.0/60.0));
                render
                    .render(window.id())
                    .clear_screen(&[0.0f32, 0.0, 1.0, 1.0])
                    .draw_sprites(&mut camera, &instances[0..number_of_sprites])
                    .finish();
            }
            _ => (),
        }
    });
}

fn create_textures(render: &mut Box<dyn SpriteRender>, instances: &mut Box<[SpriteInstance]>) {
    let start = Instant::now();
    let fruit_texture = {
        // let image = image::open("examples/fruits.png")
        let image = image::load_from_memory(include_bytes!("fruits.png"))
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
        // let image = image::open("examples/Jelly.png")
        let image = image::load_from_memory(include_bytes!("Jelly.png"))
            .expect("File not Found!")
            .to_rgba8();
        render.new_texture(
            image.width(),
            image.height(),
            image.into_raw().as_slice(),
            true,
        )
    };
    log::info!("load textures in {:?}", start.elapsed());
    for (i, instance) in instances.iter_mut().enumerate() {
        instance.texture = if i % 2 == 0 {
            fruit_texture
        } else {
            jelly_texture
        };
    }
}
