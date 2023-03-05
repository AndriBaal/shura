use crate::{
    text::{DefaultLineBreaker, Font, LineBreaker, Text},
    Color, Gpu, Sprite, Vector,
};

pub struct TextSection<'a> {
    pub position: Vector<f32>,
    pub bounds: Vector<f32>,
    pub layout: LineBreaker<DefaultLineBreaker>,
    pub text: Vec<Text<'a>>,
}

pub struct TextDescriptor<'a> {
    pub clear_color: Option<Color>,
    pub size: Vector<u32>,
    pub sections: Vec<TextSection<'a>>,
    pub font: &'a mut Font,
}

impl<'a> TextSection<'a> {
    fn to_glyph_section(self) -> wgpu_glyph::Section<'a> {
        wgpu_glyph::Section {
            screen_position: (self.position.x, self.position.y),
            bounds: (self.bounds.x, self.bounds.y),
            layout: self.layout,
            text: self.text,
        }
    }
}

impl<'a> Default for TextSection<'a> {
    fn default() -> Self {
        Self {
            position: Vector::new(0.0, 0.0),
            bounds: Vector::new(f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![],
        }
    }
}

pub trait CreateText {
    fn new_text(gpu: &Gpu, descriptor: TextDescriptor) -> Sprite;
    fn write_text(&mut self, gpu: &Gpu, descriptor: TextDescriptor);
}

impl CreateText for Sprite {
    fn new_text(gpu: &Gpu, descriptor: TextDescriptor) -> Sprite {
        let mut sprite = Sprite::empty(gpu, descriptor.size);
        sprite.write_text(gpu, descriptor);
        return sprite;
    }

    /// The text is written on the current sprite.
    fn write_text(&mut self, gpu: &Gpu, descriptor: TextDescriptor) {
        if descriptor.size != *self.size() {
            *self = Sprite::empty(gpu, descriptor.size);
        }

        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_text"),
            });

        let view = self
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_render_text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if let Some(color) = descriptor.clear_color {
                            wgpu::LoadOp::Clear(color.into())
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        for section in descriptor.sections {
            descriptor.font.queue(section.to_glyph_section());
        }

        descriptor
            .font
            .draw_queued(
                &gpu.device,
                &mut staging_belt,
                &mut encoder,
                &view,
                descriptor.size.x,
                descriptor.size.y,
            )
            .expect("Draw queued");

        staging_belt.finish();
        gpu.queue.submit(Some(encoder.finish()));
    }
}
