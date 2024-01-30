
use arrayvec::ArrayVec;
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::rwh::{HasWindowHandle, HasDisplayHandle};
use std::{sync::Arc, ops::{RangeBounds, Bound}};
use crate::{*, error::*};


// wgx
#[derive(Debug)]
pub struct Wgx {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
}

impl Wgx {
    pub fn instance() -> wgpu::Instance {
        wgpu::Instance::new(Default::default())
    }

    pub async fn request_adapter<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static>(
        instance: &wgpu::Instance, window: Option<Arc<W>>
    )
        -> Res<(wgpu::Adapter, Option<wgpu::Surface<'static>>)>
    {
        let surface = if let Some(win) = window {
            Some(instance.create_surface(win).or(Err("couldn't create surface"))?)
        }
        else { None };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface.as_ref(),
        }).await.ok_or("couldn't get adapter")?;

        Ok((adapter, surface))
    }

    pub async fn request_device(adapter: &wgpu::Adapter, features:wgpu::Features, limits:wgpu::Limits) -> Res<(wgpu::Device, wgpu::Queue)> {

        #[cfg(target_family = "wasm")] let limits = limits.using_resolution(adapter.limits());

        adapter.request_device(
            &wgpu::DeviceDescriptor { label: None, required_features: features, required_limits: limits },
            None,
        ).await.convert()
    }

    pub async fn new<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static>(
        window:Option<Arc<W>>, features:wgpu::Features, limits:wgpu::Limits
    )
        -> Res<(Self, Option<wgpu::Surface<'static>>)>
    {
        let instance = Self::instance();
        let (adapter, surface) = Self::request_adapter(&instance, window).await?;
        let (device, queue) = Self::request_device(&adapter, features, limits).await?;
        Ok((Self {device, queue, instance, adapter}, surface))
    }
}


// device methods
pub trait WgxDevice {

    fn device(&self) -> &wgpu::Device;

    // texture, sampler

    fn texture(&self, descriptor:&TexDsc) -> wgpu::Texture {
        self.device().create_texture(&descriptor.into())
    }

    fn sampler(&self, descriptor: &wgpu::SamplerDescriptor) -> wgpu::Sampler {
        self.device().create_sampler(descriptor)
    }

    fn default_sampler(&self) -> wgpu::Sampler {
        self.device().create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            border_color: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None, // Some(wgpu::CompareFunction::LessEqual),
            anisotropy_clamp: 1,
        })
    }


    // buffer

    fn buffer(&self, usage:BufUse, size:u64, mapped_at_creation:bool) -> wgpu::Buffer {
        self.device().create_buffer(&wgpu::BufferDescriptor {usage, size, mapped_at_creation, label: None})
    }

    fn buffer_from_data<T: ReadBytes>(&self, usage:BufUse, data:T) -> wgpu::Buffer {
        self.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            usage, contents: data.read_bytes(), label: None
        })
    }


    // shader

    fn load_wgsl(&self, code:&str) -> wgpu::ShaderModule {
        let source = wgpu::ShaderSource::Wgsl(code.into());
        self.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source })
    }


    // bind group

    fn layout(&self, entries:&[wgpu::BindGroupLayoutEntry]) -> wgpu::BindGroupLayout {
        self.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries, label: None
        })
    }

    fn bind(&self, layout:&wgpu::BindGroupLayout, entries:&[wgpu::BindGroupEntry]) -> wgpu::BindGroup {
        self.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout, entries, label: None
        })
    }


    fn command_encoder (&self) -> wgpu::CommandEncoder {
        self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }


    // render bundle

    fn render_bundle<'a>(&'a self,
        formats: &[Option<wgpu::TextureFormat>], depth_testing:Option<wgpu::TextureFormat>, msaa:u32,
        handler: impl FnOnce(&mut wgpu::RenderBundleEncoder<'a>),
    )
        -> wgpu::RenderBundle
    {
        let mut encoder = self.device().create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
            label: None,
            color_formats: formats,
            depth_stencil: if let Some(format) = depth_testing { Some(wgpu::RenderBundleDepthStencil {
                format, depth_read_only: false, stencil_read_only: false,
            })} else { None },
            sample_count: msaa,
            multiview: None,
        });

        handler(&mut encoder);

        encoder.finish(&wgpu::RenderBundleDescriptor::default())
    }


    // compute pipeline

    fn compute_pipeline(
        &self,
        layout:Option<(&[wgpu::PushConstantRange], &[&wgpu::BindGroupLayout])>,
        (module, entry_point):(&wgpu::ShaderModule, &str),
    ) -> wgpu::ComputePipeline {

        let layout = layout.map(|(push_constant_ranges, bind_group_layouts)|
            self.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None, push_constant_ranges, bind_group_layouts,
            })
        );

        self.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: layout.as_ref(),
            module, entry_point,
        })
    }


    // render pipeline

    fn render_pipeline<const S: usize>(
        &self,
        msaa: u32, depth_testing: Option<wgpu::TextureFormat>,
        layout: Option<(&[wgpu::PushConstantRange], &[&wgpu::BindGroupLayout])>,
        buffers: &[wgpu::VertexBufferLayout],
        (module, entry_point, primitive): (&wgpu::ShaderModule, &str, wgpu::PrimitiveState),
        fragment: Option<(&wgpu::ShaderModule, &str, &[(wgpu::TextureFormat, Option<Blend>); S])>,
    ) -> wgpu::RenderPipeline {

        // cache temporar values
        let pipeline_layout;
        let targets: ArrayVec<_, S>;


        self.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {

            label: None,

            layout: if let Some((push_constant_ranges, bind_group_layouts)) = layout {

                pipeline_layout = self.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None, push_constant_ranges, bind_group_layouts,
                });

                Some(&pipeline_layout)
            }
            else { None },

            vertex: wgpu::VertexState { module, entry_point, buffers },

            primitive,

            fragment: if let Some((module, entry_point, formats)) = fragment {

                targets = formats.iter().map(|(format, blend)| Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: *blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })).collect();

                Some(wgpu::FragmentState { module, entry_point, targets: &targets })
            }
            else {None},

            depth_stencil: if let Some(format) = depth_testing { Some(wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }) }
            else { None },

            multisample: wgpu::MultisampleState {
                count: msaa, mask: !0, alpha_to_coverage_enabled: false,
            },

            multiview: None,

        })
    }
}


