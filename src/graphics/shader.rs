use crate::{Gpu, InstanceData, Vertex};
use std::borrow::Cow;

pub use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites};

/// Properties of a [Shader]
pub struct ShaderConfig<'a> {
    pub fragment_source: &'a str,
    pub shader_lang: ShaderLang,
    pub shader_fields: &'a [ShaderField],
    pub msaa: bool,
    pub blend: BlendState,
    pub write_mask: ColorWrites,
}

/// Field that is present in the shader.
pub enum ShaderField {
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
}

#[derive(Copy, Clone, Eq, PartialEq)]
/// Supported shader languages.
pub enum ShaderLang {
    GLSL,
    WGSL,
}

/// Shader following the shura shader system. The vertex shader is the same along every shader and is provided
/// by shura.
///
/// # Example:
///
/// This example shows the [render_grey](crate::Renderer::render_grey) method from the [Renderer](crate::Renderer).
///
/// The shader (wgsl code in a sperate file):
/// ```
//// @group(1) @binding(0) // Represents a shura sprite
/// var t_diffuse: texture_2d<f32>;
/// @group(1) @binding(1)
/// var s_diffuse: sampler;
///
/// struct VertexOutput { // Output from the vertex shader
///     @builtin(position) clip_position: vec4<f32>,
///     @location(0) tex_coords: vec2<f32>,
/// }
///
/// @fragment
/// fn main(in: VertexOutput) -> @location(0) vec4<f32> {
///     let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
///     let gray = dot(vec3<f32>(0.299, 0.587, 0.114), color.rgb);
///
///     return vec4<f32>(gray, gray, gray, color.a);
/// }
/// ```
///
/// Loading the shader in shura:
/// ```
/// let grey = Shader::new(
///     gpu,
///     include_str!("grey.wgsl"),
///     ShaderLang::WGSL,
///     &[ShaderField::Sprite],
/// );
/// ```
///
/// The shader can then be used in shura with [use_shader](crate::Renderer::use_shader).
///
pub struct Shader {
    pipeline: wgpu::RenderPipeline,
    lang: ShaderLang,
    msaa: bool,
}

impl Shader {
    pub const VERTEX_GLSL: &'static str = include_str!("../../res/shader/vertex.glsl");
    pub const VERTEX_WGSL: &'static str = include_str!("../../res/shader/vertex.wgsl");
    pub const SPIRTE_WGSL: &'static str = include_str!("../../res/shader/sprite.wgsl");
    pub const RAINBOW_WGSL: &'static str = include_str!("../../res/shader/rainbow.wgsl");
    pub const GREY_WGSL: &'static str = include_str!("../../res/shader/grey.wgsl");
    pub const BLURR_WGSL: &'static str = include_str!("../../res/shader/blurr.wgsl");
    pub fn new(gpu: &Gpu, config: ShaderConfig) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&gpu.base.vertex_layout];
        for link in config.shader_fields.iter() {
            match link {
                ShaderField::Uniform => {
                    layouts.push(&gpu.base.fragment_layout);
                }
                ShaderField::Sprite => {
                    layouts.push(&gpu.base.sprite_layout);
                }
            }
        }

        let vertex_shader = match config.shader_lang {
            ShaderLang::GLSL => &gpu.base.vertex_glsl,
            ShaderLang::WGSL => &gpu.base.vertex_wgsl,
        };
        let fragment_shader = match config.shader_lang {
            ShaderLang::GLSL => gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: Cow::Borrowed(config.fragment_source),
                        stage: naga::ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                }),
            ShaderLang::WGSL => gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(config.fragment_source)),
                }),
        };

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &layouts[..],
                    push_constant_ranges: &[],
                });

        let buffers = vec![Vertex::desc(), InstanceData::desc()];

        // Default Shader Configuration
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: "main",
                    buffers: &buffers[..],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: "main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.config.format,
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
                multisample: if config.msaa {
                    gpu.base.multisample
                } else {
                    gpu.base.no_multisample
                },
                multiview: None,
            });

        Shader {
            pipeline,
            lang: config.shader_lang,
            msaa: config.msaa,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn lang(&self) -> ShaderLang {
        self.lang
    }

    pub fn msaa(&self) -> bool {
        self.msaa
    }
}
