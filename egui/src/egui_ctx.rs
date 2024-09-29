
use platform::winit::event::WindowEvent;
use platform::{time::Duration, AppCtx, AppEvent};

#[cfg(target_family="wasm")]
use platform::{web_clipboard::WebClipboard, log_warn};

use epaint::ahash::HashSet;
use egui::{Context, ClippedPrimitive, TexturesDelta, ViewportCommand, ViewportInfo, ViewportId};
use egui_winit::{State, update_viewport_info, process_viewport_commands};

use wgx::{WgxDeviceQueue, wgpu::{CommandEncoder, RenderPass}};
use crate::*;


pub struct EguiCtx {
  pub context: Context,
  pub state: State,
  pub screen_dsc: ScreenDescriptor,

  #[cfg(target_family="wasm")]
  pub web_clipboard: WebClipboard,
}

impl EguiCtx {

  pub fn new(app_ctx: &AppCtx) -> Self {
    let context = Context::default();
    // install image loaders, need to be added via features in egui_extras
    egui_extras::install_image_loaders(&context);

    let screen_dsc = ScreenDescriptor::from_window(app_ctx.window());

    #[allow(unused_mut)]
    let mut state = State::new(
      context.clone(), ViewportId::ROOT, app_ctx.window(),
      Some(screen_dsc.pixels_per_point), None /* theme */, None /* max_texture_side */,
    );

    #[cfg(not(target_family="wasm"))] {
      Self { state, screen_dsc, context }
    }

    #[cfg(target_family="wasm")] {

      state.set_clipboard_text("DUMMY_CONTENT".to_string());

      let web_clipboard = WebClipboard::connect(app_ctx, true);
      log_warn!(web_clipboard);

      Self { state, screen_dsc, context, web_clipboard }
    }
  }

  pub fn event(&mut self, app_ctx: &AppCtx, app_event: &AppEvent) -> (bool, bool) {
    match app_event {

      AppEvent::WindowEvent(window_event) => {

        if matches!(window_event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged {..}) {
          self.screen_dsc = ScreenDescriptor::from_window(app_ctx.window());
        }

        if *window_event != WindowEvent::RedrawRequested {
          let res = self.state.on_window_event(app_ctx.window(), window_event);

          #[cfg(target_family="wasm")] {

            let events = &mut self.state.egui_input_mut().events;

            if let Some(egui::Event::Paste(_)) = events.last() {

              if !self.web_clipboard.is_listening() {
                self.web_clipboard.fetch();
              }

              // unqueue event, wait for ClipboardPaste/Fetch
              events.pop();

              return (false, false)
            }
          }

          return (res.repaint, res.consumed)
        }

        (false, false)
      },

      #[cfg(target_family="wasm")]
      AppEvent::ClipboardPaste | AppEvent::ClipboardFetch => {
        if let Some(text) = self.web_clipboard.read() {
          self.state.egui_input_mut().events.push(egui::Event::Paste(text));
          (true, true)
        }
        else {(false, false)}
      },

      _ => (false, false),
    }
  }

  pub fn run(&mut self, app_ctx: &AppCtx, ui_fn: impl FnMut(&Context)) -> FrameOutput {

    let mut input = self.state.take_egui_input(app_ctx.window());

    let viewport_id = input.viewport_id;

    let viewport_info = input.viewports.get_mut(&viewport_id).unwrap();
    update_viewport_info(viewport_info, &self.context, app_ctx.window(), false);

    let mut output = self.context.run(input, ui_fn);

    #[cfg(target_family="wasm")]
    if !output.platform_output.copied_text.is_empty() {
      let copied = std::mem::take(&mut output.platform_output.copied_text);
      self.web_clipboard.write(copied);
    }

    self.state.handle_platform_output(app_ctx.window(), output.platform_output);

    let viewport_output = output.viewport_output.remove(&viewport_id).unwrap();

    if !viewport_output.commands.is_empty() {
      process_viewport_commands(
        &self.context,
        &mut ViewportInfo::default(),
        viewport_output.commands.iter().cloned(),
        app_ctx.window(),
        &mut HashSet::default(),
      );
    }

    FrameOutput {
      clipped_primitives: self.context.tessellate(output.shapes, output.pixels_per_point),
      textures_delta: output.textures_delta,
      screen_dsc: self.screen_dsc.clone(),
      commands: viewport_output.commands,
      repaint_delay: viewport_output.repaint_delay,
    }
  }
}


pub struct FrameOutput {
  pub clipped_primitives: Vec<ClippedPrimitive>,
  pub textures_delta: TexturesDelta,
  pub screen_dsc: ScreenDescriptor,
  pub commands: Vec<ViewportCommand>,
  pub repaint_delay: Duration,
}

impl FrameOutput {

  pub fn prepare(&self, renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder) {
    prepare_renderer(renderer, gx, encoder, &self.textures_delta, &self.clipped_primitives, &self.screen_dsc);
  }

  pub fn render<'a>(&'a self, renderer: &'a Renderer, rpass: &mut RenderPass<'static>) {
    renderer.render(rpass, &self.clipped_primitives, &self.screen_dsc);
  }
}