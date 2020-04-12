use sprite_render::{ SpriteRender, SpriteInstance };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState },
};

use std::mem;


fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 800.0));
    
    // create the SpriteRender
    let (window, mut render) = SpriteRender::new(wb, &events_loop);

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut n = 100;

    let mut instances: Box<[SpriteInstance]> = vec![SpriteInstance::default(); 16384].into_boxed_slice();
    for i in (0..instances.len()).into_iter().rev() {

        const COLORS: &[[f32; 4]] = &include!("colors.txt");

        instances[i] = SpriteInstance {
            transform: [ 1.0, 0.0,
                         0.0, 1.0 ],
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            color: COLORS[i%100],
            pos: [rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)],
            _padding : [0.0; 2],
        };
    }

    let mut time = 0.0f32;
    use std::time::{ Instant, Duration };
    let mut clock = Instant::now();
    let mut frame_count = 0;
    let mut fps  = 60.0;

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
                        VirtualKeyCode::Right => if n < instances.len() - 100 { n = n + 100; },
                        VirtualKeyCode::Left => if n > 100 { n = n - 100; },
                        _ => ()
                    }
                    WindowEvent::Resized(size) => render.resize(size),
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                time += 1.0/60.0;
                for i in 0..instances.len() {
                    let a = ((i + 1)*(i + 3) % 777) as f32 + time;
                    let r = 0.1;
                    instances[i].transform = 
                        [ a.cos()*r, a.sin()*r,
                         -a.sin()*r, a.cos()*r];
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
                    window.set_title(&format!("SpriteRender | {:9.2} FPS", fps));
                    println!("{}: {}", frame_count, elapsed);
                }
                // let elapsed = frame_clock.elapsed().as_secs_f32();
                // println!("elapsed: {:5.2}, sleep: {:5.2}", elapsed*1000.0, (1.0/60.0 - elapsed)*1000.0);
                // frame_clock = Instant::now();
                // if elapsed < 1.0/60.0 {
                // std::thread::sleep(Duration::from_secs_f32(1.0/60.0));
                render.draw(&instances[0..n]);
            }
            _ => ()
        }
    });
}