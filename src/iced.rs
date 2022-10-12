
use iced_wgpu::{Viewport, Renderer};
use iced_winit::{
    winit::{
        dpi::PhysicalPosition,
        window::Window, event::{WindowEvent, ModifiersState},
    },
    renderer::{Style}, program::{Program, State}, mouse::Interaction,
    *
};

use wgpu::{CommandEncoder, util::StagingBelt};

use crate::{Wgx, RenderAttachable};


pub struct Iced<P:'static + Program<Renderer=Renderer>> {
    renderer: Renderer,
    program_state: State<P>,
    viewport: Viewport,
    cursor: PhysicalPosition<f64>,
    interaction: Interaction,
    modifiers: ModifiersState,
    clipboard: Clipboard,
    staging_belt: StagingBelt,
    debug: Debug,
}


impl <P:'static + iced_winit::Program<Renderer=Renderer>>Iced<P> {

    pub fn new(mut renderer:Renderer, program:P, (width, height):(u32, u32), window:&Window) -> Self {

        let mut debug = Debug::new();

        let viewport = Viewport::with_physical_size(Size::new(width, height), window.scale_factor());

        let cursor = PhysicalPosition::new(-1.0, -1.0);
        let clipboard = Clipboard::connect(&window);

        let program_state = State::new(
            program, viewport.logical_size(),
            &mut renderer, &mut debug,
        );

        let interaction = program_state.mouse_interaction();

        Self {
            renderer, program_state, viewport, cursor, interaction,
            modifiers: ModifiersState::default(),
            clipboard,
            staging_belt: StagingBelt::new(10240),
            debug,
        }
    }


    pub fn program(&mut self) -> &P {
        self.program_state.program()
    }


    pub fn event(&mut self, event:&WindowEvent, window:&Window) {
        match event {
            WindowEvent::Resized(size) => {
                self.viewport = Viewport::with_physical_size(
                    Size::new(size.width, size.height),
                    window.scale_factor(),
                );
            }
            WindowEvent::ScaleFactorChanged { scale_factor, ref new_inner_size } => {
                self.viewport = Viewport::with_physical_size(
                    Size::new(new_inner_size.width, new_inner_size.height),
                    *scale_factor,
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = *position;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
            }
            _ => (),
        }

        if let Some(event) = iced_winit::conversion::window_event(
            &event, window.scale_factor(), self.modifiers,
        ) {
            self.program_state.queue_event(event);
        }
    }


    pub fn message(&mut self, message:P::Message) {
        self.program_state.queue_message(message)
    }


    pub fn update_cursor(&mut self, window: &Window) {
        let interaction = self.program_state.mouse_interaction();
        if self.interaction != interaction {
            window.set_cursor_icon(conversion::mouse_interaction(interaction));
            self.interaction = interaction;
        }
    }


    pub fn update(&mut self) -> (bool, Option<Command<P::Message>>) {
        if !self.program_state.is_queue_empty() {

            let (_events, command) = self.program_state.update(
                self.viewport.logical_size(),
                conversion::cursor_position(
                    self.cursor,
                    self.viewport.scale_factor(),
                ),
                &mut self.renderer,
                &Theme::Light,
                &Style { text_color: Color::BLACK },
                &mut self.clipboard,
                &mut self.debug,
            );

            (true, command)
        }
        else { (false, None) }
    }


    pub fn draw(&mut self, gx:&Wgx, encoder:&mut CommandEncoder, target: &impl RenderAttachable) {

        // borrow before the closure
        let (staging_belt, viewport, debug) = (&mut self.staging_belt, &self.viewport, &self.debug);

        self.renderer.with_primitives(|backend, primitive| {
            backend.present(
                &gx.device,
                staging_belt,
                encoder,
                target.color_views().0,
                primitive,
                viewport,
                &debug.overlay(),
            );
        });

        self.staging_belt.finish();
    }


    pub fn recall_staging_belt(&mut self) {
        self.staging_belt.recall();
    }
}