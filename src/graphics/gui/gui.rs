use std::ops::{Deref, DerefMut};

use crate::{gui::GuiContext, Gpu, Vector};
use egui_wgpu::renderer::{Renderer, ScreenDescriptor};
use egui_winit::State;
use instant::Duration;
use winit::window::Window;

pub struct Gui {
    state: State,
    // TODO: Maybe move to scene
    context: GuiContext,
    renderer: Renderer,
    screen_descriptor: ScreenDescriptor,
}

impl Gui {
    pub(crate) fn new(
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        gpu: &Gpu,
    ) -> Self {
        let config = &gpu.config;
        let device = &gpu.device;
        // TODO: Implement msaa_samnples and render to the target and not the surface texture
        let renderer = Renderer::new(device, config.format, None, 1);
        let state = State::new(event_loop);
        let context = GuiContext::default();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: 1.0,
        };
        Self {
            renderer,
            state,
            context,
            screen_descriptor,
        }
    }

    pub(crate) fn resize(&mut self, size: &Vector<u32>) {
        self.screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.x, size.y],
            pixels_per_point: 1.0,
        };
    }

    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        self.state.on_event(&self.context, event).consumed;
    }

    pub(crate) fn begin(&mut self, total_time: &Duration, window: &Window) {
        let mut egui_input = self.state.take_egui_input(window);
        egui_input.time = Some(total_time.as_secs_f64());
        self.context.begin_frame(egui_input);
    }

    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let output = self.context.end_frame();
        let paint_jobs = self.context.tessellate(output.shapes);

        for add in &output.textures_delta.set {
            self.renderer
                .update_texture(&gpu.device, &gpu.queue, add.0, &add.1);
        }

        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            encoder,
            &paint_jobs,
            &self.screen_descriptor,
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui main render pass"),
            });

            self.renderer
                .render(&mut rpass, &paint_jobs, &self.screen_descriptor);
        }

        for free in &output.textures_delta.free {
            self.renderer.free_texture(free);
        }
    }
}

impl Deref for Gui {
    type Target = GuiContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl DerefMut for Gui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}
