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

impl<'a> RenderConfigCamera<'a> {
    pub fn camera(self, defaults: &'a GpuDefaults) -> &'a CameraBuffer {
        return match self {
            RenderConfigCamera::WordCamera => &defaults.world_camera,
            RenderConfigCamera::UnitCamera => &defaults.unit_camera.0,
            RenderConfigCamera::RelativeCamera => &defaults.relative_camera.0,
            RenderConfigCamera::RelativeCameraBottomLeft => &defaults.relative_bottom_left_camera.0,
            RenderConfigCamera::RelativeCameraBottomRight => {
                &defaults.relative_bottom_right_camera.0
            }
            RenderConfigCamera::RelativeCameraTopLeft => &defaults.relative_top_left_camera.0,
            RenderConfigCamera::RelativeCameraTopRight => &defaults.relative_top_right_camera.0,
            RenderConfigCamera::Custom(c) => c,
        };
    }
}

#[derive(Clone, Copy)]
pub enum RenderConfigInstances<'a> {
    Empty,
    SingleCenteredInstance,
    Custom(&'a InstanceBuffer),
}

impl<'a> RenderConfigInstances<'a> {
    pub fn instances(self, defaults: &'a GpuDefaults) -> &'a InstanceBuffer {
        return match self {
            RenderConfigInstances::Empty => &defaults.empty_instance,
            RenderConfigInstances::SingleCenteredInstance => &defaults.single_centered_instance,
            RenderConfigInstances::Custom(c) => c,
        };
    }
}

#[derive(Clone, Copy)]
pub enum RenderConfigTarget<'a> {
    World,
    Custom(&'a RenderTarget),
}

impl<'a> RenderConfigTarget<'a> {
    pub fn target(self, defaults: &'a GpuDefaults) -> &'a RenderTarget {
        return match self {
            RenderConfigTarget::World => &defaults.world_target,
            RenderConfigTarget::Custom(c) => c,
        };
    }
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
        Renderer::new(&mut self.inner, self.defaults, self.gpu, config)
    }

    pub fn copy_to_target(&mut self, src: &Sprite, target: &RenderTarget) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
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
