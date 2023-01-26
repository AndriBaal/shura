use crate::{Dimension, Gpu};
use egui::{Context, TextureId, TexturesDelta};
use egui_wgpu::renderer::{Renderer, ScreenDescriptor};
use egui_winit::State;
use winit::window::Window;

pub struct Gui {
    pub state: State,
    gui_context: Context,
    renderer: Renderer,
    screen_descriptor: ScreenDescriptor,
}

impl Gui {
    pub(crate) fn new(
        window: &Window,
        event_loop: &winit::event_loop::EventLoop<()>,
        gpu: &Gpu,
    ) -> Self {
        let config = &gpu.config;
        let device = &gpu.device;
        let renderer = Renderer::new(device, config.format, None, 1);
        let state = State::new(event_loop);
        let gui_context = Default::default();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        Self {
            renderer,
            state,
            gui_context,
            screen_descriptor,
            tdelta: Default::default(),
        }
    }

    pub(crate) fn resize(&mut self, window: &Window, size: &Dimension<u32>) {
        self.screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: window.scale_factor() as f32,
        };
    }

    pub(crate) fn handle_event<T>(&mut self, event: &winit::event::WindowEvent) {
        self.state.on_event(&self.gui_context, event);
    }

    pub(crate) fn begin(&mut self, window: &Window) {
        self.gui_context.begin_frame(self.state.take_egui_input(window));
    }

    pub fn context(&self) -> Context {
        self.gui_context.clone()
    }

    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
    ) {
        let test = TextureId::User(25);
        let output = self.gui_context.end_frame();
        let paint_jobs = self.gui_context.tessellate(output.shapes);
        output.shapes[0].1.texture_id();
        let id = TextureId::User(0);
        self.renderer
            .update_texture(&gpu.device, &gpu.queue, test, &output.textures_delta);
        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            encoder,
            &paint_jobs,
            &self.screen_descriptor,
        );

        // Record all render passes.

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

        self.renderer.render(&mut rpass, &paint_jobs, &self.screen_descriptor);
        self.renderer.free_texture(output.textures_delta);
    }
}
