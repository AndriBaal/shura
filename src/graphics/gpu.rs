use wgpu::include_wgsl;
use winit::window::Window;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "text")]
use crate::text::{Font, FontBuilder, Text, TextSection};
use crate::{
    graphics::{
        Camera, Camera2D, CameraBuffer, CameraBuffer2D, DepthBuffer, Instance, Instance2D,
        Instance3D, InstanceBuffer, InstanceBuffer2D, Mesh, Mesh2D, MeshBuilder, MeshBuilder2D,
        Model, ModelBuilder, RenderEncoder, RenderTarget, Shader, ShaderConfig, ShaderModule,
        ShaderModuleDescriptor, ShaderModuleSoure, Sprite, SpriteBuilder, SpriteRenderTarget,
        SpriteSheet, SpriteSheetBuilder, Surface, Uniform, UniformField, Vertex, Vertex3D,
        WorldCamera3D,
    },
    math::{Isometry2, Vector2},
};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, OnceLock, RwLock},
};

pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 0.5;

pub static GLOBAL_GPU: OnceLock<Arc<Gpu>> = OnceLock::new();

#[derive(Clone)]
pub struct GpuConfig {
    pub backends: wgpu::Backends,
    pub device_features: wgpu::Features,
    pub device_limits: wgpu::Limits,
    pub max_samples: u8,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            backends: wgpu::Backends::all(),
            device_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            device_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            },
            max_samples: 4,
        }
    }
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub command_buffers: Mutex<Vec<wgpu::CommandBuffer>>,
    format: OnceLock<wgpu::TextureFormat>,
    shared_resources: OnceLock<SharedResources>,
    default_resources: OnceLock<RwLock<DefaultResources>>,

    samples: OnceLock<u32>,
    max_samples: u32,
    sample_state: OnceLock<wgpu::MultisampleState>,
}

