#[cfg(feature = "text")]
use crate::text::{Font, TextMesh};

use crate::graphics::{
    AssetManager, Camera, CameraBuffer, CameraBuffer2D, Color, ColorInstance2D, ColorMesh2D,
    DefaultAssets, DepthBuffer, Gpu, GpuId, Instance, Instance3D, InstanceBuffer, Mesh, Model,
    PositionInstance2D, PositionMesh2D, RenderTarget, Shader, Sprite, SpriteArray,
    SpriteArrayCropInstance2D, SpriteArrayMesh2D, SpriteCropInstance2D, SpriteMesh2D, Uniform,
    UniformData, Vertex,
};
use std::ops::Range;

#[derive(Default)]
struct RenderCache {
    pub bound_shader: Option<GpuId<wgpu::RenderPipeline>>,
    pub bound_buffers: [Option<GpuId<wgpu::Buffer>>; 3],
    pub bound_uniforms: [Option<GpuId<wgpu::BindGroup>>; 16],
}

pub struct Renderer<'a> {
    pub(crate) target: &'a dyn RenderTarget,
    pub gpu: &'a Gpu,
    pub assets: &'a AssetManager,
    pub default_assets: &'a DefaultAssets,
    pub indices: u32,
    pub instances: Range<u32>,
    render_pass: wgpu::RenderPass<'a>,
    cache: RenderCache,
    shader_uses_instancing: bool,
}

