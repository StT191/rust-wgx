
mod gui;
pub use gui::*;

mod clipboard;
pub use clipboard::*;


// wgx-color extension

use iced_winit::core as iced_core;

// extend wgx color
pub trait IntoIcedCoreColor {
    fn iced_core(self) -> iced_core::Color;
}

impl IntoIcedCoreColor for wgx::Color {
    fn iced_core(self) -> iced_core::Color {
        iced_core::Color::from_linear_rgba(self.r, self.g, self.b, self.a)
    }
}