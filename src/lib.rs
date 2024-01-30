
// passs wgpu types
pub use wgpu::{
    self,
    Features, Limits,
    BufferUsages as BufUse,
    TextureUsages as TexUse,
    ShaderStages as Stage,
    BlendState as Blend, BlendComponent, BlendFactor, BlendOperation,
    MapMode, // buffer mapping
    PrimitiveState as Primitive,
    PrimitiveTopology as Topology,
    IndexFormat, Face, FrontFace, PolygonMode as Polygon,
    util::{StagingBelt, DrawIndirectArgs, DrawIndexedIndirectArgs, DispatchIndirectArgs},
};

// macros
mod macros;

// common types
mod read_bytes;
pub use read_bytes::*;

mod color;
pub use color::*;

// wgx
mod wgx;
pub use wgx::*;

mod render_extension;
pub use render_extension::*;

mod buffer_extension;
pub use buffer_extension::*;

mod texture_extension;
pub use texture_extension::*;

mod render_target;
pub use render_target::*;

mod buffer_helper;
pub use buffer_helper::*;


// features

#[cfg(feature = "math")]
pub mod math;


// wgsl modules
#[cfg(feature = "wgsl_modules")]
pub use wgsl_modules;


// error handling
pub mod error;


// control flow helper

pub trait ImplicitControlflow {
    fn should_continue(&self) -> bool;
}

impl ImplicitControlflow for () {
    fn should_continue(&self) -> bool { true }
}

impl<B, C> ImplicitControlflow for std::ops::ControlFlow<B, C> {
    fn should_continue(&self) -> bool { self.is_continue() }
}