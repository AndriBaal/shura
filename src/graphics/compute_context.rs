use wgpu::RenderPass;

use crate::{Assets, BufferedColor, ComputeShader, Gpu, Sprite, Uniform};

pub struct ComputeContext<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub gpu: &'a Gpu,
    pub assets: &'a Assets,
    current_shader: Option<&'a ComputeShader>,
}

impl<'a> ComputeContext<'a> {
    pub(crate) fn new(mut render_pass: RenderPass<'a>, gpu: &'a Gpu, assets: &'a Assets) -> Self {
        render_pass.set_vertex_buffer(0, assets.compute_model.vertex_buffer().slice(..));
        render_pass.set_index_buffer(
            gpu.base.model_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        Self {
            render_pass,
            gpu,
            assets,
            current_shader: None,
        }
    }

    pub fn commit(&mut self) {
        self.render_pass.draw_indexed(0..6, 0, 0..1);
    }

    /// This uniform stores both the total time and the frame time.
    /// ```
    /// struct Times {
    ///     total_time: f32,
    ///     frame_time: f32
    /// }
    ///
    /// @group(1) @binding(0)
    /// var<uniform> total_time: Times;
    /// ```
    pub fn use_time_uniform_at(&mut self, slot: u32) {
        self.render_pass.set_bind_group(
            slot,
            self.assets().default_uniforms.times.bind_group(),
            &[],
        );
    }

    /// This uniform stores both the total time and the frame time and uses it at the default
    /// uniform position of the shader.
    /// ```
    /// struct Times {
    ///     total_time: f32,
    ///     frame_time: f32
    /// }
    ///
    /// @group(1) @binding(0)
    /// var<uniform> total_time: Times;
    /// ```
    pub fn use_time_uniform(&mut self) {
        let slot = self
            .current_shader()
            .index()
            .uniform
            .expect("The currently bound shader has no default uniform field.");
        self.use_time_uniform_at(slot);
    }

    pub fn use_shader(&mut self, shader: &'a ComputeShader) {
        self.current_shader = Some(shader);
        self.render_pass.set_pipeline(shader.pipeline());
    }

    pub fn use_sprite_at(&mut self, sprite: &'a Sprite, slot: u32) {
        self.render_pass
            .set_bind_group(slot, sprite.bind_group(), &[]);
    }

    pub fn use_color_at(&mut self, color: &'a BufferedColor, slot: u32) {
        self.render_pass
            .set_bind_group(slot, color.uniform().bind_group(), &[]);
    }

    pub fn use_uniform_at(&mut self, uniform: &'a Uniform, slot: u32) {
        self.render_pass
            .set_bind_group(slot, uniform.bind_group(), &[]);
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite) {
        let slot = self
            .current_shader()
            .index()
            .sprite
            .expect("The currently bound shader has no default sprite input field.");
        self.use_sprite_at(sprite, slot);
    }

    pub fn use_color(&mut self, color: &'a BufferedColor) {
        let slot = self
            .current_shader()
            .index()
            .color
            .expect("The currently bound shader has no default color field.");
        self.use_color_at(color, slot);
    }

    pub fn use_uniform(&mut self, uniform: &'a Uniform) {
        let slot = self
            .current_shader()
            .index()
            .uniform
            .expect("The currently bound shader has no default uniform field.");
        self.use_uniform_at(uniform, slot);
    }

    pub fn copy_sprite_to_target(&mut self, sprite: &'a Sprite) {
        self.use_shader(&self.assets.default_shaders.copy);
        self.use_sprite(sprite);
    }

    pub fn compute_grey(&mut self, source: &'a Sprite) {
        self.use_shader(&self.assets.default_shaders.grey);
        self.use_sprite(source);
    }


    pub fn compute_blurr(&mut self, source: &'a Sprite) {
        self.use_shader(&self.assets.default_shaders.blurr);
        self.use_sprite(source);
    }


    
    pub const fn assets(&self) -> &'a Assets {
        &self.assets
    }

    
    pub const fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    
    pub fn current_shader(&self) -> &'a ComputeShader {
        self.current_shader.as_ref().unwrap()
    }

    
    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.render_pass
    }
}
