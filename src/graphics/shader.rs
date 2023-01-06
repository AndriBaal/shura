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
}

impl Shader {
    pub fn new(
        gpu: &Gpu,
        fragment_source: &str,
        shader_lang: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Self {
        Self::new_wgpu(
            &gpu.device,
            gpu.config.format,
            gpu.defaults.sample_count,
            (&gpu.defaults.vertex_glsl, &gpu.defaults.vertex_wgsl),
            (
                &gpu.defaults.vertex_uniform,
                &gpu.defaults.fragment_uniform,
                &gpu.defaults.sprite_uniform,
            ),
            fragment_source,
            shader_lang,
            shader_fields,
        )
    }

    pub(crate) fn new_wgpu(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        vertex_shaders: (&wgpu::ShaderModule, &wgpu::ShaderModule),
        layouts: (
            &wgpu::BindGroupLayout,
            &wgpu::BindGroupLayout,
            &wgpu::BindGroupLayout,
        ),
        fragment_source: &str,
        shader_lang: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Self {
        let layouts = create_shader_index(layouts, shader_fields);
        let pipeline = create_pipeline(
            device,
            format,
            sample_count,
            vertex_shaders,
            fragment_source,
            shader_lang,
            &layouts[..],
        );

        Shader { pipeline }
    }

    pub fn new_custom(gpu: &Gpu, descriptor: &wgpu::RenderPipelineDescriptor) -> Self {
        let pipeline = gpu.device.create_render_pipeline(descriptor);
        Shader { pipeline }
    }

    // Getter
    pub(crate) fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

fn create_shader_index<'a>(
    layouts: (
        &'a wgpu::BindGroupLayout,
        &'a wgpu::BindGroupLayout,
        &'a wgpu::BindGroupLayout,
    ),
    shader_fields: &[ShaderField],
) -> Vec<&'a wgpu::BindGroupLayout> {
    let mut out_layouts: Vec<&wgpu::BindGroupLayout> = vec![layouts.0];
    for link in shader_fields.iter() {
        match link {
            ShaderField::Uniform => {
                out_layouts.push(layouts.1);
            }
            ShaderField::Sprite => {
                out_layouts.push(layouts.2);
            }
        }
    }

    return out_layouts;
}

fn create_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sample_count: u32,
    vertex_shaders: (&wgpu::ShaderModule, &wgpu::ShaderModule),
    fragment_shader_source: &str,
    shader_lang: ShaderLang,
    layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let vertex_shader = match shader_lang {
        ShaderLang::GLSL => vertex_shaders.0,
        ShaderLang::WGSL => vertex_shaders.1,
    };

    let fragment_shader = match shader_lang {
        ShaderLang::GLSL => device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Borrowed(fragment_shader_source),
                stage: naga::ShaderStage::Fragment,
                defines: Default::default(),
            },
        }),
        ShaderLang::WGSL => device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(fragment_shader_source)),
        }),
    };

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &layouts[..],
        push_constant_ranges: &[],
    });

    let buffers = vec![Vertex::desc(), Vertex::instance_desc()];

    // Default Shader Configuration
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                format,
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
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    return pipeline;
}