impl Gpu {
    pub(crate) async fn new(surface: &mut Surface, window: Arc<Window>, config: GpuConfig) -> Self {
        let max_samples = config.max_samples as u32;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: config.backends,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            ..Default::default()
        });
        surface.pre_adapter(&instance, window);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.surface(),
                force_fallback_adapter: false,
            })
            .await
            .expect("Invalid Graphics Backend!");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: config.device_features,
                    required_limits: config.device_limits.using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

        #[cfg(feature = "log")]
        {
            let adapter_info = adapter.get_info();
            info!("Using GPU: {}", adapter_info.name);
            info!("Using WGPU backend: {:?}", adapter_info.backend);
        }

        Self {
            instance,
            queue,
            device,
            adapter,
            max_samples,
            command_buffers: Mutex::new(Default::default()),
            format: OnceLock::new(),
            samples: OnceLock::new(),
            sample_state: OnceLock::new(),
            shared_resources: OnceLock::new(),
            default_resources: OnceLock::new(),
        }
    }

    pub fn block(&self, handle: wgpu::SubmissionIndex) {
        self.device
            .poll(wgpu::MaintainBase::WaitForSubmissionIndex(handle));
    }

    pub fn submit(&self) -> wgpu::SubmissionIndex {
        let mut command_buffers = self.command_buffers.lock().unwrap();
        let command_buffers = std::mem::take(&mut *command_buffers);
        self.queue.submit(command_buffers)
    }

    pub fn create_render_target(&self, size: Vector2<u32>) -> SpriteRenderTarget {
        SpriteRenderTarget::new(self, size)
    }

    pub fn create_custom_render_target<D: Deref<Target = [u8]>>(
        &self,
        sprite: SpriteBuilder<D>,
    ) -> SpriteRenderTarget {
        SpriteRenderTarget::custom(self, sprite)
    }

    pub fn create_instance_buffer<I: Instance>(&self, instances: &[I]) -> InstanceBuffer<I> {
        InstanceBuffer::new(self, instances)
    }

    pub fn create_camera_buffer<C: Camera>(&self, camera: &C) -> CameraBuffer<C> {
        CameraBuffer::new(self, camera)
    }

    pub fn create_mesh<V: Vertex>(&self, builder: &dyn MeshBuilder<Vertex = V>) -> Mesh<V> {
        Mesh::new(self, builder)
    }

    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(self, builder)
    }

    pub fn create_sprite<D: Deref<Target = [u8]>>(&self, desc: SpriteBuilder<D>) -> Sprite {
        Sprite::new(self, desc)
    }

    pub fn create_sprite_sheet<D: Deref<Target = [u8]>>(
        &self,
        desc: SpriteSheetBuilder<D>,
    ) -> SpriteSheet {
        SpriteSheet::new(self, desc)
    }

    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(self, data)
    }

    pub fn create_shader(&self, config: ShaderConfig) -> Shader {
        Shader::new(self, config)
    }

    pub fn create_shader_module(&self, desc: ShaderModuleDescriptor<'_>) -> ShaderModule {
        self.device.create_shader_module(desc)
    }

    #[cfg(feature = "text")]
    pub fn create_font(&self, builder: FontBuilder) -> Font {
        Font::new(self, builder)
    }

    #[cfg(feature = "text")]
    pub fn create_text<S: AsRef<str>>(&self, font: &Font, sections: &[TextSection<S>]) -> Text {
        Text::new(self, font, sections)
    }

    pub fn create_computed_target<D: Deref<Target = [u8]>>(
        &self,
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> SpriteRenderTarget {
        SpriteRenderTarget::computed(self, sprite, compute)
    }

    pub fn samples(&self) -> u32 {
        return *self.samples.get().unwrap();
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        return *self.format.get().unwrap();
    }

    pub fn sample_state(&self) -> wgpu::MultisampleState {
        return *self.sample_state.get().unwrap();
    }

    pub fn shared_resources(&self) -> &SharedResources {
        return self.shared_resources.get().unwrap();
    }

    pub fn default_resources(&self) -> impl Deref<Target = DefaultResources> + '_ {
        return self.default_resources.get().unwrap().read().unwrap();
    }

    pub fn default_resources_mut(&self) -> impl DerefMut<Target = DefaultResources> + '_ {
        return self.default_resources.get().unwrap().write().unwrap();
    }

    pub fn is_initialized(&self) -> bool {
        return self.format.get().is_some();
    }

    pub(crate) fn initialize(&self, surface: &Surface) {
        let config = surface.config();
        let format = config.format;
        let max_samples = self.max_samples;
        let sample_flags = self
            .adapter
            .get_texture_format_features(config.format)
            .flags;
        let samples: u32 = {
            if max_samples >= 16
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X16)
            {
                16
            } else if max_samples >= 8
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8)
            {
                8
            } else if max_samples >= 4
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            {
                4
            } else if max_samples >= 2
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2)
            {
                2
            } else {
                1
            }
        };
        let sample_state = wgpu::MultisampleState {
            count: samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        self.samples.set(samples).unwrap();
        self.format.set(format).unwrap();
        self.sample_state.set(sample_state).unwrap();
        self.shared_resources
            .set(SharedResources::new(self))
            .unwrap();
        self.default_resources
            .set(RwLock::new(DefaultResources::new(self, surface)))
            .unwrap();
        #[cfg(feature = "log")]
        {
            info!("Using multisample X{samples}");
            info!("Using texture format: {:?}", config.format);
            info!("Using Present mode: {:?}", config.present_mode);
        }
    }
}

#[derive(Debug)]
pub struct SharedResources {
    pub vertex_shader_module: ShaderModule,
    pub sprite_sheet_layout: wgpu::BindGroupLayout,
    pub sprite_layout: wgpu::BindGroupLayout,
    pub camera_layout: wgpu::BindGroupLayout,
    pub single_uniform_layout: wgpu::BindGroupLayout,
}

impl SharedResources {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let sprite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sprite_bind_group_layout"),
        });

        let single_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let sprite_sheet_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_sheet_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let vertex_shader_module =
            device.create_shader_module(include_wgsl!("../../static/shader/2d/vertex.wgsl"));

        Self {
            vertex_shader_module,
            sprite_sheet_layout,
            sprite_layout,
            camera_layout,
            single_uniform_layout,
        }
    }
}

