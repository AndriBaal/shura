use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

use parking_lot::{Mutex, RwLock};
use wgpu::include_wgsl;
use winit::window::Window;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "text")]
use crate::text::{Font, FontBuilder, TextMesh, TextSection};
use crate::{
    graphics::{
        Camera, Camera2D, CameraBuffer, CameraBuffer2D, DepthBuffer, Instance, Instance2D,
        Instance3D, InstanceBuffer, InstanceBuffer2D, Mesh, Mesh2D, MeshBuilder, MeshBuilder2D,
        Model, ModelBuilder, RenderEncoder, Shader, ShaderConfig, ShaderModule,
        ShaderModuleDescriptor, ShaderModuleSource, Sprite, SpriteBuilder, SpriteRenderTarget,
        SpriteSheet, SpriteSheetBuilder, UniformData, UniformField, Vertex, Vertex3D,
        WorldCamera3D,
    },
    math::{Isometry2, Vector2},
};

use super::SurfaceRenderTarget;

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
    pub surface: wgpu::Surface<'static>,
    pub command_buffers: Mutex<Vec<wgpu::CommandBuffer>>,
    pub config: Mutex<wgpu::SurfaceConfiguration>,

    format: wgpu::TextureFormat,
    shared_assets: SharedAssets,
    surface_size: Mutex<Vector2<u32>>,
    target_msaa: Mutex<Option<wgpu::Texture>>,
    default_assets: OnceLock<RwLock<DefaultAssets>>,

    samples: u32,
    sample_state: wgpu::MultisampleState,
}

