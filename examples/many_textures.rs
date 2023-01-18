use sprite_render::{Camera, SpriteInstance, SpriteRender, Texture, TextureId};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, Touch, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::EventLoop,
    window::WindowBuilder,
};

pub fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting main example!!");

    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(600.0f32, 600.0));
    let window = wb.build(&event_loop).unwrap();

    // create the SpriteRender
    let mut render: Box<dyn SpriteRender> = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "opengl")] {
                Box::new(sprite_render::GlSpriteRender::new(&window, true).unwrap())
            } else if #[cfg(all(target_arch = "wasm32", feature = "webgl"))] {
                Box::new(sprite_render::WebGLSpriteRender::new(&window))
            } else {
                log::warn!("No sprite-render backend was choosen. Enable one of them by enabling a feature, like `--features=opengl`");
                Box::new(sprite_render::NoopSpriteRender)
            }
        }
    };

    let mut camera = Camera::new(window.inner_size().width, window.inner_size().height, 2.0);

    use rand::Rng;
    let mut rng = rand::thread_rng();

    let sprite_size = 0.2;

    let count = std::env::args()
        .nth(1)
        .map_or(10, |x| x.parse::<usize>().unwrap());

    let mut instances: Box<[SpriteInstance]> =
        vec![SpriteInstance::default(); count].into_boxed_slice();
    for i in (0..instances.len()).rev() {
        const COLORS: &[[u8; 4]] = &include!("colors.txt");
        const SPRITE: &[[f32; 4]] = &[
            [0.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
            [1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
            [2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0 / 2.0],
            [0.0, 1.0 / 2.0, 1.0 / 3.0, 1.0 / 2.0],
            [1.0 / 3.0, 1.0 / 2.0, 1.0 / 3.0, 1.0 / 2.0],
        ];

        let color = COLORS[i % COLORS.len()];
        let texture = Texture::new(1, 1)
            .data(&color)
            .create(render.as_mut())
            .unwrap();

        instances[i] = SpriteInstance::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            sprite_size,
            sprite_size,
            texture,
            SPRITE[i % SPRITE.len()],
        );
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,

                WindowEvent::Resized(size) => {
                    render.resize(window_id, size.width, size.height);
                    camera.resize(size.width, size.height);
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                render
                    .render(window.id())
                    .clear_screen(&[0.0f32, 0.0, 1.0, 1.0])
                    .draw_sprites(&mut camera, &instances)
                    .finish();
            }
            _ => (),
        }
    });
}