#[derive(Debug)]
pub struct DefaultResources {
    // 2D
    pub sprite: Shader,
    pub sprite_sheet: Shader,
    pub color: Shader,
    pub rainbow: Shader,
    pub grey: Shader,
    #[cfg(feature = "text")]
    pub text: Shader,
    pub blurr: Shader,

    pub missing: Sprite,

    // 3D
    pub model: Shader,
    pub depth_buffer: DepthBuffer,
    pub unit_mesh: Mesh2D,

    pub times: Uniform<[f32; 2]>,
    pub world_camera2d: CameraBuffer2D,
    pub world_camera3d: CameraBuffer<WorldCamera3D>,
    pub relative_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_right_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_right_camera: (CameraBuffer2D, Camera2D),
    pub unit_camera: (CameraBuffer2D, Camera2D),
    pub centered_instance: InstanceBuffer2D,

    #[cfg(feature = "framebuffer")]
    pub framebuffer: SpriteRenderTarget,
}

impl DefaultResources {
    pub(crate) fn new(gpu: &Gpu, surface: &Surface) -> Self {
        let shared_resources = gpu.shared_resources();
        let sprite_sheet = gpu.create_shader(ShaderConfig {
            name: Some("sprite_sheet"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!(
                    "../../static/shader/2d/sprite_sheet.wgsl"
                )),
            },
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            ..Default::default()
        });

