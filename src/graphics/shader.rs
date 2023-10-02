use crate::{Gpu, InstancePosition, Vertex};
use std::borrow::Cow;
use wgpu::VertexAttribute;
pub use wgpu::{
    vertex_attr_array, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    VertexFormat,
};

#[cfg(feature = "log")]
use log::info;

pub struct InstanceField<'a> {
    pub format: VertexFormat,
    pub field_name: &'a str,
    pub data_type: &'a str,
}

pub enum VertexShader<'a> {
    Instance,
    Custom(&'a str, Vec<wgpu::VertexBufferLayout<'a>>),
    AutoInstance(&'a [InstanceField<'a>]),
    // ModelOnly,
}

/// Properties of a [Shader]
pub struct ShaderConfig<'a> {
    pub name: &'a str,
    pub fragment_shader: &'a str,
    pub vertex_shader: VertexShader<'a>,
    pub uniforms: &'a [UniformField],
    pub blend: BlendState,
    pub write_mask: ColorWrites,
    pub instancing: bool,
}

impl Default for ShaderConfig<'static> {
    fn default() -> Self {
        Self {
            name: "",
            fragment_shader: "",
            uniforms: &[],
            vertex_shader: VertexShader::Instance,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
            instancing: true,
        }
    }
}

/// Field that is present in the shader.
pub enum UniformField {
    /// Looks the following inside wgsl:
    /// ```
    //// @group(1) @binding(0)
    /// var t_diffuse: texture_2d<f32>;
    /// @group(1) @binding(1)
    /// var s_diffuse: sampler;
    /// ```
    Sprite,
    /// Looks the following inside wgsl (Example for a [`Uniform<Color>`](crate::Uniform)):
    /// ```
    /// struct Color {
    ///     color: vec4<f32>
    /// }
    ///
    /// @group(1) @binding(0)
    /// var<uniform> color: Color;
    /// ```
    SingleUniform,
    SpriteSheet,
    Custom(wgpu::BindGroupLayout),
}

pub struct Shader {
    pipeline: wgpu::RenderPipeline,
    instance_size: wgpu::BufferAddress,
    vertex_size: wgpu::BufferAddress,
}

impl Shader {
    pub const VERTEX: &'static str = include_str!("../../res/shader/vertex.wgsl");
    pub const VERTEX_CROP: &'static str = include_str!("../../res/shader/vertex_crop.wgsl");
    pub const VERTEX_CROP_SHEET: &'static str =
        include_str!("../../res/shader/vertex_crop_sheet.wgsl");
    pub const SPRITE: &'static str = include_str!("../../res/shader/sprite.wgsl");
    pub const SPRITE_SHEET: &'static str = include_str!("../../res/shader/sprite_sheet.wgsl");
    pub const SPRITE_SHEET_UNIFORM: &'static str =
        include_str!("../../res/shader/sprite_sheet_uniform.wgsl");
    pub const TRANSPARENT: &'static str = include_str!("../../res/shader/transparent.wgsl");
    pub const TRANSPARENT_UNIFORM: &'static str =
        include_str!("../../res/shader/transparent_uniform.wgsl");
    pub const COLOR: &'static str = include_str!("../../res/shader/color.wgsl");
    pub const COLOR_UNIFORM: &'static str = include_str!("../../res/shader/color_uniform.wgsl");
    pub const RAINBOW: &'static str = include_str!("../../res/shader/rainbow.wgsl");
    pub const GREY: &'static str = include_str!("../../res/shader/grey.wgsl");
    pub const BLURR: &'static str = include_str!("../../res/shader/blurr.wgsl");
    pub const TEXT: &'static str = include_str!("../../res/shader/text.wgsl");
    pub const AUTO_INSTANCE_INPUT_OFFSET: u32 = 4;
    pub const AUTO_INSTANCE_OUTPUT_OFFSET: u32 = 1;

