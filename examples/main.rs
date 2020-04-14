use sprite_render::{ SpriteRender, SpriteInstance };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState },
};

fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 800.0));
    
    // create the SpriteRender
    let (window, mut render) = SpriteRender::new(wb, &events_loop);
    let fruit_texture = {
        let image = image::open("examples/fruits.png").expect("File not Found!").to_rgba();
        render.load_texture(image.width(), image.height(), image.into_raw().as_slice())
    };
    let jelly_texture =   {
        let image = image::open("examples/jelly.png").expect("File not Found!").to_rgba();
        render.load_texture(image.width(), image.height(), image.into_raw().as_slice())
    };

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut number_of_sprites = 100;
    let mut sprite_size = 0.2f32;

    let mut instances: Box<[SpriteInstance]> = vec![SpriteInstance::default(); 16384].into_boxed_slice();
    for i in (0..instances.len()).into_iter().rev() {

        const COLORS: &[[f32; 4]] = &include!("colors.txt");
        const SPRITE: &[[f32; 4]] = &[
            [0.0, 0.0, 1.0/3.0, 1.0/2.0],
            [1.0/3.0, 0.0, 1.0/3.0, 1.0/2.0],
            [2.0/3.0, 0.0, 1.0/3.0, 1.0/2.0],
            [0.0, 1.0/2.0, 1.0/3.0, 1.0/2.0],
            [1.0/3.0, 1.0/2.0, 1.0/3.0, 1.0/2.0],
        ];

        instances[i] = SpriteInstance {
            transform: [ 1.0, 0.0,
                         0.0, 1.0 ],
            uv_rect: SPRITE[i%SPRITE.len()],
            color: COLORS[i%100],
            pos: [rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)],
            texture: if rng.gen() { fruit_texture } else { jelly_texture },
            _padding : [0.0; 1],
        };
    }

    let mut time = 0.0f32;
    use std::time::{ Instant };
    let mut clock = Instant::now();
    let mut change_clock = Instant::now();
    let mut change_frame = 0;
    let mut frame_count = 0;
    let mut fps  = 60.0;
    let mut do_anim = true;

    // let mut frame_clock = Instant::now();
    
    events_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
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
                        _ => ()
                    }
                    WindowEvent::Resized(size) => render.resize(size),
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                if do_anim {
                    time += 1.0/180.0;
                    for i in 0..instances.len() {
                        let a = ((i + 1)*(i + 3) % 777) as f32 + time;
                        instances[i].transform = 
                            [ a.cos()*sprite_size, a.sin()*sprite_size,
                            -a.sin()*sprite_size, a.cos()*sprite_size];
                    }
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
                    window.set_title(&format!("SpriteRender | {:9.2} FPS | {} sprites with size {:.3} | mean: {:9.2} FPS",
                        fps, number_of_sprites, sprite_size, (frame_count - change_frame) as f32/change_clock.elapsed().as_secs_f32())
                    );
                }
                // let elapsed = frame_clock.elapsed().as_secs_f32();
                // println!("elapsed: {:5.2}, sleep: {:5.2}", elapsed*1000.0, (1.0/60.0 - elapsed)*1000.0);
                // frame_clock = Instant::now();
                // if elapsed < 1.0/60.0 {
                // std::thread::sleep(Duration::from_secs_f32(1.0/60.0));
                render.draw(&instances[0..number_of_sprites]);
            }
            _ => ()
        }
    });
}