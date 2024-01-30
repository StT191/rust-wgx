
use std::sync::Arc;
use std::{time::{Instant}, mem::size_of};
use pollster::FutureExt;
use winit::{
    event_loop::{ControlFlow, EventLoop}, dpi::PhysicalSize,
    window::WindowBuilder, event::{Event, WindowEvent, KeyEvent, ElementState},
    keyboard::{PhysicalKey, /*KeyCode*/},
};
use wgx::{*, math::*};

// common
#[path="common/world_view.rs"] #[allow(dead_code)]
mod world_view;
use world_view::{WorldView, InputKey};


fn main() {

  const DEPTH_TESTING:bool = true;
  const MSAA:u32 = 4;
  const BLENDING:Option<Blend> = None;


  let (width, height) = (1000, 1000);

  let event_loop = EventLoop::new().unwrap();

  let window = Arc::new(WindowBuilder::new().with_transparent(true).build(&event_loop).unwrap());

  let _ = window.request_inner_size(PhysicalSize::<u32>::from((width, height)));
  window.set_title("WgFx");

  let features = features!(MAPPABLE_PRIMARY_BUFFERS, POLYGON_MODE_LINE/*, MULTI_DRAW_INDIRECT*/);

  let (gx, surface) = Wgx::new(Some(window.clone()), features, limits!{}).block_on().unwrap();
  let mut target = SurfaceTarget::new(&gx, surface.unwrap(), (width, height), MSAA, DEPTH_TESTING).unwrap();


  // pipeline
  let shader = gx.load_wgsl(wgsl_modules::include!("common/shaders/shader_3d_inst_text_diff.wgsl"));

  let pipeline = target.render_pipeline(&gx,
    None, &[
      vertex_dsc!(Vertex, 0 => Float32x3, 1 => Float32x3, 2 => Float32x3),
      vertex_dsc!(Instance, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4)
    ],
    (&shader, "vs_main", Primitive {
      cull_mode: Some(Face::Back),
      polygon_mode: Polygon::Fill,
      ..Primitive::default()
    }),
    (&shader, "fs_main", BLENDING),
  );

  // colors
  let bg_color = Color::from([0x00, 0x00, 0x00, 0xCC]);

  let color_texture = TextureLot::new_2d_with_data(&gx, (1, 1), 1, DEFAULT_SRGB, None, TexUse::TEXTURE_BINDING, [255u8, 0, 0, 255]);
  let sampler = gx.default_sampler();

  // compute vertices
  type Vertex = [[f32;3];3];

  let steps = 64u32;

  let wg_size = UVec3::new(8, 8, 3); // workgroup size

  let vertex_size = size_of::<Vertex>() as u64;

  let mesh_len = 3 * (2 * 3) * (steps * steps);
  let mesh_size = vertex_size * mesh_len as u64;

  println!("mesh_len: {mesh_len:#}");

  let vertex_buffer = gx.buffer(BufUse::STORAGE | BufUse::VERTEX | BufUse::MAP_READ, mesh_size, false);

  let layout = gx.layout(&[binding!(0, Stage::COMPUTE, StorageBuffer, mesh_size, false)]);

  let cp_shader = gx.load_wgsl(wgsl_modules::include!("common/shaders/compute_sphere_square.wgsl"));

  let cp_pipeline = gx.compute_pipeline(Some((&[], &[&layout])), (&cp_shader, "cp_main"));

  let binding_cp = gx.bind(&layout, &[bind!(0, Buffer, &vertex_buffer)]);

  gx.with_encoder(|encoder| {
    encoder.with_compute_pass(|cpass| {
      cpass.set_pipeline(&cp_pipeline);
      cpass.set_bind_group(0, &binding_cp, &[]);
      cpass.dispatch_workgroups(steps/wg_size.x, steps/wg_size.y, 3/wg_size.z);
    });
  });


  // read out the first triangles
  /*vertex_buffer.with_map_sync(&gx, 0..(3*vertex_size), MapMode::Read, |buffer_slice| {

    let mapped = buffer_slice.get_mapped_range();
    let vertices: &[Vertex] = unsafe { mapped.align_to().1 };
    // let vertices: Vec<_> = vertices.iter().map(|v| v[0]).collect();
    // let vertices: &[Vertex] = unsafe { vertices.align_to().1 };
    let vertices: Vec<_> = vertices.iter().map(|v| format!("{:?}", v)).collect();

    eprintln!("{:#?}", vertices);

  }).unwrap();*/


  // instance data

  let instance_data = [
    Mat4::from_rotation_y(deg(000.0)),
    Mat4::from_rotation_y(deg(090.0)),
    Mat4::from_rotation_y(deg(180.0)),
    Mat4::from_rotation_y(deg(270.0)),
    Mat4::from_rotation_y(deg(000.0))*Mat4::from_rotation_z(deg(180.0)),
    Mat4::from_rotation_y(deg(090.0))*Mat4::from_rotation_z(deg(180.0)),
    Mat4::from_rotation_y(deg(180.0))*Mat4::from_rotation_z(deg(180.0)),
    Mat4::from_rotation_y(deg(270.0))*Mat4::from_rotation_z(deg(180.0)),
  ];

  // buffers
  let indirect_buffer = gx.buffer_from_data(BufUse::INDIRECT, [
    DrawIndirectArgs::try_from_ranges(0..mesh_len as usize, 0..instance_data.len() as usize).unwrap(),
  ]);

  let instance_buffer = gx.buffer_from_data(BufUse::VERTEX, instance_data);


  // world
  let (width, height) = (width as f32, height as f32);
  let mut world = WorldView::new(&gx, 10.0, 5.0, 0.1, FovProjection::window(45.0, width, height));

  world.objects = Mat4::from_uniform_scale(0.25 * height);
  world.calc_clip_matrix();

  let light_matrix = Mat4::from_rotation_x(deg(-30.0));

  world.light_matrix = light_matrix * world.rotation; // keep light


  // staging belt
  let mut staging_belt = StagingBelt::new(4 * world.clip_buffer.size());

  gx.with_encoder(|mut encoder| {
    staging_belt.write_data(&gx, &mut encoder, &world.clip_buffer, 0, world.clip_matrix);
    staging_belt.write_data(&gx, &mut encoder, &world.light_buffer, 0, world.light_matrix);
    staging_belt.finish();
  });
  staging_belt.recall();


  // bind
  let binding = gx.bind(&pipeline.get_bind_group_layout(0), &[
    bind!(0, Buffer, world.clip_buffer),
    bind!(1, Buffer, world.light_buffer),
    bind!(2, TextureView, &color_texture.view),
    bind!(3, Sampler, &sampler),
  ]);

  // render bundles
  let bundles = [target.render_bundle(&gx, |rpass| {
    rpass.set_pipeline(&pipeline);
    rpass.set_bind_group(0, &binding, &[]);
    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rpass.set_vertex_buffer(1, instance_buffer.slice(..));
    rpass.draw_indirect(&indirect_buffer, 0);
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
        world.fov.resize_window(size.width as f32, size.height as f32, true);
        world.calc_clip_matrix();
        world.light_matrix = light_matrix * world.rotation; // keep light

        gx.with_encoder(|mut encoder| {
          staging_belt.write_data(&gx, &mut encoder, &world.clip_buffer, 0, world.clip_matrix);
          staging_belt.write_data(&gx, &mut encoder, &world.light_buffer, 0, world.light_matrix);
          staging_belt.finish();
        });
        staging_belt.recall();
      },

      Event::WindowEvent { event: WindowEvent::KeyboardInput { event: KeyEvent {
        physical_key: PhysicalKey::Code(keycode), state: ElementState::Pressed, ..
      }, ..}, ..} => {
        if let Some(key) = InputKey::match_keycode(keycode) {
          world.input(key);
          world.calc_clip_matrix();
          world.light_matrix = light_matrix * world.rotation; // keep light

          gx.with_encoder(|mut encoder| {
            staging_belt.write_data(&gx, &mut encoder, &world.clip_buffer, 0, world.clip_matrix);
            staging_belt.write_data(&gx, &mut encoder, &world.light_buffer, 0, world.light_matrix);
            staging_belt.finish();
          });
          staging_belt.recall();

          window.request_redraw();
        }
      },

      Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {

        let then = Instant::now();

        target.with_frame(None, |frame| gx.with_encoder(|encoder| {
          encoder.pass_bundles(frame.attachments(Some(bg_color), Some(1.0)), &bundles);
        })).expect("frame error");

        println!("{:?}", then.elapsed());
      },

      _ => {}
    }
  }).unwrap();
}