    pub fn new(gpu: &Gpu, config: ShaderConfig) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&gpu.base.camera_layout];
        for link in config.uniforms.iter() {
            let layout = match link {
                UniformField::SingleUniform => &gpu.base.single_uniform_layout,
                UniformField::Sprite => &gpu.base.sprite_layout,
                UniformField::SpriteSheet => &gpu.base.sprite_sheet_layout,
                UniformField::Custom(c) => c,
            };
            layouts.push(layout);
        }

        let fragment_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(config.name),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(config.fragment_shader)),
            });

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(config.name),
                    bind_group_layouts: &layouts[..],
                    push_constant_ranges: &[],
                });

        let mut attributes = InstancePosition::ATTRIBUTES.to_vec();
        let (vertex_shader, buffers) = match config.vertex_shader {
            VertexShader::Instance => {
                let buffers = vec![Vertex::DESC, InstancePosition::DESC];
                let shader = Cow::Borrowed(Self::VERTEX);
                (shader, buffers)
            }
            VertexShader::Custom(src, layouts) => {
                let shader = Cow::Borrowed(src);
                (shader, layouts)
            }
            VertexShader::AutoInstance(instance_attributes) => {
                let mut buffers = vec![Vertex::DESC];
                let mut array_stride = InstancePosition::SIZE;
                let mut vertex_shader = Self::VERTEX.to_string();

                if !instance_attributes.is_empty() {
                    let mut instance_inputs: String = Default::default();
                    let mut vertex_outputs: String = Default::default();
                    let mut assignments: String = Default::default();
                    for (index, field) in instance_attributes.iter().enumerate() {
                        let vertex_input = index as u32 + Self::AUTO_INSTANCE_INPUT_OFFSET;
                        let vertex_output = index as u32 + Self::AUTO_INSTANCE_OUTPUT_OFFSET;
                        attributes.push(VertexAttribute {
                            format: field.format,
                            offset: array_stride,
                            shader_location: vertex_input,
                        });
                        array_stride += field.format.size();
                        instance_inputs += &format!(
                            "\n\t@location({vertex_input}) {}: {},",
                            field.field_name, field.data_type
                        );
                        vertex_outputs += &format!(
                            "\n\t@location({vertex_output}) {}: {},",
                            field.field_name, field.data_type
                        );
                        assignments += &format!("\n\tout.{0} = instance.{0};", field.field_name);
                    }
                    vertex_shader =
                        vertex_shader.replace("// SHURA_MARKER_INSTANCE_INPUT", &instance_inputs);
                    vertex_shader =
                        vertex_shader.replace("// SHURA_MARKER_VERTEX_OUTPUT", &vertex_outputs);
                    vertex_shader =
                        vertex_shader.replace("// SHURA_MARKER_VARIABLE_ASSIGNMENT", &assignments);
                }
                buffers.push(wgpu::VertexBufferLayout {
                    array_stride,
                    attributes: &attributes,
                    step_mode: wgpu::VertexStepMode::Instance,
                });
                let shader = Cow::Owned(vertex_shader);
                (shader, buffers)
            }
        };

        let mut vertex_size = 0;
        let mut instance_size = 0;

        for buffer in &buffers {
            match &buffer.step_mode {
                wgpu::VertexStepMode::Vertex => {
                    vertex_size += buffer.array_stride;
                }
                wgpu::VertexStepMode::Instance => {
                    instance_size += buffer.array_stride;
                }
            }
        }

        // Default Shader Configuration
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(config.name),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &gpu
                        .device
                        .create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some(config.name),
                            source: wgpu::ShaderSource::Wgsl(vertex_shader),
                        }),
                    entry_point: "main",
                    buffers: &buffers[..],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: "main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.format,
                        blend: Some(config.blend),
                        write_mask: config.write_mask,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: gpu.base.multisample,
                multiview: None,
            });

        #[cfg(feature = "log")]
        info!("Successfully compiled shader {}", config.name);

        Shader {
            pipeline,
            vertex_size,
            instance_size,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn instance_size(&self) -> wgpu::BufferAddress {
        self.instance_size
    }

    pub fn vertex_size(&self) -> wgpu::BufferAddress {
        self.vertex_size
    }
}
