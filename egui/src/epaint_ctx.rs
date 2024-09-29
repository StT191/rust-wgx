
use wgx::{WgxDeviceQueue, wgpu::{CommandEncoder, RenderPass}};

use crate::*;

use epaint::{
  Rect, Fonts, text::{FontDefinitions, /*FontId*/}, TextureId, Shape, ClippedShape, ClippedPrimitive,
  Tessellator, TessellationOptions, ImageData, TextureManager, TextureAtlas,
};



pub fn clip_shapes(shapes: impl IntoIterator<Item=Shape>, clip_rect: Rect) -> impl Iterator<Item=ClippedShape> {
  shapes.into_iter().map(move |shape| ClippedShape {shape, clip_rect})
}


pub struct EpaintCtx {
  pub texture_manager: TextureManager,
  pub fonts: Fonts,
  pub screen_dsc: ScreenDescriptor,
}


impl EpaintCtx {

  pub fn new(screen_dsc: ScreenDescriptor, max_texture_side: usize, font_defs: FontDefinitions) -> Self {

    let mut texture_manager = TextureManager::default();

    let fonts = Fonts::new(screen_dsc.pixels_per_point, max_texture_side, font_defs);

    assert_eq!(
      texture_manager.alloc("font_texture".to_string(), ImageData::Font(fonts.image()), TextureAtlas::texture_options()),
      TextureId::default(),
    );

    Self { texture_manager, fonts, screen_dsc }
  }

  pub fn begin_frame(&mut self, screen_dsc: Option<ScreenDescriptor>, max_texture_side: Option<usize>) {
    if let Some(screen_dsc) = screen_dsc {
      self.screen_dsc = screen_dsc;
    }
    let max_texture_side = max_texture_side.unwrap_or(self.fonts.max_texture_side());
    self.fonts.begin_pass(self.screen_dsc.pixels_per_point, max_texture_side);
  }

  pub fn clip_shapes(&self, shapes: impl IntoIterator<Item=Shape>, clip_rect: Option<Rect>) -> impl Iterator<Item=ClippedShape> {
    clip_shapes(shapes, clip_rect.unwrap_or(self.screen_dsc.clip_rect()))
  }

  pub fn tessellate(&self, options: TessellationOptions, shapes: impl IntoIterator<Item=ClippedShape>, out: &mut Vec<ClippedPrimitive>) {

    let mut tesselator = Tessellator::new(
      self.screen_dsc.pixels_per_point, options, self.fonts.image().size,
      self.fonts.texture_atlas().lock().prepared_discs(),
    );

    for clipped_shape in shapes {
      tesselator.tessellate_clipped_shape(clipped_shape, out);
    }
  }

  pub fn prepare(&mut self,
    renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder,
    clipped_primitives: &[ClippedPrimitive],
  ) {
    // update fonts texture it necessary
    if let Some(image_delta) = self.fonts.texture_atlas().lock().take_delta() {
      self.texture_manager.set(TextureId::default(), image_delta);
    }
    prepare_renderer(renderer, gx, encoder, &self.texture_manager.take_delta(), clipped_primitives, &self.screen_dsc);
  }

  pub fn render<'a>(&'a self,
    renderer: &'a Renderer, rpass: &mut RenderPass<'static>, clipped_primitives: &'a [ClippedPrimitive],
  ) {
    renderer.render(rpass, clipped_primitives, &self.screen_dsc);
  }
}