impl Gpu {
    pub(crate) async fn new(window: Arc<Window>, gpu_config: GpuConfig) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: gpu_config.backends,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            ..Default::default()
        });
        // Important: Request surface before adapter!
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Invalid Graphics Backend!");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: gpu_config.device_features,
                    required_limits: gpu_config.device_limits.using_resolution(adapter.limits()),
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

        let config = Self::default_config(&surface, &adapter, &window);
        let format = config.format;
        let max_samples = gpu_config.max_samples;
        let sample_flags = adapter.get_texture_format_features(config.format).flags;
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
        #[cfg(feature = "log")]
        {
            info!("Using multisample X{samples}");
            info!("Using texture format: {:?}", config.format);
            info!("Using Present mode: {:?}", config.present_mode);
        }

        let gpu = Self {
            shared_assets: SharedAssets::new(&device),
            config: Mutex::new(config),
            surface,
            instance,
            queue,
            device,
            adapter,
            command_buffers: Mutex::new(Default::default()),
            format,
            samples,
            sample_state,

            // These get initialized below
            default_assets: OnceLock::new(),
            surface_size: Default::default(),
            target_msaa: Default::default(),
        };

        gpu.resume(&window);
        gpu.default_assets
            .set(RwLock::new(DefaultAssets::new(&gpu)))
            .unwrap();

        return gpu;
    }

    pub(crate) fn compute_surface_size(window: &Window) -> Vector2<u32> {
        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);
        return Vector2::new(width, height);
    }

    pub(crate) fn default_config(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        window: &Window,
    ) -> wgpu::SurfaceConfiguration {
        let surface_size = Self::compute_surface_size(window);
        surface
            .get_default_config(adapter, surface_size.x, surface_size.y)
            .expect("Surface isn't supported by the adapter.")
    }

    pub(crate) fn resume(&self, window: &Window) {
        #[cfg(feature = "log")]
        log::info!("Surface resume");

        let config = Self::default_config(&self.surface, &self.adapter, &window);
        self.update_msaa(Vector2::new(config.width, config.height));
        self.surface.configure(&self.device, &config);
        *self.surface_size.lock() = Vector2::new(config.width, config.height);
        *self.config.lock() = config;
    }

    /// Resize the surface, making sure to not resize to zero.
    pub(crate) fn resize(&self, size: Vector2<u32>) {
        #[cfg(feature = "log")]
        log::info!("Surface resize {size:?}");
        self.update_msaa(size);

        let mut config = self.config.lock();
        config.width = size.x.max(1);
        config.height = size.y.max(1);
        *self.surface_size.lock() = Vector2::new(config.width, config.height);
        self.surface.configure(&self.device, &config);
    }

    pub(crate) fn start_frame(&self, gpu: &Gpu) -> SurfaceRenderTarget {
        let config = self.config.lock();
        let surface_texture = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            // If we timed out, just try again
            Err(wgpu::SurfaceError::Timeout) => self.surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                self.surface.configure(&gpu.device, &config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        };

        return SurfaceRenderTarget {
            target_view: surface_texture.texture.create_view(&Default::default()),
            msaa_view: self
                .target_msaa
                .lock()
                .as_ref()
                .map(|msaa| msaa.create_view(&Default::default())),
            surface_texture,
        };
    }

    pub(crate) fn apply_vsync(&self, vsync: bool) {
        let mut config = self.config.lock();
        let new_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        config.present_mode = new_mode;
        self.surface.configure(&self.device, &config);
    }

    pub(crate) fn update_msaa(&self, size: Vector2<u32>) {
        let mut target_msaa = self.target_msaa.lock();
        if self.samples() != 1 && (size != self.surface_size() || target_msaa.is_none()) {
            *target_msaa = Some(SpriteRenderTarget::create_msaa(self, size));
        }
    }

    pub fn block(&self, handle: wgpu::SubmissionIndex) {
        self.device
            .poll(wgpu::MaintainBase::WaitForSubmissionIndex(handle));
    }

    pub fn submit(&self) -> wgpu::SubmissionIndex {
        let mut command_buffers = self.command_buffers.lock();
        let command_buffers = std::mem::take(&mut *command_buffers);
        self.queue.submit(command_buffers)
    }

    pub fn create_render_target(&self, size: Vector2<u32>) -> SpriteRenderTarget {
        SpriteRenderTarget::new(self, size)
    }

    pub fn create_depth_buffer(
        &self,
        size: Vector2<u32>,
        format: wgpu::TextureFormat,
    ) -> DepthBuffer {
        DepthBuffer::new(self, size, format)
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

    pub fn create_uniform_data<T: bytemuck::Pod>(&self, data: T) -> UniformData<T> {
        UniformData::new(self, data)
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
    pub fn create_text_mesh<S: AsRef<str>>(
        &self,
        font: &Font,
        sections: &[TextSection<S>],
    ) -> TextMesh {
        TextMesh::new(self, font, sections)
    }

    pub fn create_computed_target<D: Deref<Target = [u8]>>(
        &self,
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> SpriteRenderTarget {
        SpriteRenderTarget::computed(self, sprite, compute)
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn sample_state(&self) -> wgpu::MultisampleState {
        self.sample_state
    }

    pub fn shared_assets(&self) -> &SharedAssets {
        &self.shared_assets
    }

    pub fn default_assets(&self) -> impl Deref<Target = DefaultAssets> + '_ {
        self.default_assets.get().unwrap().read()
    }

    pub fn default_assets_mut(&self) -> impl DerefMut<Target = DefaultAssets> + '_ {
        self.default_assets.get().unwrap().write()
    }

    pub fn surface_size(&self) -> Vector2<u32> {
        self.surface_size.lock().clone()
    }

    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn surface_config(&self) -> wgpu::SurfaceConfiguration {
        self.config.lock().clone()
    }
}

#[derive(Debug)]
pub struct SharedAssets {
    pub vertex_shader_module: ShaderModule,
    pub sprite_sheet_layout: wgpu::BindGroupLayout,
    pub sprite_layout: wgpu::BindGroupLayout,
    pub camera_layout: wgpu::BindGroupLayout,
    pub single_uniform_layout: wgpu::BindGroupLayout,
}

impl SharedAssets {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
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
pub struct DefaultAssets {
    // 2D
    pub sprite: Shader,
    pub sprite_sheet: Shader,
    pub color: Shader,
    pub rainbow: Shader,
    pub grey: Shader,
    #[cfg(feature = "text")]
    pub text_mesh: Shader,
    #[cfg(feature = "text")]
    pub text_instance: Shader,
    pub blurr: Shader,

    pub missing: Sprite,

    // 3D
    pub model: Shader,
    pub depth_buffer: DepthBuffer,
    pub unit_mesh: Mesh2D,

    pub times: UniformData<[f32; 2]>,
    pub world_camera2d: CameraBuffer2D,
    pub world_camera3d: CameraBuffer<WorldCamera3D>,
    pub relative_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_right_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_right_camera: (CameraBuffer2D, Camera2D),
    pub unit_camera: (CameraBuffer2D, Camera2D),
    pub single_instance: InstanceBuffer2D,

    #[cfg(feature = "framebuffer")]
    pub framebuffer: SpriteRenderTarget,
}

impl DefaultAssets {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let shared_assets = gpu.shared_assets();
        let sprite_sheet = gpu.create_shader(ShaderConfig {
            name: Some("sprite_sheet"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!(
                    "../../static/shader/2d/sprite_sheet.wgsl"
                )),
            },
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            ..Default::default()
        });

        #[cfg(feature = "text")]
        let text_mesh = gpu.create_shader(ShaderConfig {
            name: Some("text_vertex"),
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            source: ShaderModuleSource::Separate {
                vertex: &gpu.create_shader_module(include_wgsl!(
                    "../../static/shader/2d/vertex_text_mesh.wgsl"
                )),
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/text.wgsl")),
            },
            buffers: &[
                crate::text::Vertex2DText::LAYOUT,
                wgpu::VertexBufferLayout {
                    array_stride: Instance2D::SIZE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        4 => Float32x2,
                        5 => Float32x2,
                        6 => Float32,
                        7 => Float32x2,
                        8 => Float32x2,
                        9 => Float32x4,
                        10 => Uint32,
                    ],
                },
            ],
            ..Default::default()
        });

        #[cfg(feature = "text")]
        let text_instance = gpu.create_shader(ShaderConfig {
            name: Some("text_instance"),
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/text.wgsl")),
            },
            buffers: &[
                crate::graphics::Vertex2D::LAYOUT,
                crate::text::LetterInstance2D::LAYOUT,
            ],
            ..Default::default()
        });

        let model = gpu.create_shader(ShaderConfig {
            name: Some("model"),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/3d/model.wgsl")),
            ),
            buffers: &[Vertex3D::LAYOUT, Instance3D::LAYOUT],
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::DEPTH_FORMAT_3D,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            ..Default::default()
        });

        let color = gpu.create_shader(ShaderConfig {
            name: Some("color"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/color.wgsl")),
            },
            uniforms: &[UniformField::Camera],
            ..Default::default()
        });

        let sprite = gpu.create_shader(ShaderConfig {
            name: Some("sprite"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/sprite.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let rainbow = gpu.create_shader(ShaderConfig {
            name: Some("rainbow"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/rainbow.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::SingleUniform],
            ..Default::default()
        });

        let grey = gpu.create_shader(ShaderConfig {
            name: Some("grey"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/grey.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let blurr = gpu.create_shader(ShaderConfig {
            name: Some("blurr"),
            source: ShaderModuleSource::Separate {
                vertex: &shared_assets.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/blurr.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let size = gpu.surface_size();
        let times = UniformData::new(gpu, [0.0, 0.0]);
        let single_instance = gpu.create_instance_buffer(&[Instance2D::default()]);

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
        let depth_buffer = DepthBuffer::new(gpu, size, DepthBuffer::DEPTH_FORMAT_3D);

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
            text_mesh,
            #[cfg(feature = "text")]
            text_instance,
            sprite,
            rainbow,
            grey,
            blurr,
            color,

            model,
            unit_mesh,
            depth_buffer,
            missing,

            times,
            unit_camera,
            single_instance,
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
    pub(crate) fn apply_render_scale(
        &mut self,
        gpu: &Gpu,
        screen_config: &crate::graphics::ScreenConfig,
    ) {
        use super::RenderTarget;
        let size = screen_config.render_size(gpu);
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
