use sprite_render::SpriteRender;

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent },
};


fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
    
    let (window, render) = SpriteRender::new(wb, &events_loop);

    events_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                    WindowEvent::Resized(size) => render.resize(size),
                    _ => (),
                }

                render.draw();
            },
            _ => ()
        }
    });
}