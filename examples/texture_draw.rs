
use std::sync::Arc;
use std::{time::{Instant}};
use pollster::FutureExt;
use winit::{
    event_loop::{ControlFlow, EventLoop}, dpi::PhysicalSize,
    window::Window, event::{Event, WindowEvent, KeyEvent, ElementState},
    keyboard::{PhysicalKey, KeyCode},
};
use wgx::*;


fn main() {

    const DEPTH_TESTING:bool = false;
    const MSAA:u32 = 4;
    const BLENDING:Option<Blend> = None;


    let event_loop = EventLoop::new().unwrap();

    let window = Arc::new(Window::new(&event_loop).unwrap());

    // size
    let sf = window.scale_factor() as f32;

    let width = (sf * 800.0) as u32;
    let height = (sf * 600.0) as u32;

    let _ = window.request_inner_size(PhysicalSize::<u32>::from((width, height)));
    window.set_title("WgFx");


    let (gx, surface) = Wgx::new(Some(window.clone()), features!(), limits!{}).block_on().unwrap();
    let mut target = SurfaceTarget::new(&gx, surface.unwrap(), (width, height), MSAA, DEPTH_TESTING).unwrap();


    // common/shaders
    let shader = gx.load_wgsl(wgsl_modules::include!("common/shaders/shader_flat_text.wgsl"));

    // pipeline
    let pipeline = target.render_pipeline(&gx,
        None, &[vertex_dsc!(Vertex, 0 => Float32x3, 1 => Float32x2)],
        (&shader, "vs_main", Primitive { topology: Topology::TriangleStrip, ..Primitive::default() }),
        (&shader, "fs_main", BLENDING),
    );

    // sampler
    let sampler = gx.default_sampler();

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Vtx([f32;3], [f32;2]);
    unsafe impl wgx::ReadBytes for Vtx {}

    // vertices
    let vertex_data = [
        Vtx([ 0.5,  0.5, 0.0f32], [1.0, 0.0f32]),
        Vtx([-0.5,  0.5, 0.0], [0.0, 0.0]),
        Vtx([ 0.5, -0.5, 0.0], [1.0, 1.0]),
        Vtx([-0.5, -0.5, 0.0], [0.0, 1.0]),
    ];
    let vertices = gx.buffer_from_data(BufUse::VERTEX, &vertex_data[..]);


    // colors
    let color_texture = TextureLot::new_2d_with_data(&gx,
        (1, 1), 1, DEFAULT_SRGB, None, TexUse::TEXTURE_BINDING,
        // Color::from([0.5, 0.0, 1.0]).u8(),
        Color::ORANGE.u8(),
    );

    let bg_color_draw_target = Color::ORANGE;
    let bg_color_target = Color::ORANGE;


    // draw target
    const DRAW_MSAA:u32 = 4;

    let draw_target = TextureTarget::new(&gx, (width, height), DRAW_MSAA, false, DEFAULT_SRGB, None, TexUse::TEXTURE_BINDING);
    // let draw_target2 = TextureTarget::new(&gx, (width, height), DRAW_MSAA, false, DEFAULT_SRGB, None, TexUse::TEXTURE_BINDING);

    let draw_pipeline = gx.render_pipeline(
        DRAW_MSAA, None, None,
        &[vertex_dsc!(Vertex, 0 => Float32x3, 1 => Float32x2)],
        (&shader, "vs_main", Primitive { topology: Topology::TriangleStrip, ..Primitive::default() }),
        Some((&shader, "fs_main", &[
            (draw_target.view_format(), BLENDING),
            // (draw_target2.view_format(), BLENDING),
        ])),
    );

    let draw_binding = gx.bind(&draw_pipeline.get_bind_group_layout(0), &[
        bind!(0, TextureView, &color_texture.view),
        bind!(1, Sampler, &sampler),
    ]);

    target.with_frame(None, |frame| gx.with_encoder(|encoder| { // !! ecoder witout draw to attachment produces hang!

        encoder.with_render_pass(
            (
                [
                    Some(draw_target.color_attachment(Some(bg_color_draw_target))),
                    // Some(draw_target2.color_attachment(Some(bg_color_draw_target))),
                ],
                None
            ),
            |rpass| {
                rpass.set_pipeline(&draw_pipeline);
                rpass.set_bind_group(0, &draw_binding, &[]);
                rpass.set_vertex_buffer(0, vertices.slice(..));
                rpass.draw(0..vertex_data.len() as u32, 0..1);
            }
        );

        encoder.render_pass(frame.attachments(Some(bg_color_target), None));

    })).expect("frame error");


    // binding
    let binding = gx.bind(&pipeline.get_bind_group_layout(0), &[
        bind!(0, TextureView, &draw_target.view),
        bind!(1, Sampler, &sampler),
    ]);

    // event loop

    event_loop.run(move |event, event_target| {

        event_target.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {event: WindowEvent::CloseRequested, ..} => {
                event_target.exit();
            },

            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                let size = (size.width, size.height);
                target.update(&gx, size);
            },

            Event::WindowEvent { event: WindowEvent::KeyboardInput { event: KeyEvent {
                state: ElementState::Pressed, physical_key: PhysicalKey::Code(KeyCode::KeyR), ..
            }, ..}, ..} => {
                window.request_redraw();
            },

            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {


                let then = Instant::now();

                target.with_frame(None, |frame| gx.with_encoder(|encoder| {
                    encoder.with_render_pass(
                        frame.attachments(Some(bg_color_target), None),
                        |rpass| {
                            rpass.set_pipeline(&pipeline);
                            rpass.set_bind_group(0, &binding, &[]);
                            rpass.set_vertex_buffer(0, vertices.slice(..));
                            rpass.draw(0..vertex_data.len() as u32, 0..1);
                        }
                    );
                })).expect("frame error");

                println!("{:?}", then.elapsed());
            },

            _ => {}
        }
    }).unwrap();
}