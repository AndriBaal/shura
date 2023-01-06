use crate::{Dimension, Gpu};
use std::mem;
use egui::{Context, FontDefinitions, TexturesDelta};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use instant::Duration;
use winit::window::Window;

pub struct Gui {
    pub platform: Platform,
    renderer: RenderPass,
    screen_descriptor: ScreenDescriptor,
    tdelta: TexturesDelta
}

impl Gui {
    pub(crate) fn new(window: &Window, gpu: &Gpu) -> Self {
        let config = &gpu.config;
        let device = &gpu.device;
        let renderer = RenderPass::new(device, config.format, 1);
        let platform = Platform::new(PlatformDescriptor {
            physical_width: config.width as u32,
            physical_height: config.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let screen_descriptor = ScreenDescriptor {
            physical_width: config.width,
            physical_height: config.height,
            scale_factor: window.scale_factor() as f32,
        };
        Gui {
            renderer,
            platform,
            screen_descriptor,
            tdelta: Default::default()
        }
    }

    pub(crate) fn resize(&mut self, window: &Window, size: &Dimension<u32>) {
        self.screen_descriptor = ScreenDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor() as f32,
        };
    }

    pub(crate) fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.platform.handle_event(event);
    }

    pub(crate) fn begin(&mut self, total_time: Duration) {
        self.renderer
            .remove_textures(mem::take(&mut self.tdelta))
            .expect("remove texture ok");
        self.platform.update_time(total_time.as_secs_f64());
        self.platform.begin_frame();
    }

    pub fn context(&self) -> Context {
        self.platform.context()
    }

    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
    ) {
        let output = self.platform.end_frame(Some(window));
        let paint_jobs = self.platform.context().tessellate(output.shapes);
        self.tdelta = output.textures_delta;
        self.renderer
            .add_textures(&gpu.device, &gpu.queue, &self.tdelta)
            .expect("add texture ok");
        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &paint_jobs,
            &self.screen_descriptor,
        );

        // Record all render passes.
        self.renderer
            .execute(encoder, view, &paint_jobs, &self.screen_descriptor, None)
            .unwrap();
    }
}
