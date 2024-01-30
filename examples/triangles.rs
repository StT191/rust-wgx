
use std::sync::Arc;
use std::{time::{Instant}};
use pollster::FutureExt;
use winit::{
    event_loop::{ControlFlow, EventLoop}, dpi::PhysicalSize,
    window::Window, event::{Event, WindowEvent, KeyEvent, ElementState},
    keyboard::{PhysicalKey, KeyCode},
};
use wgx::{*};


fn main() {

    const DEPTH_TESTING:bool = true;
    const MSAA:u32 = 4;
    const BLENDING:Option<Blend> = Some(Blend::ALPHA_BLENDING);


    let event_loop = EventLoop::new().unwrap();

    let window = Arc::new(Window::new(&event_loop).unwrap());
    let _ = window.request_inner_size(PhysicalSize::<u32>::from((1200, 1200)));
    window.set_title("WgFx");


    let (gx, surface) = Wgx::new(Some(window.clone()), features!(), limits!{}).block_on().unwrap();
    let mut target = SurfaceTarget::new(&gx, surface.unwrap(), (1200, 1200), MSAA, DEPTH_TESTING).unwrap();


    // global pipeline
    let shader = gx.load_wgsl(wgsl_modules::include!("common/shaders/shader_flat_text.wgsl"));

    let pipeline = target.render_pipeline(&gx,
        None, &[vertex_dsc!(Vertex, 0 => Float32x3, 1 => Float32x2)],
        (&shader, "vs_main", Primitive::default()),
        (&shader, "fs_main", BLENDING),
    );

    // colors
    let texture = TextureLot::new_2d_with_data(&gx,
        (2, 1), 1, DEFAULT_SRGB, None, /*TexUse::COPY_SRC |*/ TexUse::TEXTURE_BINDING,
        [[255u8, 0, 0, 255], [0, 0, 255, 50]]
    );

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Vtx([f32;3], [f32;2]);
    unsafe impl wgx::ReadBytes for Vtx {}

    // vertices
    let data = [
        Vtx([-0.25, -0.5, 0.35f32], [0.0, 0.0f32]),
        Vtx([0.0, -0.5, 0.35], [1.0, 0.0]),
        Vtx([-1.0, 0.5, 0.1], [0.0, 0.0]),

        Vtx([0.25, -0.5, 0.1], [0.0, 0.0]),
        Vtx([0.5, -0.5, 0.1], [1.0, 0.0]),
        Vtx([-1.0, 0.5, 0.6], [0.0, 0.0]),

        Vtx([-0.75, -0.5, 0.1], [0.0, 0.0]),
        Vtx([-1.0, -0.5, 0.1], [1.0, 0.0]),
        Vtx([-0.3, 0.5, 0.312], [1.0, 0.0]),
    ];
    let vertices = gx.buffer_from_data(BufUse::VERTEX, &data[..]);


    // texture + sampler
    let sampler = gx.default_sampler();

    let binding = gx.bind(&pipeline.get_bind_group_layout(0), &[
        bind!(0, TextureView, &texture.view),
        bind!(1, Sampler, &sampler),
    ]);


    // render bundles
    let bundles = [target.render_bundle(&gx, |rpass| {
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &binding, &[]);
        rpass.set_vertex_buffer(0, vertices.slice(..));
        rpass.draw(0..data.len() as u32, 0..1);
    })];

    // event loop

    event_loop.run(move |event, event_target| {

        event_target.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {event: WindowEvent::CloseRequested, ..} => {
                event_target.exit();
            },

            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                target.update(&gx, (size.width, size.height));
            },

            Event::WindowEvent { event: WindowEvent::KeyboardInput { event: KeyEvent {
                state: ElementState::Pressed, physical_key: PhysicalKey::Code(KeyCode::KeyR), ..
            }, ..}, ..} => {
                window.request_redraw();
            },

            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {

                let then = Instant::now();

                target.with_frame(None, |frame| gx.with_encoder(|encoder| {
                    encoder.pass_bundles(frame.attachments(Some(Color::GREEN), Some(1.0)), &bundles);
                })).expect("frame error");

                println!("{:?}", then.elapsed());
            },

            _ => {}
        }
    }).unwrap();
}