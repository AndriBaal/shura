use crate::{Gpu, Vertex};
use std::borrow::Cow;

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
    /// Looks the following inside wgsl (Example for a [Uniform<Color>](crate::Uniform)):
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
}

impl Shader {
    pub fn new(
        gpu: &Gpu,
        fragment_source: &str,
        shader_lang: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&gpu.base.vertex_uniform];
        for link in shader_fields.iter() {
            match link {
                ShaderField::Uniform => {
                    layouts.push(&gpu.base.fragment_uniform);
                }
                ShaderField::Sprite => {
                    layouts.push(&gpu.base.sprite_uniform);
                }
            }
        }

        let vertex_shader = match shader_lang {
            ShaderLang::GLSL => &gpu.base.vertex_glsl,
            ShaderLang::WGSL => &gpu.base.vertex_wgsl,
        };
        let fragment_shader = match shader_lang {
            ShaderLang::GLSL => gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: Cow::Borrowed(fragment_source),
                        stage: naga::ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                }),
            ShaderLang::WGSL => gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(fragment_source)),
                }),
        };

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &layouts[..],
                    push_constant_ranges: &[],
                });

        let buffers = vec![Vertex::desc(), Vertex::instance_desc()];

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
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
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
                multisample: wgpu::MultisampleState {
                    count: gpu.base.sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        Shader {
            pipeline,
            lang: shader_lang,
        }
    }

    pub fn new_custom(
        gpu: &Gpu,
        lang: ShaderLang,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> Self {
        let pipeline = gpu.device.create_render_pipeline(descriptor);
        Shader { pipeline, lang }
    }

    // Getter
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn lang(&self) -> ShaderLang {
        self.lang
    }
}
