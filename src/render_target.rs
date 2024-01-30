
use crate::{*, error::*};
use wgpu::{PresentMode as Prs, SurfaceCapabilities, TextureFormat};


pub trait RenderTarget {

    // to implement
    fn size(&self) -> (u32, u32);
    fn msaa(&self) -> u32;
    fn depth_testing(&self) -> Option<TextureFormat>;
    fn format(&self) -> TextureFormat;
    fn view_format(&self) -> TextureFormat;

    // provided
    fn clear_color_transform(&self) -> ColorTransform {
        if self.format().is_srgb() && self.view_format().is_srgb() && self.msaa() > 1 {
            ColorTransform::None
        } else {
            ColorTransform::Linear
        }
    }

    fn render_bundle<'a>(&self, gx: &'a Wgx, handler: impl FnOnce(&mut wgpu::RenderBundleEncoder<'a>)) -> wgpu::RenderBundle {
        gx.render_bundle(&[Some(self.view_format())], self.depth_testing(), self.msaa(), handler)
    }

    fn render_pipeline(
        &self, gx: &impl WgxDevice,
        layout: Option<(&[wgpu::PushConstantRange], &[&wgpu::BindGroupLayout])>,
        buffers: &[wgpu::VertexBufferLayout],
        vertex_state: (&wgpu::ShaderModule, &str, Primitive),
        (fs_module, fs_entry_point, blend): (&wgpu::ShaderModule, &str, Option<Blend>),
    ) -> wgpu::RenderPipeline {
        gx.render_pipeline(
            self.msaa(), self.depth_testing(), layout, buffers, vertex_state,
            Some((fs_module, fs_entry_point, &[(self.view_format(), blend)])),
        )
    }
}

pub trait RenderAttachable: RenderTarget {

    // to implement
    fn color_views(&self) -> (&wgpu::TextureView, Option<&wgpu::TextureView>);
    fn depth_view(&self) -> Option<&wgpu::TextureView>;

    // provided
    fn color_attachment(&self, clear_color: Option<Color>) -> wgpu::RenderPassColorAttachment {
        let (view, msaa) = self.color_views();
        ColorAttachment { view, msaa, clear: clear_color.map(|cl| (cl, self.clear_color_transform())) }.into()
    }

    fn depth_attachment(&self, clear: Option<f32>) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        self.depth_view().map(|view| DepthAttachment { view, clear }.into())
    }

    fn attachments(&self, clear_color: Option<Color>, clear_depth: Option<f32>) -> RenderAttachments<1> {
        ([Some(self.color_attachment(clear_color))], self.depth_attachment(clear_depth))
    }
}


// helper
#[derive(Debug)]
pub struct TextureLot {
    pub texture: wgpu::Texture,
    pub descriptor: TexDsc,
    pub view: wgpu::TextureView,
}

impl TextureLot {
    pub fn new(gx:&impl WgxDevice, descriptor: TexDsc) -> Self {
        let texture = gx.texture(&descriptor);
        let view = texture.create_default_view(Some(descriptor.view_format));
        TextureLot { texture, descriptor, view }
    }
    pub fn new_with_data<T: ReadBytes>(gx:&impl WgxDeviceQueue, descriptor: TexDsc, data: T) -> Self {
        let texture = gx.texture_with_data(&descriptor, data);
        let view = texture.create_default_view(Some(descriptor.view_format));
        TextureLot { texture, descriptor, view }
    }
    pub fn new_2d(
        gx:&impl WgxDevice, size:(u32, u32), sample_count:u32,
        format:TextureFormat, view_format:Option<TextureFormat>, usage:TexUse
    ) -> Self {
        Self::new(gx, TexDsc::new_2d(size, sample_count, format, view_format, usage))
    }
    pub fn new_2d_with_data<T: ReadBytes>(
        gx:&impl WgxDeviceQueue, size:(u32, u32), sample_count:u32,
        format:TextureFormat, view_format:Option<TextureFormat>, usage:TexUse, data: T,
    ) -> Self {
        Self::new_with_data(gx, TexDsc::new_2d(size, sample_count, format, view_format, usage), data)
    }
}

