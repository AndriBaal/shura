use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, InstanceIndices, Model, RenderCamera,
    RenderConfigInstances, RenderTarget, Shader, Sprite, SpriteSheet, Uniform, Vector, ComponentManager, ComponentController, InstanceIndex, Context,
};
use std::ops::{Range, DerefMut, Deref};

#[cfg(feature = "text")]
use crate::text::{FontBrush, TextSection};

pub struct ComponentRenderer<'a>{
    pub renderer: Renderer<'a>,
    // pub components: &'a ComponentManager,
    pub screenshot: Option<&'a RenderTarget>,
}

impl <'a>Deref for ComponentRenderer<'a> {
    type Target=Renderer<'a>;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}

impl <'a>DerefMut for ComponentRenderer<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.renderer
    }
}

impl <'a>ComponentRenderer<'a> {

    // pub fn render_each<C: ComponentController>(
    //     &'a mut self,
    //     camera: RenderCamera<'a>,
    //     each: impl FnMut(&mut Renderer<'a>, &C, InstanceIndex),
    // ) {
    //     self.components.type_render::<C>().render_each(&mut self.renderer, camera, each)
    // }

    // pub fn render_single<C: ComponentController>(
    //     &'a mut self,
    //     camera: RenderCamera<'a>,
    //     each: impl FnOnce(&mut Renderer<'a>, &C, InstanceIndex),
    // ) {
    //     self.components.type_render::<C>().render_single(&mut self.renderer, camera, each)
    // }

    pub fn render_all<C: ComponentController>(
        &mut self,
        ctx: &'a Context<'a>,
        camera: RenderCamera<'a>,
        all: impl FnMut(&mut Renderer<'a>, InstanceIndices),
    ) {
        ctx.components.type_render::<C>().render_all(&mut self.renderer, camera, all)
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
        render_pass.set_index_buffer(model.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
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
        self.render_pass
            .set_vertex_buffer(Self::INSTANCE_SLOT, buffer.slice());
    }

    pub fn use_camera_buffer(&mut self, camera: &'a CameraBuffer) {
        self.render_pass
            .set_bind_group(Self::CAMERA_SLOT, camera.uniform().bind_group(), &[]);
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
        self.render_pass.set_pipeline(shader.pipeline());
    }

    pub fn use_model(&mut self, model: &'a Model) {
        self.indices = model.amount_of_indices();
        self.render_pass
            .set_index_buffer(model.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
        self.render_pass
            .set_vertex_buffer(Self::MODEL_SLOT, model.vertex_buffer().slice(..));
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
        self.render_pass.set_bind_group(slot, bind_group, &[]);
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
