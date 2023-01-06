use crate::{
    text::{DefaultLineBreaker, Font, LineBreaker, Text},
    Color, Context, Dimension, Sprite, Vector,
};

pub struct TextSection<'a> {
    pub position: Vector<f32>,
    pub bounds: Dimension<f32>,
    pub layout: LineBreaker<DefaultLineBreaker>,
    pub text: Vec<Text<'a>>,
}

pub struct TextDescriptor<'a> {
    pub clear_color: Option<Color>,
    pub size: Dimension<u32>,
    pub sections: Vec<TextSection<'a>>,
    pub font: Option<&'a mut Font>,
}

impl<'a> TextSection<'a> {
    fn to_glyph_section(self) -> wgpu_glyph::Section<'a> {
        wgpu_glyph::Section {
            screen_position: (self.position.x, self.position.y),
            bounds: (self.bounds.width, self.bounds.height),
            layout: self.layout,
            text: self.text,
        }
    }
}

impl<'a> Default for TextSection<'a> {
    fn default() -> Self {
        Self {
            position: Vector::new(0.0, 0.0),
            bounds: Dimension::new(f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![],
        }
    }
}

pub trait CreateText {
    fn new_text(ctx: &mut Context, descriptor: TextDescriptor) -> Sprite;
    fn write_text(&mut self, ctx: &mut Context, descriptor: TextDescriptor);
}

impl CreateText for Sprite {
    fn new_text(ctx: &mut Context, descriptor: TextDescriptor) -> Sprite {
        let mut sprite = Sprite::empty(ctx.gpu, descriptor.size);
        sprite.write_text(ctx, descriptor);
        return sprite;
    }

    /// The text is written on the current sprite.
    fn write_text(&mut self, ctx: &mut Context, descriptor: TextDescriptor) {
        if descriptor.size != *self.size() {
            *self = Sprite::empty(ctx.gpu, descriptor.size);
        }

        let gpu = &mut ctx.gpu;
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        let font = descriptor.font.unwrap_or(&mut gpu.defaults.default_font);
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
            font.queue(section.to_glyph_section());
        }

        font.draw_queued(
            &gpu.device,
            &mut staging_belt,
            &mut encoder,
            &view,
            descriptor.size.width,
            descriptor.size.height,
        )
        .expect("Draw queued");

        // Submit the work!
        staging_belt.finish();
        gpu.queue.submit(Some(encoder.finish()));
    }
}
