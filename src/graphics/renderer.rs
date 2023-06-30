use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, InstanceIndices, Model,
    RenderCamera, RenderConfigInstances, RenderTarget, Shader,
    Sprite, SpriteSheet, Uniform, Vector,
};
use std::{ops::Range, ptr::null};

#[cfg(feature = "text")]
use crate::text::{FontBrush, TextSection};

struct RenderCache {
    pub bound_shader: *const Shader,
    pub bound_camera: *const CameraBuffer,
    pub bound_vertex_buffer: *const wgpu::Buffer,
    pub bound_index_buffer: *const wgpu::Buffer,
    pub bound_instances: *const InstanceBuffer,
    pub bound_uniforms: [*const wgpu::BindGroup; 5],
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            bound_shader: null(),
            bound_camera: null(),
            bound_vertex_buffer: null(),
            bound_index_buffer: null(),
            bound_instances: null(),
            bound_uniforms: [null(), null(), null(), null(), null()],
        }
    }
}

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    pub screenshot: Option<&'a RenderTarget>,
    target: &'a RenderTarget,
    msaa: bool,
    indices: u32,
    render_pass: wgpu::RenderPass<'a>,
    cache: RenderCache,
}

impl<'a> Renderer<'a> {
    pub const MODEL_SLOT: u32 = 0;
    pub const INSTANCE_SLOT: u32 = 1;
    pub const CAMERA_SLOT: u32 = 0;
    pub fn new(
        render_encoder: &'a mut wgpu::CommandEncoder,
        defaults: &'a GpuDefaults,
        gpu: &'a Gpu,
        target: &'a RenderTarget,
        msaa: bool,
        clear: Option<Color>,
    ) -> Renderer<'a> {
        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: if msaa { target.msaa() } else { target.view() },
                resolve_target: if msaa { Some(target.view()) } else { None },
                ops: wgpu::Operations {
                    load: if let Some(clear_color) = clear {
                        wgpu::LoadOp::Clear(clear_color.into())
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        return Self {
            render_pass,
            indices: 0,
            msaa: msaa,
            cache: Default::default(),
            defaults: defaults,
            target,
            gpu,
            screenshot: None,
        };
    }

    pub(crate) fn output_renderer(
        encoder: &'a mut wgpu::CommandEncoder,
        output: &'a wgpu::TextureView,
        defaults: &'a GpuDefaults,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        let shader = &defaults.sprite_no_msaa;
        let sprite = &defaults.world_target.sprite();
        let model = &defaults.relative_camera.0.model();

        render_pass.set_bind_group(
            Self::CAMERA_SLOT,
            &defaults.relative_camera.0.uniform().bind_group(),
            &[],
        );
        render_pass.set_vertex_buffer(
            Self::INSTANCE_SLOT,
            defaults.single_centered_instance.slice(),
        );
        render_pass.set_pipeline(shader.pipeline());
        render_pass.set_bind_group(1, sprite.bind_group(), &[]);
        render_pass.set_vertex_buffer(Self::MODEL_SLOT, model.vertex_buffer().slice(..));
        render_pass.set_index_buffer(model.index_buffer(defaults).slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..model.amount_of_indices(), 0, 0..1);

        // renderer.use_camera_buffer(&defaults.relative_camera.0);
        // renderer.use_instance_buffer(&defaults.single_centered_instance);
        // renderer.use_shader(&ctx.defaults.sprite_no_msaa);
        // renderer.use_model(ctx.defaults.relative_camera.0.model());
        // renderer.use_sprite(ctx.defaults.world_target.sprite(), 1);
        // renderer.draw(0);
    }

    pub fn target(&self) -> &RenderTarget {
        self.target
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instance_buffer(&mut self, buffer: &'a InstanceBuffer) {
        let ptr = buffer as *const _;
        if ptr != self.cache.bound_instances {
            self.cache.bound_instances = ptr;
            self.render_pass
                .set_vertex_buffer(Self::INSTANCE_SLOT, buffer.slice());
        }
    }

    pub fn use_camera_buffer(&mut self, camera: &'a CameraBuffer) {
        let ptr = camera as *const _;
        if ptr != self.cache.bound_camera {
            self.cache.bound_camera = ptr;
            self.render_pass
                .set_bind_group(Self::CAMERA_SLOT, camera.uniform().bind_group(), &[]);
        }
    }

    pub fn use_instances(&mut self, instances: RenderConfigInstances<'a>) {
        let buffer = instances.instances(self.defaults);
        self.use_instance_buffer(buffer);
    }

    pub fn use_camera(&mut self, camera: RenderCamera<'a>) {
        let buffer = camera.camera(self.defaults);
        self.use_camera_buffer(buffer);
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        assert_eq!(
            shader.msaa(),
            self.msaa,
            "The Renderer and the Shader both need to have msaa enabled / disabled!"
        );
        let ptr = shader as *const _;
        if ptr != self.cache.bound_shader {
            self.cache.bound_shader = ptr;
            self.render_pass.set_pipeline(shader.pipeline());
        }
    }

    pub fn use_model(&mut self, model: &'a Model) {
        let index_buffer = model.index_buffer(self.defaults);
        let index_ptr = index_buffer as *const _;
        let vertex_ptr = model.vertex_buffer() as *const _;

        if index_ptr != self.cache.bound_index_buffer {
            self.cache.bound_index_buffer = index_ptr;

            self.render_pass
                .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            self.indices = model.amount_of_indices();
        }

        if vertex_ptr != self.cache.bound_vertex_buffer {
            self.cache.bound_vertex_buffer = vertex_ptr;
            self.render_pass
                .set_vertex_buffer(Self::MODEL_SLOT, model.vertex_buffer().slice(..));
        }
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.use_bind_group(sprite.bind_group(), slot);
    }

    pub fn use_sprite_sheet(&mut self, sprite_sheet: &'a SpriteSheet, slot: u32) {
        self.use_bind_group(sprite_sheet.bind_group(), slot);
    }

    pub fn use_uniform<T: bytemuck::Pod>(&mut self, uniform: &'a Uniform<T>, slot: u32) {
        self.use_bind_group(uniform.bind_group(), slot);
    }

    pub fn use_bind_group(&mut self, bind_group: &'a wgpu::BindGroup, slot: u32) {
        let ptr = bind_group as *const _;
        if let Some(cache_slot) = self.cache.bound_uniforms.get_mut(slot as usize) {
            if *cache_slot != ptr {
                *cache_slot = ptr;
                self.render_pass.set_bind_group(slot, bind_group, &[]);
            }
        } else {
            self.render_pass.set_bind_group(slot, bind_group, &[]);
        }
    }

    pub fn draw(&mut self, instances: impl Into<InstanceIndices>) {
        self.draw_indexed(0..self.indices, instances);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, instances: impl Into<InstanceIndices>) {
        self.render_pass
            .draw_indexed(indices, 0, instances.into().range);
    }

    pub const fn msaa(&self) -> bool {
        self.msaa
    }

    #[cfg(feature = "text")]
    pub fn render_font(&mut self, font: &'a FontBrush) {
        self.cache = Default::default();
        font.render(
            self.gpu,
            &mut self.render_pass,
            self.target.size().cast::<f32>(),
        )
    }

    #[cfg(feature = "text")]
    pub fn queue_text(
        &mut self,
        camera: RenderCamera,
        font: &'a FontBrush,
        sections: Vec<TextSection>,
    ) {
        font.queue(self.defaults, camera, self.target.size(), sections);
    }

    pub fn render_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_sheet(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a SpriteSheet,
    ) {
        self.use_shader(&self.defaults.sprite_sheet);
        self.use_model(model);
        self.use_sprite_sheet(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_sheet_uniform(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a SpriteSheet,
        sprite_index: &'a Uniform<Vector<i32>>,
    ) {
        self.use_shader(&self.defaults.sprite_sheet_uniform);
        self.use_model(model);
        self.use_sprite_sheet(sprite, 1);
        self.use_uniform(sprite_index, 2);
        self.draw(instances);
    }

    pub fn render_color(&mut self, instances: impl Into<InstanceIndices>, model: &'a Model) {
        self.use_shader(&self.defaults.color);
        self.use_model(model);
        self.draw(instances);
    }

    pub fn render_color_uniform(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color_uniform);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.draw(instances);
    }

    pub fn render_grey(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.grey);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_blurred(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.blurr);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_no_msaa(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite_no_msaa);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_rainbow(&mut self, instances: impl Into<InstanceIndices>, model: &'a Model) {
        self.use_shader(&self.defaults.rainbow);
        self.use_model(model);
        self.use_uniform(&self.defaults.times, 1);
        self.draw(instances);
    }
}