impl RenderTarget for TextureLot {
    fn size(&self) -> (u32, u32) { (self.texture.width(), self.texture.height()) }
    fn msaa(&self) -> u32 { 1 }
    fn depth_testing(&self) -> Option<TextureFormat> { None }
    fn format(&self) -> TextureFormat { self.descriptor.format }
    fn view_format(&self) -> TextureFormat { self.descriptor.view_format }
}

impl RenderAttachable for TextureLot {
    fn color_views(&self) -> (&wgpu::TextureView, Option<&wgpu::TextureView>) { (&self.view, None) }
    fn depth_view(&self) -> Option<&wgpu::TextureView> { None }
}



#[derive(Debug)]
pub struct SurfaceTarget {
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub view_format: TextureFormat,
    pub msaa: u32,
    pub msaa_opt: Option<TextureLot>,
    pub depth_opt: Option<TextureLot>,
}

impl RenderTarget for SurfaceTarget {
    fn size(&self) -> (u32, u32) { (self.config.width, self.config.height) }
    fn msaa(&self) -> u32 { self.msaa }
    fn depth_testing(&self) -> Option<TextureFormat> { self.depth_opt.as_ref().map(|d| d.view_format()) }
    fn format(&self) -> TextureFormat { self.config.format }
    fn view_format(&self) -> TextureFormat { self.view_format }
}


#[derive(Debug)]
pub struct SurfaceFrame<'a> {
    pub target: &'a mut SurfaceTarget,
    pub view: wgpu::TextureView,
}

impl RenderTarget for SurfaceFrame<'_> {
    fn size(&self) -> (u32, u32) { self.target.size() }
    fn msaa(&self) -> u32 { self.target.msaa() }
    fn depth_testing(&self) -> Option<TextureFormat> { self.target.depth_testing() }
    fn format(&self) -> TextureFormat { self.target.format() }
    fn view_format(&self) -> TextureFormat { self.target.view_format() }
}

impl RenderAttachable for SurfaceFrame<'_> {
    fn color_views(&self) -> (&wgpu::TextureView, Option<&wgpu::TextureView>) {
        (&self.view, self.target.msaa_opt.as_ref().map(|o| &o.view))
    }
    fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.target.depth_opt.as_ref().map(|o| &o.view)
    }
}


// cloneable surface configuration
const DEFAULT_CONFIG: wgpu::SurfaceConfiguration = wgpu::SurfaceConfiguration {
    usage: TexUse::RENDER_ATTACHMENT,
    format: DEFAULT_SRGB, width: 0, height: 0,
    present_mode: Prs::Mailbox,
    alpha_mode: wgpu::CompositeAlphaMode::Auto,
    desired_maximum_frame_latency: 2,
    view_formats: Vec::new(),
};


impl SurfaceTarget {

    pub fn new(gx:&Wgx, surface:wgpu::Surface<'static>, size:(u32, u32), msaa:u32, depth_testing:bool) -> Res<Self>
    {
        let mut config = DEFAULT_CONFIG.clone();
        config.width = size.0;
        config.height = size.1;

        let SurfaceCapabilities {formats, present_modes, ..} = surface.get_capabilities(&gx.adapter);

        let format =
            *formats.iter().find(|fmt| fmt.is_srgb()) // find a supported srgb format
            .unwrap_or(&formats[0]) // or use the default format
        ;

        config.format = format;
        let view_format = format.add_srgb_suffix();

        if view_format != format {
            config.view_formats.push(view_format);
        }

        config.present_mode =
            if present_modes.contains(&Prs::Mailbox) { Prs::Mailbox }
            else if present_modes.contains(&Prs::AutoVsync) { Prs::AutoVsync }
            else { present_modes[0] }
        ;

        surface.configure(gx.device(), &config);

        Ok(Self {
            config, surface, view_format, msaa,
            msaa_opt: if msaa > 1 { Some(TextureLot::new_2d(gx, size, msaa, format, Some(view_format), TexUse::RENDER_ATTACHMENT)) } else { None },
            depth_opt: if depth_testing { Some(TextureLot::new_2d(gx, size, msaa, DEFAULT_DEPTH, None, TexUse::RENDER_ATTACHMENT)) } else { None },
        })
    }


    pub fn update(&mut self, gx:&impl WgxDevice, size:(u32, u32)) {

        self.config.width = size.0;
        self.config.height = size.1;

        self.surface.configure(gx.device(), &self.config);

        let map_opt = |lot:&TextureLot| {
            let mut descriptor = lot.descriptor.clone();
            descriptor.set_size_2d(size);
            descriptor.sample_count = self.msaa;
            TextureLot::new(gx, descriptor)
        };

        self.msaa_opt = if self.msaa > 1 { self.msaa_opt.as_ref().map(map_opt).or_else(||
            Some(TextureLot::new_2d(gx, size, self.msaa, self.format(), Some(self.view_format), TexUse::RENDER_ATTACHMENT))
        )}
        else { None };

        self.depth_opt = self.depth_opt.as_ref().map(map_opt);
    }


    pub fn with_frame<C: ImplicitControlflow>(
        &mut self, dsc: Option<&wgpu::TextureViewDescriptor>, handler: impl FnOnce(&SurfaceFrame) -> C
    ) -> Res<()>
    {
        let frame = self.surface.get_current_texture().convert()?;

        let controlflow = handler(&SurfaceFrame {
            view: if let Some(dsc) = dsc {
                frame.texture.create_view(dsc)
            } else {
                frame.texture.create_default_view(Some(self.view_format()))
            },
            target: self,
        });

        if controlflow.should_continue() {
            frame.present();
        }

        Ok(())
    }
}



#[derive(Debug)]
pub struct TextureTarget {
    pub texture: wgpu::Texture,
    pub descriptor: TexDsc,
    pub view: wgpu::TextureView,

