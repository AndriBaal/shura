use crate::{
    CameraBuffer, Color, GpuDefaults, InstanceBuffer, InstanceIndices, Model, ModelIndexBuffer,
    RenderConfig, RenderConfigCamera, RenderConfigInstances, Shader, Sprite, Uniform, Gpu, Vector
};
#[cfg(feature = "text")]
use crate::text::TextDescriptor;
use std::ptr::null;

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
    pub render_pass: wgpu::RenderPass<'a>,
    pub indices: u32,
    pub msaa: bool,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    target_size: Vector<u32>,
    cache: RenderCache,
}

impl<'a> Renderer<'a> {
    pub fn new(
        render_encoder: &'a mut wgpu::CommandEncoder,
        defaults: &'a GpuDefaults,
        gpu: &'a Gpu,
        config: RenderConfig<'a>,
    ) -> Renderer<'a> {
        let target = config.target.target(defaults);
        let camera = config.camera.camera(defaults);

        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: if config.msaa {
                    target.msaa()
                } else {
                    target.view()
                },
                resolve_target: if config.msaa {
                    Some(target.view())
                } else {
                    None
                },
                ops: wgpu::Operations {
                    load: if let Some(clear_color) = config.clear_color {
                        wgpu::LoadOp::Clear(clear_color.into())
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        let mut renderer = Self {
            render_pass,
            indices: 0,
            msaa: config.msaa,
            cache: Default::default(),
            defaults: defaults,
            target_size: *target.size(),
            gpu
        };

        renderer.use_camera(camera);

        if let Some(instances) = config.intances {
            let instances = instances.instances(defaults);
            renderer.use_instances(instances);
        }

        return renderer;
    }

    pub(crate) fn output_renderer(
        encoder: &'a mut wgpu::CommandEncoder,
        output: &'a wgpu::TextureView,
        defaults: &'a GpuDefaults,
        gpu: &'a Gpu,
    ) -> Renderer<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        let mut renderer = Self {
            render_pass,
            indices: 0,
            msaa: false,
            cache: Default::default(),
            defaults,
            target_size: Vector::default(),
            gpu
        };
        renderer.use_uniform(defaults.relative_camera.0.uniform(), 0);
        renderer.use_instances(&defaults.single_centered_instance);
        return renderer;
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instances(&mut self, buffer: &'a InstanceBuffer) {
        let ptr = buffer as *const _;
        if ptr != self.cache.bound_instances {
            self.cache.bound_instances = ptr;
            self.render_pass.set_vertex_buffer(1, buffer.slice());
        }
    }

    pub fn use_camera(&mut self, camera: &'a CameraBuffer) {
        let ptr = camera as *const _;
        if ptr != self.cache.bound_camera {
            self.cache.bound_camera = ptr;
            self.render_pass
                .set_bind_group(0, camera.uniform().bind_group(), &[]);
        }
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
        let index_buffer = match model.index_buffer() {
            ModelIndexBuffer::Triangle => &self.defaults.triangle_index_buffer,
            ModelIndexBuffer::Cuboid => &self.defaults.cuboid_index_buffer,
            ModelIndexBuffer::Custom(c) => c,
        };
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
                .set_vertex_buffer(0, model.vertex_buffer().slice(..));
        }
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.use_bind_group(sprite.bind_group(), slot);
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
        self.render_pass
            .draw_indexed(0..self.indices, 0, instances.into().range);
    }

    pub const fn msaa(&self) -> bool {
        self.msaa
    }

    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.render_pass
    }

    
    #[cfg(feature = "text")]
    pub fn render_text(&mut self, descriptor: TextDescriptor<'a>) {
        let fov = unsafe { (&*self.cache.bound_camera).model().aabb(Default::default()).dim() };
        let target = self.target_size.cast::<f32>();
        let font = &mut descriptor.font.brush;
        let resolution = target.x / fov.x;
        let sections = descriptor.sections.into_iter().map(|s| s.to_glyph_section(resolution)).collect();


        self.cache = Default::default();
        font.resize_view(target.x, target.y, &self.gpu.queue);
        font.queue(&self.gpu.device, &self.gpu.queue, sections).unwrap();
        font.draw(&mut self.render_pass);
        
        // if let Some(color) = config.clear_color {
        //     self.clear(config.target, color);
        // }

        // let target = config.target.target(self.defaults);
        // let camera = config.camera.camera(self.defaults);
        // let fov = camera.model().aabb(Default::default()).dim();

        // let mut staging_belt = wgpu::util::StagingBelt::new(1024);

        // let resolution = target.size().x as f32 / fov.x;
        // for section in descriptor.sections {
        //     descriptor
        //         .font
        //         .brush
        //         .queue(section.to_glyph_section(resolution));
        // }

        // descriptor
        //     .font
        //     .brush
        //     .draw_queued(
        //         &self.gpu.device,
        //         &mut staging_belt,
        //         &mut self.inner,
        //         target.view(),
        //         target.size().x,
        //         target.size().y,
        //     )
        //     .expect("Draw queued");

        // staging_belt.finish();
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

    pub fn render_colored_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.colored_sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(color, 2);
        self.draw(instances);
    }

    pub fn render_transparent_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        transparency: &'a Uniform<f32>,
    ) {
        self.use_shader(&self.defaults.transparent);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(transparency, 2);
        self.draw(instances);
    }

    pub fn render_color(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.draw(instances);
    }

    pub fn render_color_no_msaa(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color_no_msaa);
        self.use_model(model);
        self.use_uniform(color, 1);
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
