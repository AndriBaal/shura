use crate::graphics::{Gpu, Instance, Instance2D, Vertex, Vertex2D};
pub use wgpu::{
    include_spirv, include_wgsl, vertex_attr_array, BlendComponent, BlendFactor, BlendOperation,
    BlendState, ColorWrites, Id as GpuId, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
};

#[cfg(feature = "log")]
use log::info;

pub enum ShaderModuleSource<'a> {
    Single(&'a ShaderModule),
    Separate {
        vertex: &'a ShaderModule,
        fragment: &'a ShaderModule,
    },
    #[doc(hidden)]
    Dummy,
}

pub struct ShaderConfig<'a> {
    pub name: Option<&'a str>,
    pub source: ShaderModuleSource<'a>,
    pub buffers: &'a [VertexBufferLayout<'a>],
    pub uniforms: &'a [UniformField],
    pub blend: BlendState,
    pub write_mask: ColorWrites,
    pub vertex_entry: &'static str,
    pub fragment_entry: &'static str,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl Default for ShaderConfig<'static> {
    fn default() -> Self {
        Self {
            name: None,
            uniforms: &[UniformField::Camera],
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
            buffers: &[Vertex2D::LAYOUT, Instance2D::LAYOUT],
            source: ShaderModuleSource::Dummy,
            depth_stencil: None,
            fragment_entry: "fs_main",
            vertex_entry: "vs_main",
        }
    }
}

pub enum UniformField {
    Sprite,
    SingleUniform,
    SpriteSheet,
    Camera,
    Custom(wgpu::BindGroupLayout),
}

#[derive(Debug)]
pub struct Shader {
    pipeline: wgpu::RenderPipeline,
    instance_size: wgpu::BufferAddress,
    vertex_size: wgpu::BufferAddress,
}

impl Shader {
    pub fn new(gpu: &Gpu, config: ShaderConfig) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = Vec::with_capacity(config.uniforms.len());
        let shared_assets = gpu.shared_assets();
        for link in config.uniforms.iter() {
            let layout = match link {
                UniformField::SingleUniform => &shared_assets.single_uniform_layout,
                UniformField::Sprite => &shared_assets.sprite_layout,
                UniformField::SpriteSheet => &shared_assets.sprite_sheet_layout,
                UniformField::Camera => &shared_assets.camera_layout,
                UniformField::Custom(c) => c,
            };
            layouts.push(layout);
        }

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: config.name,
                    bind_group_layouts: &layouts[..],
                    push_constant_ranges: &[],
                });

        let mut vertex_size = 0;
        let mut instance_size = 0;

        for buffer in config.buffers {
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
                label: config.name,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: match config.source {
                        ShaderModuleSource::Single(s) => s,
                        ShaderModuleSource::Separate { vertex, .. } => vertex,
                        ShaderModuleSource::Dummy => panic!("Dummy not allowed!"),
                    },
                    entry_point: config.vertex_entry,
                    buffers: config.buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: match config.source {
                        ShaderModuleSource::Single(s) => s,
                        ShaderModuleSource::Separate { fragment, .. } => fragment,
                        ShaderModuleSource::Dummy => panic!("Dummy not allowed!"),
                    },
                    entry_point: config.fragment_entry,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.format(),
                        blend: Some(config.blend),
                        write_mask: config.write_mask,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
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
                depth_stencil: config.depth_stencil,
                multisample: gpu.sample_state(),
                multiview: None,
            });

        #[cfg(feature = "log")]
        if let Some(name) = config.name {
            info!("Successfully compiled shader {name}");
        }

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
