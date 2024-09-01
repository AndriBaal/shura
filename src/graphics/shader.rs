use crate::graphics::{Gpu, Instance, PositionInstance2D, SpriteVertex2D, Vertex};
pub use wgpu::{
    include_spirv, include_wgsl, vertex_attr_array, BlendComponent, BlendFactor, BlendOperation,
    BlendState, ColorWrites, Id as GpuId, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
};

#[cfg(feature = "log")]
use crate::log::info;

pub enum ShaderModuleSource<'a> {
    Single(&'a ShaderModule),
    Separate {
        vertex: &'a ShaderModule,
        fragment: &'a ShaderModule,
    },
    Dummy,
}

pub enum VertexBuffers<'a> {
    Vertex(&'a [wgpu::VertexFormat]),
    VertexInstance(&'a [wgpu::VertexFormat], &'a [wgpu::VertexFormat]),
    Custom(Vec<wgpu::VertexBufferLayout<'a>>),
}

impl<'a> VertexBuffers<'a> {
    pub fn vertex<V: Vertex>() -> Self {
        Self::Vertex(V::ATTRIBUTES)
    }

    pub fn instance<V: Vertex, I: Instance>() -> Self {
        Self::VertexInstance(V::ATTRIBUTES, I::ATTRIBUTES)
    }
}

pub struct ShaderConfig<'a> {
    pub name: Option<&'a str>,
    pub source: ShaderModuleSource<'a>,
    pub uniforms: &'a [UniformField<'a>],
    pub vertex_buffers: VertexBuffers<'a>,
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
            depth_stencil: None,
            fragment_entry: "fs_main",
            vertex_entry: "vs_main",
            vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, PositionInstance2D>(),
            source: ShaderModuleSource::Dummy,
        }
    }
}

pub enum UniformField<'a> {
    Sprite,
    SingleUniform,
    SpriteArray,
    Camera,
    Custom(&'a wgpu::BindGroupLayout),
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
        let default_layouts = gpu.default_layouts();
        for link in config.uniforms.iter() {
            let layout = match link {
                UniformField::SingleUniform => &*default_layouts.single_uniform_layout,
                UniformField::Sprite => &*default_layouts.sprite_layout,
                UniformField::SpriteArray => &*default_layouts.sprite_array_layout,
                UniformField::Camera => &*default_layouts.camera_layout,
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

        let va;
        let ia;
        let buffers = match config.vertex_buffers {
            VertexBuffers::VertexInstance(vertex_attributes, instance_attributes) => {
                let mut shader_index_counter = 0;
                let mut vertex_size = 0;
                let mut instance_size = 0;
                va = vertex_attributes
                    .iter()
                    .map(|format| {
                        let attr = wgpu::VertexAttribute {
                            format: *format,
                            offset: vertex_size,
                            shader_location: shader_index_counter,
                        };
                        vertex_size += format.size();
                        shader_index_counter += 1;
                        attr
                    })
                    .collect::<Vec<_>>();
                ia = instance_attributes
                    .iter()
                    .map(|format| {
                        let attr = wgpu::VertexAttribute {
                            format: *format,
                            offset: instance_size,
                            shader_location: shader_index_counter,
                        };
                        instance_size += format.size();
                        shader_index_counter += 1;
                        attr
                    })
                    .collect::<Vec<_>>();

                vec![
                    wgpu::VertexBufferLayout {
                        array_stride: vertex_size,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &va,
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: instance_size,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &ia,
                    },
                ]
            }
            VertexBuffers::Vertex(vertex_attributes) => {
                let mut shader_index_counter = 0;
                let mut vertex_size = 0;
                va = vertex_attributes
                    .iter()
                    .map(|format| {
                        let attr = wgpu::VertexAttribute {
                            format: *format,
                            offset: vertex_size,
                            shader_location: shader_index_counter,
                        };
                        vertex_size += format.size();
                        shader_index_counter += 1;
                        attr
                    })
                    .collect::<Vec<_>>();

                vec![wgpu::VertexBufferLayout {
                    array_stride: vertex_size,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &va,
                }]
            }
            VertexBuffers::Custom(custom) => custom,
        };

        // let cache = unsafe { gpu.device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor { label: None, data: None, fallback: true }) };

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
                    buffers: &buffers,
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
                cache: None, // cache: Some(&cache)
            });

        #[cfg(feature = "log")]
        if let Some(name) = config.name {
            info!("Successfully compiled shader {name}");
        }

        Shader {
            pipeline,
            instance_size: Self::size_of_step_mode(&buffers, wgpu::VertexStepMode::Instance),
            vertex_size: Self::size_of_step_mode(&buffers, wgpu::VertexStepMode::Vertex),
        }
    }

    pub fn size_of_step_mode(
        buffers: &[wgpu::VertexBufferLayout],
        step_mode: wgpu::VertexStepMode,
    ) -> u64 {
        buffers
            .iter()
            .filter(|s| s.step_mode == step_mode)
            .map(|b| b.array_stride)
            .max()
            .unwrap_or(0)
    }

    pub fn custom(gpu: &Gpu, descriptor: &wgpu::RenderPipelineDescriptor) -> Self {
        let pipeline = gpu.device.create_render_pipeline(descriptor);
        Self {
            pipeline,
            instance_size: Self::size_of_step_mode(
                descriptor.vertex.buffers,
                wgpu::VertexStepMode::Instance,
            ),
            vertex_size: Self::size_of_step_mode(
                descriptor.vertex.buffers,
                wgpu::VertexStepMode::Vertex,
            ),
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