impl<'a> Renderer<'a> {
    pub const MODEL_SLOT: u32 = 0;
    pub const INSTANCE_SLOT: u32 = 1;
    pub const CAMERA_SLOT: u32 = 0;
    pub fn new(
        render_encoder: &'a mut wgpu::CommandEncoder,
        assets: &'a AssetManager,
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
            assets,
            default_assets,
            target,
            gpu,
            cache: RenderCache::default(),
            instances: 0..0,
            shader_uses_instancing: false,
        }
    }

    pub fn target(&self) -> &dyn RenderTarget {
        self.target
    }

    pub fn pass(self) -> wgpu::RenderPass<'a> {
        self.render_pass
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.render_pass.set_scissor_rect(x, y, width, height)
    }

    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        self.render_pass
            .set_viewport(x, y, w, h, min_depth, max_depth)
    }

    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.render_pass.set_stencil_reference(reference)
    }

    pub fn use_instances<I: Instance>(&mut self, instances: &InstanceBuffer<I>) {
        self.use_instances_with_range(instances, instances.instances());
    }

    pub fn use_instances_with_range<I: Instance>(
        &mut self,
        instances: &InstanceBuffer<I>,
        range: Range<u32>,
    ) {
        let buffer_id: GpuId<wgpu::Buffer> = instances.buffer().global_id();
        self.instances = range;
        if self.cache.bound_buffers[Self::INSTANCE_SLOT as usize].map_or(true, |id| id != buffer_id)
        {
            self.cache.bound_buffers[Self::INSTANCE_SLOT as usize] = Some(buffer_id);
            self.render_pass
                .set_vertex_buffer(Self::INSTANCE_SLOT, instances.slice());
        }
    }

    pub fn use_camera<C: Camera>(&mut self, camera: &CameraBuffer<C>) {
        self.use_uniform(camera.uniform(), Self::CAMERA_SLOT)
    }

    pub fn use_shader(&mut self, shader: &Shader) {
        let pipeline = shader.pipeline();
        let pipeline_id = pipeline.global_id();
        if self.cache.bound_shader.map_or(true, |id| id != pipeline_id) {
            self.cache.bound_shader = Some(pipeline_id);
            self.render_pass.set_pipeline(shader.pipeline());
            self.shader_uses_instancing = shader.instance_size() != 0;
        }
    }

    pub fn use_mesh<T: Vertex>(&mut self, mesh: &Mesh<T>) {
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

    pub fn use_uniform(&mut self, uniform: &dyn Uniform, slot: u32) {
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

    pub fn use_sprite(&mut self, sprite: &Sprite, slot: u32) {
        self.use_uniform(sprite, slot);
    }

    pub fn use_sprite_array(&mut self, sprite_array: &SpriteArray, slot: u32) {
        self.use_uniform(sprite_array, slot);
    }

    pub fn use_uniform_data<T: bytemuck::Pod>(&mut self, uniform: &UniformData<T>, slot: u32) {
        self.use_uniform(uniform, slot);
    }

    pub fn render(&mut self) {
        if self.shader_uses_instancing {
            self.render_custom(0..self.indices, 0, self.instances.clone());
        } else {
            self.render_custom(0..self.indices, 0, 0..1);
        }
    }

    pub fn render_custom(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances)
    }

    pub fn draw<I: Instance, V: Vertex>(
        &mut self,
        shader: &Shader,
        instances: &InstanceBuffer<I>,
        mesh: &Mesh<V>,
        uniforms: &[&dyn Uniform],
    ) {
        if instances.buffer_size() != 0
            && mesh.vertex_buffer_size() != 0
            && mesh.index_buffer_size() != 0
        {
            self.use_shader(shader);
            self.use_instances(instances);
            self.use_mesh(mesh);
            for (i, uniform) in uniforms.iter().enumerate() {
                self.use_uniform(*uniform, i as u32);
            }
            self.render();
        }
    }

    pub fn draw_mesh<V: Vertex>(
        &mut self,
        shader: &Shader,
        mesh: &Mesh<V>,
        uniforms: &[&dyn Uniform],
    ) {
        if mesh.vertex_buffer_size() != 0 && mesh.index_buffer_size() != 0 {
            self.use_shader(shader);
            self.use_mesh(mesh);
            for (i, uniform) in uniforms.iter().enumerate() {
                self.use_uniform(*uniform, i as u32);
            }
            self.render();
        }
    }

    pub fn draw_sprite(
        &mut self,
        instances: &InstanceBuffer<PositionInstance2D>,
        mesh: &SpriteMesh2D,
        camera: &CameraBuffer2D,
        sprite: &Sprite,
    ) {
        if instances.buffer_size() != 0
            && mesh.vertex_buffer_size() != 0
            && mesh.index_buffer_size() != 0
        {
            self.use_shader(&self.default_assets.sprite_shader);
            self.use_instances(instances);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.render();
        }
    }

    pub fn draw_sprite_array(
        &mut self,
        instances: &InstanceBuffer<SpriteArrayCropInstance2D>,
        mesh: &SpriteMesh2D,
        camera: &CameraBuffer2D,
        sprite: &Sprite,
    ) {
        if instances.buffer_size() != 0
            && mesh.vertex_buffer_size() != 0
            && mesh.index_buffer_size() != 0
        {
            self.use_shader(&self.default_assets.sprite_shader);
            self.use_instances(instances);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.render();
        }
    }

    pub fn draw_color(
        &mut self,
        instances: &InstanceBuffer<ColorInstance2D>,
        mesh: &PositionMesh2D,
        camera: &CameraBuffer2D,
    ) {
        if instances.buffer_size() != 0
            && mesh.vertex_buffer_size() != 0
            && mesh.index_buffer_size() != 0
        {
            self.use_shader(&self.default_assets.color_shader);
            self.use_instances(instances);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.render();
        }
    }

    pub fn draw_color_mesh(&mut self, mesh: &ColorMesh2D, camera: &CameraBuffer2D) {
        if mesh.vertex_buffer_size() != 0 && mesh.index_buffer_size() != 0 {
            self.use_shader(&self.default_assets.mesh_color_shader);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.render();
        }
    }

    pub fn draw_sprite_mesh(
        &mut self,
        mesh: &SpriteMesh2D,
        camera: &CameraBuffer2D,
        sprite: &Sprite,
    ) {
        if mesh.vertex_buffer_size() != 0 && mesh.index_buffer_size() != 0 {
            self.use_shader(&self.default_assets.mesh_sprite_shader);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.render();
        }
    }

    pub fn draw_sprite_array_mesh(
        &mut self,
        mesh: &SpriteArrayMesh2D,
        camera: &CameraBuffer2D,
        sprite: &Sprite,
    ) {
        if mesh.vertex_buffer_size() != 0 && mesh.index_buffer_size() != 0 {
            self.use_shader(&self.default_assets.mesh_sprite_shader);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.render();
        }
    }

    pub fn draw_sprite_crop(
        &mut self,
        instances: &InstanceBuffer<SpriteCropInstance2D>,
        mesh: &SpriteMesh2D,
        camera: &CameraBuffer2D,
        sprite: &Sprite,
    ) {
        if instances.buffer_size() != 0
            && mesh.vertex_buffer_size() != 0
            && mesh.index_buffer_size() != 0
        {
            self.use_shader(&self.default_assets.sprite_crop_shader);
            self.use_instances(instances);
            self.use_mesh(mesh);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.render();
        }
    }

    #[cfg(feature = "text")]
    pub fn draw_text_mesh(&mut self, text: &TextMesh, camera: &CameraBuffer2D, font: &Font) {
        if text.mesh().vertex_buffer_size() != 0 {
            self.use_shader(&self.default_assets.mesh_text_shader);
            self.use_camera(camera);
            self.use_mesh(text.mesh());
            self.use_sprite_array(font.sprite_array(), 1);
            self.render();
        }
    }

    pub fn draw_model<C: Camera>(
        &mut self,
        instances: &InstanceBuffer<Instance3D>,
        model: &Model,
        camera: &CameraBuffer<C>,
    ) {
        if instances.buffer_size() != 0 {
            self.use_shader(&self.default_assets.model_shader);
            self.use_instances(instances);
            self.use_camera(camera);
            for mesh in &model.meshes {
                if mesh.1.vertex_buffer_size() != 0 {
                    let sprite = if let Some(index) = mesh.0 {
                        &model.sprites[index]
                    } else {
                        &self.default_assets.missing_sprite
                    };
                    self.use_sprite(sprite, 1);
                    self.use_mesh(&mesh.1);
                    self.render();
                }
            }
        }
    }
}
