#[cfg(feature = "text")]
use crate::text::TextDescriptor;
use crate::{
    CameraBuffer, Color, ComponentController, ComponentFilter, Context, Gpu, GpuDefaults,
    InstanceBuffer, InstanceIndex, InstanceIndices, RenderTarget, Renderer, Sprite,
};

#[derive(Clone, Copy)]
/// Configuration passed to [Renderer]
pub struct RenderConfig<'a> {
    pub target: RenderConfigTarget<'a>,
    pub camera: RenderConfigCamera<'a>,
    pub intances: Option<RenderConfigInstances<'a>>,
    pub msaa: bool,
    pub clear_color: Option<Color>,
}

impl<'a> Default for RenderConfig<'a> {
    fn default() -> Self {
        Self::WORLD
    }
}

impl<'a> RenderConfig<'a> {
    pub const WORLD: RenderConfig<'static> = RenderConfig {
        target: RenderConfigTarget::World,
        camera: RenderConfigCamera::WordCamera,
        intances: None,
        msaa: true,
        clear_color: None,
    };
    pub const UNIT_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::UnitCamera,
        ..Self::WORLD
    };
    pub const RELATIVE_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::RelativeCamera,
        ..Self::WORLD
    };
    pub const RELATIVE_BOTTOM_LEFT_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::RelativeCameraBottomLeft,
        ..Self::WORLD
    };
    pub const RELATIVE_BOTTOM_RIGHT_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::RelativeCameraBottomRight,
        ..Self::WORLD
    };
    pub const RELATIVE_TOP_LEFT_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::RelativeCameraTopLeft,
        ..Self::WORLD
    };
    pub const RELATIVE_TOP_RIGHT_WORLD: RenderConfig<'static> = RenderConfig {
        camera: RenderConfigCamera::RelativeCameraTopRight,
        ..Self::WORLD
    };
}

#[derive(Clone, Copy)]
pub enum RenderConfigCamera<'a> {
    WordCamera,
    UnitCamera,
    RelativeCamera,
    RelativeCameraBottomLeft,
    RelativeCameraBottomRight,
    RelativeCameraTopLeft,
    RelativeCameraTopRight,
    Custom(&'a CameraBuffer),
}

#[derive(Clone, Copy)]
pub enum RenderConfigInstances<'a> {
    Empty,
    SingleCenteredInstance,
    Custom(&'a InstanceBuffer),
}

#[derive(Clone, Copy)]
pub enum RenderConfigTarget<'a> {
    World,
    Custom(&'a RenderTarget),
}

/// Encoder of [Renderers](crate::Renderer) and utilities to copy, clear and render text onto [RenderTargets](crate::RenderTarget)
pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub defaults: &'a GpuDefaults,
    pub gpu: &'a Gpu,
}

impl<'a> RenderEncoder<'a> {
    pub fn new(gpu: &'a Gpu, defaults: &'a GpuDefaults) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        Self {
            inner: encoder,
            defaults,
            gpu,
        }
    }

    pub fn clear(&mut self, target: RenderConfigTarget, color: Color) {
        let target = match target {
            crate::RenderConfigTarget::World => &self.defaults.world_target,
            crate::RenderConfigTarget::Custom(c) => c,
        };
        self.inner.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.msaa(),
                resolve_target: Some(target.view()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub fn renderer<'b>(&'b mut self, config: RenderConfig<'b>) -> Renderer<'b> {
        Renderer::new(&mut self.inner, self.defaults, config)
    }

    #[cfg(feature = "text")]
    pub fn render_text(
        &mut self,
        target: RenderConfigTarget,
        gpu: &Gpu,
        descriptor: TextDescriptor,
    ) {
        if let Some(color) = descriptor.clear_color {
            self.clear(target, color);
        }
        let target = match target {
            crate::RenderConfigTarget::World => &self.defaults.world_target,
            crate::RenderConfigTarget::Custom(c) => c,
        };
        let target_size = target.size();
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        for section in descriptor.sections {
            descriptor
                .font
                .brush
                .queue(section.to_glyph_section(descriptor.resolution));
        }

        descriptor
            .font
            .brush
            .draw_queued(
                &gpu.device,
                &mut staging_belt,
                &mut self.inner,
                target.view(),
                (descriptor.resolution * target_size.x as f32) as u32,
                (descriptor.resolution * target_size.y as f32) as u32,
            )
            .expect("Draw queued");

        staging_belt.finish();
    }

    pub fn copy_to_target(&mut self, src: &Sprite, target: &RenderTarget) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.defaults,
            RenderConfig {
                target: RenderConfigTarget::Custom(target),
                camera: RenderConfigCamera::RelativeCamera,
                intances: Some(RenderConfigInstances::SingleCenteredInstance),
                msaa: false,
                clear_color: None,
            },
        );
        renderer.use_shader(&self.defaults.sprite_no_msaa);
        renderer.use_model(self.defaults.relative_camera.0.model());
        renderer.use_sprite(src, 1);
        renderer.draw(0);
    }

    pub fn render_each<'b, C: ComponentController>(
        &'b mut self,
        ctx: &'b Context<'b>,
        config: RenderConfig<'b>,
        mut each: impl FnMut(&mut Renderer<'b>, &'b C, InstanceIndex),
    ) {
        let mut renderer = self.renderer(config);
        for (buffer, components) in ctx.components.iter_render::<C>(ComponentFilter::Active) {
            renderer.use_instances(buffer);
            for (instance, component) in components {
                (each)(&mut renderer, component, instance);
            }
        }
    }

    pub fn render_all<'b, C: ComponentController>(
        &'b mut self,
        ctx: &'b Context<'b>,
        config: RenderConfig<'b>,
        mut all: impl FnMut(&mut Renderer<'b>, InstanceIndices),
    ) {
        let mut renderer = self.renderer(config);
        for (buffer, _) in ctx.components.iter_render::<C>(ComponentFilter::Active) {
            renderer.use_instances(buffer);
            (all)(&mut renderer, buffer.all_instances());
        }
    }

    pub fn finish(self) {
        self.gpu.commands.write().unwrap().push(self.inner.finish())
    }
}
