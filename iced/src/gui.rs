
use platform::winit::{dpi::PhysicalPosition, event::WindowEvent, keyboard::ModifiersState};

use platform::{AppCtx, AppEvent};
use wgx::wgpu::{CommandEncoder, TextureFormat};
use wgx::{Wgx, WgxDevice, WgxDeviceQueue, ImplicitControlFlow, RenderAttachable, Color};

use iced_wgpu::{Renderer};
pub use iced_wgpu::{Engine};

use iced_winit::{
  graphics::{Viewport, Antialiasing},
  runtime::{program::{Program, State}, Task, Debug},
  core::{Event, mouse::{Interaction, Cursor}, Pixels, Size, Font, renderer::Style},
  conversion,
};

#[cfg(target_family = "wasm")]
use iced_winit::{
  winit::{event::{KeyEvent, ElementState}, keyboard::{PhysicalKey, KeyCode}},
  runtime::keyboard::{self, key},
};

use super::{Clipboard, IntoIcedCoreColor};


// render engine
pub trait RenderEngine {
  fn new_wgx(gx: &Wgx, format: TextureFormat, msaa: u32) -> Self;
  fn renderer(&self, gx: &impl WgxDevice) -> Renderer;
  fn with_encoder<T: ImplicitControlFlow>(&mut self, gx: &impl WgxDeviceQueue, handler: impl FnOnce(&mut Self, &mut CommandEncoder) -> T) -> T;
}

impl RenderEngine for Engine {

  fn new_wgx(gx: &Wgx, format: TextureFormat, msaa: u32) -> Self {
    let antialiasing = match msaa {
      1 => None,
      2 => Some(Antialiasing::MSAAx2),
      4 => Some(Antialiasing::MSAAx4),
      8 => Some(Antialiasing::MSAAx8),
      16 => Some(Antialiasing::MSAAx16),
      _ => panic!("RenderEngine: unsupported msaa value of {:?}", msaa),
    };
    Engine::new(&gx.adapter, &gx.device, &gx.queue, format, antialiasing)
  }

  fn renderer(&self, gx: &impl WgxDevice) -> Renderer {
    Renderer::new(gx.device(), self, Font::DEFAULT, Pixels(12.0))
  }

  fn with_encoder<T: ImplicitControlFlow>(&mut self, gx: &impl WgxDeviceQueue, handler: impl FnOnce(&mut Self, &mut CommandEncoder) -> T) -> T {
    let mut encoder = gx.command_encoder();
    let res = handler(self, &mut encoder);
    if res.should_continue() {
      self.submit(gx.queue(), encoder);
    }
    res
  }
}


// Gui
pub struct Gui<P: 'static + Program<Renderer=Renderer>> {
  pub renderer: Renderer,
  pub state: State<P>,
  pub viewport: Viewport,
  pub cursor: PhysicalPosition<f64>,
  pub interaction: Interaction,
  pub modifiers: ModifiersState,
  pub theme: P::Theme,
  pub style: Style,
  pub clipboard: Clipboard,
  pub debug: Debug,
}


impl<P: 'static + Program<Renderer=Renderer>> Gui<P> {

  pub fn new(app_ctx: &AppCtx, mut renderer: Renderer, program: P, theme: P::Theme) -> Self {

    let mut debug = Debug::new();

    let size = app_ctx.window().inner_size();
    let scale_factor = app_ctx.window().scale_factor();

    let viewport = Viewport::with_physical_size(Size::new(size.width, size.height), scale_factor);

    let cursor = PhysicalPosition::new(-1.0, -1.0);

    let state = State::new(program, viewport.logical_size(), &mut renderer, &mut debug);

    let interaction = state.mouse_interaction();

    #[cfg(not(target_family = "wasm"))] let clipboard = Clipboard::connect(app_ctx.window_clone());
    #[cfg(target_family = "wasm")] let clipboard = Clipboard::connect(app_ctx);

    Self {
      renderer, state, viewport, cursor, interaction,
      modifiers: ModifiersState::default(), theme,
      style: Style::default(),
      clipboard,
      debug,
    }
  }


  pub fn event(&mut self, _app_ctx: &AppCtx, app_event: &AppEvent) -> bool {
    match app_event {

      AppEvent::WindowEvent(window_event) => {

        match window_event {
          WindowEvent::CursorMoved { position, .. } => {
            self.cursor = *position;
          }

          WindowEvent::ModifiersChanged(modifiers) => {
            self.modifiers = modifiers.state();
          }

          #[cfg(target_family = "wasm")]
          WindowEvent::KeyboardInput { event: KeyEvent {
            physical_key: PhysicalKey::Code(KeyCode::KeyV), state: ElementState::Pressed, ..
          }, ..} => {
            if self.modifiers.control_key() && self.clipboard.web.is_connected() {
              if !self.clipboard.web.is_listening() {
                self.clipboard.web.fetch();
              }
              // return early, don't queue event, wait for ClipboardPaste/Fetch
              return false;
            }
          }

          WindowEvent::Resized(size) => {
            self.viewport = Viewport::with_physical_size(
              Size::new(size.width, size.height),
              self.viewport.scale_factor(),
            );
          }

          WindowEvent::ScaleFactorChanged { scale_factor, ..} => {
            self.viewport = Viewport::with_physical_size(
              Size::new(self.viewport.physical_width(), self.viewport.physical_height()),
              *scale_factor,
            );
          }

          _ => {}
        }

        if let Some(iced_event) = conversion::window_event(
          window_event.clone(), self.viewport.scale_factor(), self.modifiers,
        ) {
          self.state.queue_event(iced_event);
          true
        }
        else { false }
      },

      #[cfg(target_family="wasm")]
      AppEvent::ClipboardPaste | AppEvent::ClipboardFetch => {
        self.state.queue_event(Event::Keyboard(keyboard::Event::KeyPressed {
          key: key::Key::Character("v".into()),
          modified_key: key::Key::Character("v".into()),
          physical_key: key::Physical::Code(key::Code::KeyV),
          location: keyboard::Location::Standard,
          modifiers: keyboard::Modifiers::CTRL,
          text: None,
        }));
        true
      },

      _ => false,
    }
  }


  pub fn program(&mut self) -> &P {
    self.state.program()
  }


  pub fn update(&mut self, app_ctx: &AppCtx) -> (Vec<Event>, Option<Task<P::Message>>) {

    let res = self.state.update(
      self.viewport.logical_size(),
      Cursor::Available(conversion::cursor_position(
        self.cursor,
        self.viewport.scale_factor(),
      )),
      &mut self.renderer,
      &self.theme,
      &self.style,
      &mut self.clipboard,
      &mut self.debug,
    );

    let interaction = self.state.mouse_interaction();

    if self.interaction != interaction {
      app_ctx.window().set_cursor(conversion::mouse_interaction(interaction));
      self.interaction = interaction;
    }

    res
  }


  pub fn draw(
    &mut self, gx: &impl WgxDeviceQueue, engine: &mut Engine, encoder: &mut CommandEncoder,
    target: &impl RenderAttachable, clear_color: Option<Color>,
  ) {
    self.renderer.present(
      engine,
      gx.device(),
      gx.queue(),
      encoder,
      clear_color.map(|cl| cl.iced_core()),
      target.view_format(),
      target.color_views().0,
      &self.viewport,
      &self.debug.overlay(),
    );
  }

}
