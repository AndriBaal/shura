use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, Instances, RenderTarget, Renderer,
};

pub enum RenderCamera<'a> {
    WorldCamera,
    RelativeCamera,
    Custom(&'a CameraBuffer),
}

pub enum RenderInstances<'a> {
    SingleInstance,
    Custom(&'a InstanceBuffer),
}

pub struct RenderConfig<'a> {
    pub camera: RenderCamera<'a>,
    pub instances: RenderInstances<'a>,
    pub target: &'a RenderTarget,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
}

pub struct RenderEncoder {
    pub encoder: wgpu::CommandEncoder,
}

impl RenderEncoder {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self { encoder }
    }

    pub fn clear(&mut self, target: &RenderTarget, color: Color) {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.target_msaa(),
                resolve_target: Some(target.target_view()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub fn renderer<'a>(&'a mut self, temp: RenderConfig<'a>) -> (Instances, Renderer<'a>) {
        Renderer::new(self, temp)
    }

}