    pub msaa: u32,
    pub msaa_opt: Option<TextureLot>,
    pub depth_opt: Option<TextureLot>,
}

impl RenderTarget for TextureTarget {
    fn size(&self) -> (u32, u32) { self.descriptor.size_2d() }
    fn msaa(&self) -> u32 { self.msaa }
    fn depth_testing(&self) -> Option<TextureFormat> { self.depth_opt.as_ref().map(|d| d.view_format()) }
    fn format(&self) -> TextureFormat { self.descriptor.format }
    fn view_format(&self) -> TextureFormat { self.descriptor.view_format }
}

impl RenderAttachable for TextureTarget {
    fn color_views(&self) -> (&wgpu::TextureView, Option<&wgpu::TextureView>) {
        (&self.view, self.msaa_opt.as_ref().map(|o| &o.view))
    }
    fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_opt.as_ref().map(|o| &o.view)
    }
}

impl TextureTarget {

    pub fn new(
        gx:&impl WgxDevice, size:(u32, u32), msaa:u32, depth_testing: bool,
        format:TextureFormat, view_format:Option<TextureFormat>, usage:wgpu::TextureUsages,
    ) -> Self
    {
        let TextureLot { texture, descriptor, view } = TextureLot::new_2d(gx, size, 1, format, view_format, usage | TexUse::RENDER_ATTACHMENT);
        Self {
            texture, descriptor, view, // output attachment can have only one sample
            msaa,
            msaa_opt: if msaa > 1 { Some(TextureLot::new_2d(gx, size, msaa, format, view_format, TexUse::RENDER_ATTACHMENT)) } else { None },
            depth_opt: if depth_testing { Some(TextureLot::new_2d(gx, size, msaa, DEFAULT_DEPTH, None, TexUse::RENDER_ATTACHMENT)) } else { None },
        }
    }

    pub fn update(&mut self, gx:&impl WgxDevice, size:(u32, u32)) {

        let map_opt = |descriptor: &TexDsc| {
            let mut descriptor = descriptor.clone();
            descriptor.set_size_2d(size);
            descriptor.sample_count = self.msaa;
            TextureLot::new(gx, descriptor)
        };

        let TextureLot { texture, descriptor, view } = map_opt(&self.descriptor);
        self.texture = texture; self.descriptor = descriptor; self.view = view;

        self.msaa_opt = if self.msaa > 1 { self.msaa_opt.as_ref().map(|d| map_opt(&d.descriptor)).or_else(||
            Some(TextureLot::new_2d(gx, size, self.msaa, self.format(), Some(self.view_format()), TexUse::RENDER_ATTACHMENT))
        )}
        else { None };

        self.depth_opt = self.depth_opt.as_ref().map(|d| map_opt(&d.descriptor));
    }
}
