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

/// Properties of a [Shader]
pub struct ShaderConfig<'a> {
    pub fragment_source: &'a str,
    pub name: &'a str,
    pub vertex_shader: Option<&'a str>,
    pub uniforms: &'a [UniformField],
    pub instance_fields: &'a [InstanceField<'a>],
    pub blend: BlendState,
    pub write_mask: ColorWrites,
}

impl Default for ShaderConfig<'static> {
    fn default() -> Self {
        Self {
            name: "",
            fragment_source: "",
            uniforms: &[],
            instance_fields: &[],
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
            vertex_shader: None,
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
    Uniform,
    SpriteSheet,
}

pub struct Shader {
    pipeline: wgpu::RenderPipeline,
    instance_size: u64,
}

impl Shader {
    pub const VERTEX: &'static str = include_str!("../../res/shader/vertex.wgsl");
    pub const SPRITE: &'static str = include_str!("../../res/shader/sprite.wgsl");
    pub const SPRITE_SHEET: &'static str = include_str!("../../res/shader/sprite_sheet.wgsl");
    pub const SPRITE_SHEET_UNIFORM: &'static str =
        include_str!("../../res/shader/sprite_sheet_uniform.wgsl");
    pub const COLOR: &'static str = include_str!("../../res/shader/color.wgsl");
    pub const COLOR_UNIFORM: &'static str = include_str!("../../res/shader/color_uniform.wgsl");
    pub const RAINBOW: &'static str = include_str!("../../res/shader/rainbow.wgsl");
    pub const GREY: &'static str = include_str!("../../res/shader/grey.wgsl");
    pub const BLURR: &'static str = include_str!("../../res/shader/blurr.wgsl");
    pub const VERTEX_INPUT_OFFSET: u32 = 4;
    pub const VETEX_OUTPUT_OFFSET: u32 = 1;

    pub fn new(gpu: &Gpu, config: ShaderConfig) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&gpu.base.camera_layout];
        for link in config.uniforms.iter() {
            match link {
                UniformField::Uniform => {
                    layouts.push(&gpu.base.uniform_layout);
                }
                UniformField::Sprite => {
                    layouts.push(&gpu.base.sprite_layout);
                }
                UniformField::SpriteSheet => {
                    layouts.push(&gpu.base.sprite_sheet_layout);
                }
            }
        }

        let fragment_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(config.name),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(config.fragment_source)),
            });

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(config.name),
                    bind_group_layouts: &layouts[..],
                    push_constant_ranges: &[],
                });

        let mut buffers = vec![Vertex::desc()];
        let mut attributes = InstancePosition::attributes();
        let mut array_stride = InstancePosition::size();
        let vertex_shader = if let Some(vertex_shader) = config.vertex_shader {
            for (index, field) in config.instance_fields.iter().enumerate() {
                let vertex_input = index as u32 + Self::VERTEX_INPUT_OFFSET;
                attributes.push(VertexAttribute {
                    format: field.format,
                    offset: array_stride,
                    shader_location: vertex_input,
                });
                array_stride += field.format.size();
            }
            Cow::Borrowed(vertex_shader)
        } else {
            let mut vertex_shader = Self::VERTEX.to_string();
            if !config.instance_fields.is_empty() {
                let mut instance_inputs: String = Default::default();
                let mut vertex_outputs: String = Default::default();
                let mut assignments: String = Default::default();
                for (index, field) in config.instance_fields.iter().enumerate() {
                    let vertex_input = index as u32 + Self::VERTEX_INPUT_OFFSET;
                    let vertex_output = index as u32 + Self::VETEX_OUTPUT_OFFSET;
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
                        "\n\t@location({vertex_output}) {}: {}",
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
            Cow::Owned(vertex_shader)
        };
        buffers.push(wgpu::VertexBufferLayout {
            array_stride,
            attributes: &attributes,
            step_mode: wgpu::VertexStepMode::Instance,
        });

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
            instance_size: array_stride,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn instance_size(&self) -> u64 {
        self.instance_size
    }
}
