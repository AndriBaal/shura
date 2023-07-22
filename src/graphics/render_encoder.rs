use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, RenderTarget, Renderer, Sprite,
};

#[derive(Clone, Copy)]
/// Camera used for rendering. Allow to easily select a default camera from shura or
/// to use a custom camera. All default cameras are living inside the [GpuDefaults](crate::GpuDefaults).
pub enum RenderCamera<'a> {
    World,
    Unit,
    Relative,
    RelativeBottomLeft,
    RelativeBottomRight,
    RelativeTopLeft,
    RelativeTopRight,
    Custom(&'a CameraBuffer),
}

impl<'a> RenderCamera<'a> {
    pub fn camera(self, defaults: &'a GpuDefaults) -> &'a CameraBuffer {
        return match self {
            RenderCamera::World => &defaults.world_camera,
            RenderCamera::Unit => &defaults.unit_camera.0,
            RenderCamera::Relative => &defaults.relative_camera.0,
            RenderCamera::RelativeBottomLeft => &defaults.relative_bottom_left_camera.0,
            RenderCamera::RelativeBottomRight => &defaults.relative_bottom_right_camera.0,
            RenderCamera::RelativeTopLeft => &defaults.relative_top_left_camera.0,
            RenderCamera::RelativeTopRight => &defaults.relative_top_right_camera.0,
            RenderCamera::Custom(c) => c,
        };
    }
}

#[derive(Clone, Copy)]
/// Instances used for rendering
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

/// Encoder of [Renderers](crate::Renderer) and utilities to copy, clear and render text onto [RenderTargets](crate::RenderTarget)
pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub defaults: &'a GpuDefaults,
    pub gpu: &'a Gpu,
    // pub screenshot: bool
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

    pub fn clear(&mut self, target: &RenderTarget, color: Color) {
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

    pub fn renderer<'b>(
        &'b mut self,
        target: &'b RenderTarget,
        clear: Option<Color>,
        msaa: bool,
    ) -> Renderer<'b> {
        Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            target,
            msaa,
            clear,
        )
    }

    pub fn copy_to_target(&mut self, src: &Sprite, target: &RenderTarget) {
        let mut renderer =
            Renderer::new(&mut self.inner, self.defaults, self.gpu, target, true, None);
        renderer.use_camera_buffer(&self.defaults.relative_camera.0);
        renderer.use_instance_buffer(&self.defaults.single_centered_instance);
        renderer.use_shader(&self.defaults.sprite);
        renderer.use_model(self.defaults.relative_camera.0.model());
        renderer.use_sprite(src, 1);
        renderer.draw(0);
    }

    pub fn finish(self) -> wgpu::CommandBuffer {
        self.inner.finish()
    }

    pub fn submit(self, gpu: &Gpu) -> wgpu::SubmissionIndex {
        gpu.submit(self)
    }
}
