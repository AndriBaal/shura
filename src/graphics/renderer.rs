#[cfg(feature = "text")]
use crate::text::{Font, LetterInstance2D, TextMesh};

use crate::graphics::{
    Camera, CameraBuffer, CameraBuffer2D, Color, DefaultAssets, DepthBuffer, Gpu, GpuId, Instance,
    InstanceBuffer, InstanceBuffer2D, InstanceBuffer3D, Mesh, Mesh2D, Model, RenderTarget, Shader,
    Sprite, SpriteSheet, Uniform, UniformData, Vertex,
};
use std::ops::Range;

struct RenderCache {
    pub bound_shader: Option<GpuId<wgpu::RenderPipeline>>,
    pub bound_buffers: [Option<GpuId<wgpu::Buffer>>; 3],
    pub bound_uniforms: [Option<GpuId<wgpu::BindGroup>>; 16],
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            bound_shader: None,
            bound_buffers: [None; 3],

            bound_uniforms: [None; 16],
        }
    }
}

pub struct Renderer<'a> {
    pub(crate) target: &'a dyn RenderTarget,
    pub gpu: &'a Gpu,
    pub default_assets: &'a DefaultAssets,
    pub indices: u32,
    pub instances: Range<u32>,
    render_pass: wgpu::RenderPass<'a>,
    cache: RenderCache,
}

