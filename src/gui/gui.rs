use std::ops::{Deref, DerefMut};

use crate::{
    graphics::{Gpu, RenderEncoder, RenderTarget, SurfaceRenderTarget},
    gui::GuiContext,
    math::Vector2,
};
use egui::mutex::Mutex;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use instant::Duration;
use winit::window::Window;

pub struct Gui {
    state: State,
    context: GuiContext,
    renderer: Renderer,
    screen_descriptor: Mutex<ScreenDescriptor>,
}

impl Gui {
    pub(crate) fn new(window: &Window, gpu: &Gpu) -> Self {
        let context = GuiContext::default();
        let state = State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            Some(gpu.device.limits().max_texture_dimension_2d as _),
        );
        let size = window.inner_size();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: 1.0,
        };
        Self {
            renderer: Renderer::new(&gpu.device, gpu.format(), None, gpu.samples()),
            state,
            context,
            screen_descriptor: Mutex::new(screen_descriptor),
        }
    }

    pub(crate) fn resize(&mut self, size: Vector2<u32>) {
        *self.screen_descriptor.lock() = ScreenDescriptor {
            size_in_pixels: [size.x, size.y],
            pixels_per_point: 1.0,
        };
    }

    pub(crate) fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub(crate) fn begin(&mut self, total_time: &Duration, window: &Window) {
        let mut egui_input = self.state.take_egui_input(window);
        egui_input.time = Some(total_time.as_secs_f64());
        self.context.begin_frame(egui_input);
    }

    pub(crate) fn render(
        &mut self,
        surface_target: &SurfaceRenderTarget,
        gpu: &Gpu,
        encoder: &mut RenderEncoder,
    ) {
        let output = self.context.end_frame();
        let paint_jobs = self.context.tessellate(output.shapes, 1.0);

        for add in &output.textures_delta.set {
            self.renderer
                .update_texture(&gpu.device, &gpu.queue, add.0, &add.1);
        }

        let screen_descriptor = self.screen_descriptor.lock();
        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &mut encoder.inner,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut rpass = encoder
                .inner
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(surface_target.attachment(None))],
                    depth_stencil_attachment: None,
                    label: Some("egui main render pass"),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            self.renderer
                .render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        for free in &output.textures_delta.free {
            self.renderer.free_texture(free);
        }
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.screen_descriptor.lock().pixels_per_point
    }

    pub fn set_pixels_per_point(&self, value: f32) {
        self.screen_descriptor.lock().pixels_per_point = value
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
