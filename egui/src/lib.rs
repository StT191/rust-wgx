
// re-exports
pub use epaint::{self, ecolor, emath};
pub use egui_wgpu::{self, Renderer, ScreenDescriptor};

#[cfg(feature = "egui")]
pub use egui::*;

#[cfg(feature = "egui")]
pub use egui_winit;


// mods
mod epaint_ctx;
pub use epaint_ctx::*;

#[cfg(feature = "egui")]
mod egui_ctx;
#[cfg(feature = "egui")]
pub use egui_ctx::*;


// common items
use wgx::{WgxDevice, WgxDeviceQueue, RenderTarget, wgpu::CommandEncoder};

pub fn renderer(gx: &impl WgxDevice, target: &impl RenderTarget) -> Renderer {
  Renderer::new(gx.device(), target.view_format(), target.depth_testing(), target.msaa(), false)
}

fn prepare_renderer(
  renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder,
  textures_delta: &TexturesDelta, clipped_primitives: &[ClippedPrimitive], screen_dsc: &ScreenDescriptor,
) {
  for (id, image_delta) in &textures_delta.set {
    renderer.update_texture(gx.device(), gx.queue(), *id, image_delta);
  }

  if !clipped_primitives.is_empty() {
    let commands = renderer.update_buffers(gx.device(), gx.queue(), encoder, clipped_primitives, screen_dsc);

    if !commands.is_empty() {
      gx.queue().submit(commands);
    }
  }

  for id in &textures_delta.free {
    renderer.free_texture(id);
  }
}


// helper trait
use platform::winit::{window::Window as WinitWindow};

pub trait ScreenDescriptorExtension {
  fn new(size_in_pixels: [u32; 2], pixels_per_point: f32) -> Self;
  fn from_window(window: &WinitWindow) -> Self;
  fn clone(&self) -> Self;
  fn clip_rect(&self) -> Rect;
}

impl ScreenDescriptorExtension for ScreenDescriptor {

  fn new(size_in_pixels: [u32; 2], pixels_per_point: f32) -> Self {
    Self { size_in_pixels, pixels_per_point }
  }

  fn from_window(window: &WinitWindow) -> Self {
    let size = window.inner_size();
    Self::new([size.width, size.height], window.scale_factor() as f32)
  }

  fn clone(&self) -> Self {
    Self::new(self.size_in_pixels, self.pixels_per_point)
  }

  fn clip_rect(&self) -> Rect {
    let sf = self.pixels_per_point;
    let [w, h] = self.size_in_pixels;
    [[0.0, 0.0].into(), [w as f32/sf, h as f32/sf].into()].into()
  }
}