        #[cfg(feature = "text")]
        let text = gpu.create_shader(ShaderConfig {
            name: Some("text"),
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            source: ShaderModuleSoure::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/text.wgsl")),
            ),
            buffers: &[
                crate::text::Vertex2DText::DESC,
                // Not Instance2D::DESC because of offset
                wgpu::VertexBufferLayout {
                    array_stride: Instance2D::SIZE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        4 => Float32x2,
                        5 => Float32x4,
                        6 => Float32x2,
                        7 => Float32x2,
                        8 => Float32x4,
                        9 => Uint32,
                    ],
                },
            ],
            ..Default::default()
        });

        let model = gpu.create_shader(ShaderConfig {
            name: Some("model"),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            source: ShaderModuleSoure::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/3d/model.wgsl")),
            ),
            buffers: &[Vertex3D::DESC, Instance3D::DESC],
            depth_stencil: Some(DepthBuffer::depth_state()),
            ..Default::default()
        });

        let color = gpu.create_shader(ShaderConfig {
            name: Some("color"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/color.wgsl")),
            },
            uniforms: &[UniformField::Camera],
            ..Default::default()
        });

        let sprite = gpu.create_shader(ShaderConfig {
            name: Some("sprite"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/sprite.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let rainbow = gpu.create_shader(ShaderConfig {
            name: Some("rainbow"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/rainbow.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::SingleUniform],
            ..Default::default()
        });

        let grey = gpu.create_shader(ShaderConfig {
            name: Some("grey"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/grey.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let blurr = gpu.create_shader(ShaderConfig {
            name: Some("blurr"),
            source: ShaderModuleSoure::Seperate {
                vertex: &shared_resources.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/blurr.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let size = surface.size();
        let times = Uniform::new(gpu, [0.0, 0.0]);
        let centered_instance = gpu.create_instance_buffer(&[Instance2D::default()]);

        let fov = Self::relative_fov(size);

        let relative_bottom_left_camera =
            CameraBuffer2D::new_camera(gpu, Camera2D::new(Isometry2::new(fov, 0.0), fov));
        let relative_bottom_right_camera = CameraBuffer2D::new_camera(
            gpu,
            Camera2D::new(Isometry2::new(Vector2::new(-fov.x, fov.y), 0.0), fov),
        );
        let relative_top_right_camera =
            CameraBuffer2D::new_camera(gpu, Camera2D::new(Isometry2::new(-fov, 0.0), fov));
        let relative_top_left_camera = CameraBuffer2D::new_camera(
            gpu,
            Camera2D::new(Isometry2::new(Vector2::new(fov.x, -fov.y), 0.0), fov),
        );
        let relative_camera =
            CameraBuffer2D::new_camera(gpu, Camera2D::new(Default::default(), fov));
        let unit_camera = CameraBuffer2D::new_camera(
            gpu,
            Camera2D::new(Default::default(), Vector2::new(0.5, 0.5)),
        );
        let world_camera2d = CameraBuffer2D::empty(gpu);
        let world_camera3d = CameraBuffer::empty(gpu);

        let unit_mesh = gpu.create_mesh(&MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5)));

        #[cfg(feature = "framebuffer")]
        let framebuffer = SpriteRenderTarget::new(gpu, size);
        let depth_buffer = DepthBuffer::new(gpu, size);

        let missing = gpu.create_sprite(
            SpriteBuilder::bytes(include_bytes!("../../static/img/missing.png")).sampler(
                wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::Repeat,
                    address_mode_v: wgpu::AddressMode::Repeat,
                    address_mode_w: wgpu::AddressMode::Repeat,
                    ..Sprite::DEFAULT_SAMPLER
                },
            ),
        );

        Self {
            sprite_sheet,
            #[cfg(feature = "text")]
            text,
            sprite,
            rainbow,
            grey,
            blurr,
            color,

            // test,
            model,
            unit_mesh,
            depth_buffer,
            missing,

            times,
            unit_camera,
            centered_instance,
            relative_camera,
            relative_bottom_left_camera,
            relative_bottom_right_camera,
            relative_top_left_camera,
            relative_top_right_camera,
            world_camera2d,
            world_camera3d,

            #[cfg(feature = "framebuffer")]
            framebuffer,
        }
    }

    #[cfg(feature = "framebuffer")]
    pub(crate) fn apply_render_scale(&mut self, surface: &Surface, gpu: &Gpu, scale: f32) {
        let size = surface.size().cast::<f32>() * scale;
        let size = Vector2::new(size.x as u32, size.y as u32);
        if self.framebuffer.size() != size {
            self.framebuffer = gpu.create_render_target(size);
        }
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, window_size: Vector2<u32>) {
        #[cfg(feature = "framebuffer")]
        self.framebuffer.resize(gpu, window_size);

        self.depth_buffer.resize(gpu, window_size);

        let fov = Self::relative_fov(window_size);
        self.relative_bottom_left_camera.1 = Camera2D::new(Isometry2::new(fov, 0.0), fov);
        self.relative_bottom_right_camera.1 =
            Camera2D::new(Isometry2::new(Vector2::new(-fov.x, fov.y), 0.0), fov);
        self.relative_top_right_camera.1 = Camera2D::new(Isometry2::new(-fov, 0.0), fov);
        self.relative_top_left_camera.1 =
            Camera2D::new(Isometry2::new(Vector2::new(fov.x, -fov.y), 0.0), fov);
        self.relative_camera.1 = Camera2D::new(Isometry2::default(), fov);

        self.relative_bottom_left_camera
            .0
            .write(gpu, &self.relative_bottom_left_camera.1);
        self.relative_bottom_right_camera
            .0
            .write(gpu, &self.relative_bottom_right_camera.1);
        self.relative_top_right_camera
            .0
            .write(gpu, &self.relative_top_right_camera.1);
        self.relative_top_left_camera
            .0
            .write(gpu, &self.relative_top_left_camera.1);
        self.relative_camera.0.write(gpu, &self.relative_camera.1);
    }

    pub fn unit_mesh(&self) -> &Mesh2D {
        &self.unit_mesh
    }

    fn relative_fov(window_size: Vector2<u32>) -> Vector2<f32> {
        let yx = window_size.y as f32 / window_size.x as f32;
        let xy = window_size.x as f32 / window_size.y as f32;
        let scale = yx.max(xy) / 2.0;
        if window_size.x > window_size.y {
            Vector2::new(scale, RELATIVE_CAMERA_SIZE)
        } else {
            Vector2::new(RELATIVE_CAMERA_SIZE, scale)
        }
    }
}