// queue methods
pub trait WgxQueue {

    fn queue(&self) -> &wgpu::Queue;

    fn write_texture<T: ReadBytes>(&self, texture:&wgpu::Texture, (x, y, w, h):(u32, u32, u32, u32), data:T) {
        self.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture, mip_level: 0, origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All
            },
            data.read_bytes(),
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: None, rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        )
    }

    fn write_buffer<T: ReadBytes>(&self, buffer:&wgpu::Buffer, offset:u64, data:T) {
        self.queue().write_buffer(buffer, offset, data.read_bytes());
    }

    fn staging_view<'a>(&'a self, buffer:&'a wgpu::Buffer, range: impl RangeBounds<u64>) -> Option<wgpu::QueueWriteBufferView<'a>> {

        let offset = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => start + 1,
            Bound::Unbounded => 0,
        };

        let size = match range.end_bound() {
            Bound::Included(end) => end + 1,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => buffer.size(),
        }.checked_sub(offset)?;

        self.queue().write_buffer_with(buffer, offset, wgpu::BufferSize::new(size)?)
    }
}


pub trait WgxDeviceQueue: WgxDevice + WgxQueue {

    fn texture_with_data<T: ReadBytes>(&self, descriptor: &TexDsc, data: T) -> wgpu::Texture {
        self.device().create_texture_with_data(
            self.queue(), &descriptor.into(), TextureDataOrder::default(), data.read_bytes(),
        )
    }

    // with CommandEncoder

    fn with_encoder<C: ImplicitControlflow>(&self, handler: impl FnOnce(&mut wgpu::CommandEncoder) -> C)
    {
        let mut encoder = self.command_encoder();
        let controlflow = handler(&mut encoder);
        if controlflow.should_continue() {
            self.queue().submit([encoder.finish()]);
        }
    }
}


// impl

impl<T: WgxDevice + WgxQueue> WgxDeviceQueue for T {}

impl WgxDevice for Wgx { fn device(&self) -> &wgpu::Device { &self.device } }
impl WgxQueue for Wgx { fn queue(&self) -> &wgpu::Queue { &self.queue } }

impl WgxDevice for wgpu::Device { fn device(&self) -> &wgpu::Device { self } }
impl WgxQueue for wgpu::Queue { fn queue(&self) -> &wgpu::Queue { self } }

impl WgxDevice for (wgpu::Device, wgpu::Queue) { fn device(&self) -> &wgpu::Device { &self.0 } }
impl WgxQueue for (wgpu::Device, wgpu::Queue) { fn queue(&self) -> &wgpu::Queue { &self.1 } }

impl WgxDevice for (&wgpu::Device, &wgpu::Queue) { fn device(&self) -> &wgpu::Device { self.0 } }
impl WgxQueue for (&wgpu::Device, &wgpu::Queue) { fn queue(&self) -> &wgpu::Queue { self.1 } }