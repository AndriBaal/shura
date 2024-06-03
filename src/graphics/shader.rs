use std::marker::PhantomData;

use crate::graphics::{Gpu, Instance, Vertex};
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
    Dummy
}

pub struct ShaderConfig<'a, V: Vertex, I: Instance> {
    pub name: Option<&'a str>,
    pub source: ShaderModuleSource<'a>,
    pub uniforms: &'a [UniformField],
    pub blend: BlendState,
    pub write_mask: ColorWrites,
    pub vertex_entry: &'static str,
    pub fragment_entry: &'static str,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub marker: PhantomData<(V, I)>
}

impl <V: Vertex, I: Instance>Default for ShaderConfig<'static, V, I> {
    fn default() -> Self {
        Self {
            name: None,
            uniforms: &[UniformField::Camera],
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
            source: ShaderModuleSource::Dummy,
            depth_stencil: None,
            fragment_entry: "fs_main",
            vertex_entry: "vs_main",
            marker: PhantomData,
        }
    }
}

pub enum UniformField {
    Sprite,
    SingleUniform,
    SpriteArray,
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
    pub fn new<V: Vertex, I: Instance>(gpu: &Gpu, config: ShaderConfig<V, I>) -> Self {
        let mut layouts: Vec<&wgpu::BindGroupLayout> = Vec::with_capacity(config.uniforms.len());
        let shared_assets = gpu.shared_assets();
        for link in config.uniforms.iter() {
            let layout = match link {
                UniformField::SingleUniform => &shared_assets.single_uniform_layout,
                UniformField::Sprite => &shared_assets.sprite_layout,
                UniformField::SpriteArray => &shared_assets.sprite_array_layout,
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

        let mut shader_index_counter = 0;
        let vertex_attributes = V::ATTRIBUTES
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
        let instance_attributes = I::ATTRIBUTES
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

        let mut buffers = vec![VertexBufferLayout {
            array_stride: V::SIZE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attributes,
        }];

        if !instance_attributes.is_empty() {
            buffers.push(VertexBufferLayout {
                array_stride: I::SIZE,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attributes,
            });
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

    pub fn custom(gpu: &Gpu, descriptor: &wgpu::RenderPipelineDescriptor) -> Self {
        let pipeline = gpu.device.create_render_pipeline(descriptor);
        let vertex_size = descriptor
            .vertex
            .buffers
            .iter()
            .filter(|s| s.step_mode == wgpu::VertexStepMode::Vertex)
            .fold(0, |sum, s| sum + s.array_stride);
        let instance_size = descriptor
            .vertex
            .buffers
            .iter()
            .filter(|s| s.step_mode == wgpu::VertexStepMode::Instance)
            .fold(0, |sum, s| sum + s.array_stride);
        Self {
            pipeline,
            instance_size,
            vertex_size,
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