impl<'a> Renderer<'a> {
    pub const MODEL_SLOT: u32 = 0;
    pub const INSTANCE_SLOT: u32 = 1;
    pub const CAMERA_SLOT: u32 = 0;
    pub fn new(
        render_encoder: &'a mut wgpu::CommandEncoder,
        default_assets: &'a DefaultAssets,
        gpu: &'a Gpu,
        target: &'a dyn RenderTarget,
        clear: Option<Color>,
        depth: Option<&'a DepthBuffer>,
    ) -> Renderer<'a> {
        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(target.attachment(clear))],
            depth_stencil_attachment: depth.map(|depth| wgpu::RenderPassDepthStencilAttachment {
                view: depth.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        Self {
            indices: 0,
            render_pass,
            default_assets,
            target,
            gpu,
            cache: RenderCache::default(),
            instances: 0..0,
        }
    }

    pub fn target(&self) -> &dyn RenderTarget {
        self.target
    }

    pub fn pass(&'a mut self) -> &mut wgpu::RenderPass {
        self.cache = Default::default();
        &mut self.render_pass
    }

    pub fn use_instances<I: Instance>(&mut self, buffer: &'a InstanceBuffer<I>) {
        let buffer_id: GpuId<wgpu::Buffer> = buffer.buffer().global_id();
        self.instances = buffer.instances();
        if self.cache.bound_buffers[Self::INSTANCE_SLOT as usize].map_or(true, |id| id != buffer_id)
        {
            self.cache.bound_buffers[Self::INSTANCE_SLOT as usize] = Some(buffer_id);
            self.render_pass
                .set_vertex_buffer(Self::INSTANCE_SLOT, buffer.slice());
        }
    }

    pub fn use_camera<C: Camera>(&mut self, camera: &'a CameraBuffer<C>) {
        self.use_uniform(camera.uniform(), Self::CAMERA_SLOT)
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        let pipeline = shader.pipeline();
        let pipeline_id = pipeline.global_id();
        if self.cache.bound_shader.map_or(true, |id| id != pipeline_id) {
            self.cache.bound_shader = Some(pipeline_id);
            self.render_pass.set_pipeline(shader.pipeline());
        }
    }

    pub fn use_shader_with_buffers<I: Instance, T: Vertex>(
        &mut self,
        shader: &'a Shader,
        instances: &'a InstanceBuffer<I>,
        mesh: &'a Mesh<T>,
    ) {
        debug_assert_eq!(shader.instance_size(), instances.instance_size());
        debug_assert_eq!(shader.vertex_size(), mesh.vertex_size());
        self.use_shader(shader);
        self.use_mesh(mesh);
        self.use_instances(instances);
    }

    pub fn use_mesh<T: Vertex>(&mut self, mesh: &'a Mesh<T>) {
        let vertex_buffer_id = mesh.buffer().global_id();
        self.indices = mesh.index_amount();
        if self.cache.bound_buffers[Self::MODEL_SLOT as usize]
            .map_or(true, |id| id != vertex_buffer_id)
        {
            self.cache.bound_buffers[Self::MODEL_SLOT as usize] = Some(vertex_buffer_id);
            self.render_pass
                .set_index_buffer(mesh.index_buffer(), wgpu::IndexFormat::Uint32);
            self.render_pass
                .set_vertex_buffer(Self::MODEL_SLOT, mesh.vertex_buffer());
        }
    }

    pub fn use_uniform(&mut self, uniform: &'a dyn Uniform, slot: u32) {
        let bind_group = uniform.bind_group();
        let bind_group_id = bind_group.global_id();
        if let Some(cache_slot) = self.cache.bound_uniforms.get_mut(slot as usize) {
            if cache_slot.map_or(true, |id| id != bind_group_id) {
                *cache_slot = Some(bind_group_id);
                self.render_pass.set_bind_group(slot, bind_group, &[]);
            }
        } else {
            self.render_pass.set_bind_group(slot, bind_group, &[]);
        }
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.use_uniform(sprite, slot);
    }

    pub fn use_sprite_sheet(&mut self, sprite_sheet: &'a SpriteSheet, slot: u32) {
        self.use_uniform(sprite_sheet, slot);
    }

    pub fn use_uniform_data<T: bytemuck::Pod>(&mut self, uniform: &'a UniformData<T>, slot: u32) {
        self.use_uniform(uniform, slot);
    }

    pub fn draw(&mut self) {
        self.draw_custom(0..self.indices, 0, self.instances.clone());
    }

    pub fn draw_custom(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances)
    }

    pub fn render<I: Instance, V: Vertex>(
        &mut self,
        shader: &'a Shader,
        buffer: &'a InstanceBuffer<I>,
        mesh: &'a Mesh<V>,
        uniforms: &[&'a dyn Uniform],
    ) {
        self.use_shader_with_buffers(shader, buffer, mesh);
        for (i, uniform) in uniforms.iter().enumerate() {
            self.use_uniform(*uniform, i as u32);
        }
        self.draw();
    }

    pub fn render_sprite(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.sprite, buffer, mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw();
        }
    }

    pub fn render_sprite_sheet(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
        sprite: &'a SpriteSheet,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.sprite_sheet, buffer, mesh);
            self.use_camera(camera);
            self.use_sprite_sheet(sprite, 1);
            self.draw();
        }
    }

    pub fn render_color(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.color, buffer, mesh);
            self.use_camera(camera);
            self.draw();
        }
    }

    #[cfg(feature = "text")]
    pub fn render_text_mesh(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        text: &'a TextMesh,
    ) {
        if buffer.buffer_size() != 0 && text.mesh().vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.text_mesh, buffer, text.mesh());
            self.use_camera(camera);
            self.use_mesh(text.mesh());
            self.use_sprite_sheet(text.font().sprite_sheet(), 1);
            self.draw();
        }
    }

    #[cfg(feature = "text")]
    pub fn render_text(
        &mut self,
        buffer: &'a InstanceBuffer<LetterInstance2D>,
        camera: &'a CameraBuffer2D,
        font: &'a Font,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(
                &self.default_assets.text_instance,
                buffer,
                self.default_assets.unit_mesh(),
            );
            self.use_camera(camera);
            self.use_sprite_sheet(font.sprite_sheet(), 1);
            self.draw();
        }
    }

    pub fn render_grey(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.grey, buffer, mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw();
        }
    }

    pub fn render_blurred(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.blurr, buffer, mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw();
        }
    }

    pub fn render_rainbow(
        &mut self,
        buffer: &'a InstanceBuffer2D,
        camera: &'a CameraBuffer2D,
        mesh: &'a Mesh2D,
    ) {
        if buffer.buffer_size() != 0 && mesh.vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.default_assets.rainbow, buffer, mesh);
            self.use_camera(camera);
            self.use_uniform_data(&self.default_assets.times, 1);
            self.draw();
        }
    }

    pub fn render_model<C: Camera>(
        &mut self,
        buffer: &'a InstanceBuffer3D,
        camera: &'a CameraBuffer<C>,
        model: &'a Model,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader(&self.default_assets.model);
            self.use_instances(buffer);
            self.use_camera(camera);
            for mesh in &model.meshes {
                if mesh.1.vertex_buffer_size() != 0 {
                    let sprite = if let Some(index) = mesh.0 {
                        &model.sprites[index]
                    } else {
                        &self.default_assets.missing
                    };
                    self.use_sprite(sprite, 1);
                    self.use_mesh(&mesh.1);
                    self.draw();
                }
            }
        }
    }
}
