use std::ops::{Deref, DerefMut};

use crate::{
    graphics::{DefaultResources, Gpu, RenderEncoder, RenderTarget},
    gui::GuiContext,
    math::Vector2,
};
use egui_wgpu::renderer::{Renderer, ScreenDescriptor};
use egui_winit::State;
use instant::Duration;
use winit::window::Window;

pub struct Gui {
    state: State,
    context: GuiContext,
    renderer: OnceLock<Renderer>,
    screen_descriptor: ScreenDescriptor,
}

impl Gui {
    pub(crate) fn new(
        window: &Window,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        gpu: &Gpu,
    ) -> Self {
        let device = &gpu.device;
        let renderer = Renderer::new(device, gpu.format, None, gpu.base.sample_count);
        let state = State::new(
            egui::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            Some(gpu.device.limits().max_texture_dimension_2d as _),
        );
        let context = GuiContext::default();
        let size = gpu.render_size();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.x, size.y],
            pixels_per_point: 1.0,
        };
        Self {
            renderer,
            state,
            context,
            screen_descriptor,
        }
    }

    pub(crate) fn resize(&mut self, size: Vector2<u32>) {
        self.screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.x, size.y],
            pixels_per_point: 1.0,
        };
    }

    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        let _ = self.state.on_window_event(&self.context, event);
    }

    pub(crate) fn begin(&mut self, total_time: &Duration, window: &Window) {
        let mut egui_input = self.state.take_egui_input(window);
        egui_input.time = Some(total_time.as_secs_f64());
        self.context.begin_frame(egui_input);
    }

    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        default_resources: &DefaultResources,
        encoder: &mut RenderEncoder,
    ) {
        let output = self.context.end_frame();
        let paint_jobs = self.context.tessellate(output.shapes, 1.0);

        for add in &output.textures_delta.set {
            self.renderer
                .update_texture(&gpu.device, &gpu.queue, add.0, &add.1);
        }

        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &mut encoder.inner,
            &paint_jobs,
            &self.screen_descriptor,
        );

        {
            let target = &defaults.surface;
            let mut rpass = encoder
                .inner
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(target.attachment(None))],
                    depth_stencil_attachment: None,
                    label: Some("egui main render pass"),
                    timestamp_writes: None,
                    occlusion_query_set: None,
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
