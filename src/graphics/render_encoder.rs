use crate::{CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, RenderTarget, Renderer, Instances};

pub enum RenderCamera<'a> {
    WorldCamera,
    RelativeCamera,
    Custom(&'a CameraBuffer),
}

pub enum RenderInstances<'a> {
    SingleInstance,
    Custom(&'a InstanceBuffer),
}

pub struct RenderEncoder<'a> {
    pub(crate) target: &'a RenderTarget,
    pub encoder: wgpu::CommandEncoder,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
}

impl<'a> RenderEncoder<'a> {
    pub fn new(gpu: &Gpu, defaults: &GpuDefaults, target: &'a RenderTarget) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self {
            encoder,
            target,
            gpu,
            defaults,
        }
    }

    pub fn submit(self, encoder: wgpu::CommandEncoder) {
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn clear(&mut self, color: Color) {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.target.target_msaa(),
                resolve_target: Some(self.target.target_view()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub fn renderer(
        &'a mut self,
        camera: RenderCamera<'a>,
        instances: RenderInstances<'a>,
    ) -> (Instances, Renderer<'a>) {
        Renderer::new(self, camera, instances)
    }

    // pub fn save_target(&self, into: &RenderTarget) {
    //     let gpu = self.gpu;
    //     let defaults = self.defaults;
    //     let relative_camera = &defaults.relative_camera;

    //     {
    //         let mut renderer =
    //             Renderer::new(&mut encoder, gpu, defaults, &target, relative_camera, None);
    //         renderer.use_uniform(relative_camera.uniform(), 0);
    //         renderer.set_instance_buffer(&defaults.single_centered_instance);
    //         renderer.render_sprite(
    //             relative_camera.model(),
    //             self.target.as_ref().unwrap().target(),
    //         );
    //         renderer.commit(0..1);
    //     }
    // }

    pub fn target(&self) -> &RenderTarget {
        self.target
    }
}
