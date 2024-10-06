
use platform::winit::{window::{WindowAttributes}, event::{WindowEvent}};
use platform::{*, time::*};
use wgx_iced::*;
use wgx::*;

mod ui;

main_app_closure! {
  LogLevel::Warn,
  WindowAttributes::default().with_title("WgFx"),
  init_app,
}

async fn init_app(app_ctx: &mut AppCtx) -> impl FnMut(&mut AppCtx, &AppEvent) {

  let window = app_ctx.window_clone();

  // let features = features!(TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
  // let limits = limits!(max_inter_stage_shader_components: 60);

  let (gx, mut target) = Wgx::new_with_target(window.clone(), features!(), limits!(), window.inner_size(), false, 1, None).await.unwrap();

  log::warn!("{:?}", gx.adapter.get_info());


  let mut engine = Engine::new_wgx(&gx, target.format(), 4);

  let mut gui = Gui::new(app_ctx, engine.renderer(&gx), ui::Ui::default(), ui::theme());


  // let mut frame_counter = timer::IntervalCounter::from_secs(5.0);

  move |app_ctx: &mut AppCtx, event: &AppEvent| {

    let event_was_queued = gui.event(app_ctx, event);

    // redraw handling
    if event_was_queued {
      app_ctx.request = Some(Duration::ZERO); // as early as possible
    }

    if let AppEvent::WindowEvent(window_event) = event { match window_event {

      WindowEvent::Resized(size) => {
        target.update(&gx, *size);
      },

      WindowEvent::RedrawRequested => {

        // gui handling
        gui.update(app_ctx);

        // draw
        target.with_frame(None, |frame| engine.with_encoder(&gx, |engine, encoder| {

          let bg_color = gui.program().bg_color;
          gui.draw(&gx, engine, encoder, frame, Some(bg_color));

        })).expect("frame error");

        /*frame_counter.add();
        if let Some(counted) = frame_counter.count() { println!("{:?}", counted) }*/

      },

      _ => (),
    }}
  }
}