use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use parking_lot::Mutex;
use wgpu::include_wgsl;
use winit::window::Window;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "text")]
use crate::text::{Font, FontBuilder, TextMesh, TextSection, TextVertex2D};
use crate::{
    graphics::{
        Camera, Camera2D, CameraBuffer, CameraBuffer2D, ColorInstance2D, ColorVertex2D,
        DepthBuffer, Instance, Instance3D, InstanceBuffer, Mesh, MeshData, MeshData2D, Model,
        ModelBuilder, PositionVertex2D, RenderEncoder, Shader, ShaderConfig, ShaderModule,
        ShaderModuleDescriptor, ShaderModuleSource, Sprite, SpriteArray, SpriteArrayBuilder,
        SpriteArrayCropInstance2D, SpriteArrayInstance2D, SpriteArrayVertex2D, SpriteBuilder,
        SpriteCropInstance2D, SpriteInstance2D, SpriteMesh2D, SpriteRenderTarget, SpriteVertex2D,
        SurfaceRenderTarget, UniformData, UniformField, Vertex, Vertex3D, VertexBuffers,
        WorldCamera3D,
    },
    math::{Isometry2, Vector2},
};

use super::PositionMesh2D;

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
                    memory_hints: wgpu::MemoryHints::Performance,
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
            surface_size: Default::default(),
            target_msaa: Default::default(),
        };

        gpu.resume(&window);

        gpu
    }

    pub(crate) fn compute_surface_size(window: &Window) -> Vector2<u32> {
        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);
        Vector2::new(width, height)
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

        let config = Self::default_config(&self.surface, &self.adapter, window);
        self.update_msaa(Vector2::new(config.width, config.height));
        self.surface.configure(&self.device, &config);
        *self.surface_size.lock() = Vector2::new(config.width, config.height);
        *self.config.lock() = config;
    }

    /// Resize the surface, making sure to not resize to zero.
    pub(crate) fn resize(&self, size: Vector2<u32>) {
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

    pub fn create_mesh<V: Vertex>(&self, builder: &dyn MeshData<Vertex = V>) -> Mesh<V> {
        Mesh::new(self, builder)
    }

    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(self, builder)
    }

    pub fn create_sprite<D: Deref<Target = [u8]>>(&self, desc: SpriteBuilder<D>) -> Sprite {
        Sprite::new(self, desc)
    }

    pub fn create_sprite_array<D: Deref<Target = [u8]>>(
        &self,
        desc: SpriteArrayBuilder<D>,
    ) -> SpriteArray {
        SpriteArray::new(self, desc)
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
        SpriteRenderTarget::computed(sprite, compute)
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

    pub fn surface_size(&self) -> Vector2<u32> {
        *self.surface_size.lock()
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
    pub sprite_array_layout: wgpu::BindGroupLayout,
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

        let sprite_array_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_array_layout"),
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

        Self {
            sprite_array_layout,
            sprite_layout,
            camera_layout,
            single_uniform_layout,
        }
    }
}

#[derive(Debug)]
pub struct DefaultAssets {
    // 2D
    pub sprite_shader: Shader,
    pub color_shader: Shader,
    pub sprite_array_shader: Shader,
    pub sprite_crop_shader: Shader,
    pub sprite_array_crop_shader: Shader,

    pub mesh_color_shader: Shader,
    pub mesh_sprite_shader: Shader,
    pub mesh_sprite_array_shader: Shader,
    pub mesh_text_shader: Shader,

    pub missing_sprite: Sprite,

    // 3D
    pub model_shader: Shader,
    pub depth_buffer: DepthBuffer,

    pub sprite_mesh: SpriteMesh2D,
    pub position_mesh: PositionMesh2D,
    pub times: UniformData<[f32; 2]>,
    pub world_camera2d: CameraBuffer2D,
    pub world_camera3d: CameraBuffer<WorldCamera3D>,
    pub relative_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_bottom_right_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_left_camera: (CameraBuffer2D, Camera2D),
    pub relative_top_right_camera: (CameraBuffer2D, Camera2D),
    pub unit_camera: (CameraBuffer2D, Camera2D),
    #[cfg(feature = "framebuffer")]
    pub framebuffer: SpriteRenderTarget,
}

impl DefaultAssets {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let model_shader = gpu.create_shader(ShaderConfig {
            name: Some("model"),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/3d/model.wgsl")),
            ),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::DEPTH_FORMAT_3D,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            vertex_buffers: VertexBuffers::instance::<Vertex3D, Instance3D>(),
            ..Default::default()
        });

        let color_shader = gpu.create_shader(ShaderConfig {
            name: Some("color"),
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/color.wgsl")),
            ),
            uniforms: &[UniformField::Camera],
            vertex_buffers: VertexBuffers::instance::<PositionVertex2D, ColorInstance2D>(),
            ..Default::default()
        });

        let sprite_shader = gpu.create_shader(ShaderConfig {
            name: Some("sprite"),
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/sprite.wgsl")),
            ),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, SpriteInstance2D>(),
            ..Default::default()
        });

        let sprite_crop_shader = gpu.create_shader(ShaderConfig {
            name: Some("sprite_crop"),
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/sprite_crop.wgsl")),
            ),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, SpriteCropInstance2D>(),
            ..Default::default()
        });

        let sprite_array_shader =
            gpu.create_shader(ShaderConfig {
                name: Some("sprite_array"),
                source: ShaderModuleSource::Single(&gpu.create_shader_module(include_wgsl!(
                    "../../static/shader/2d/sprite_array.wgsl"
                ))),
                uniforms: &[UniformField::Camera, UniformField::SpriteArray],
                vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, SpriteArrayInstance2D>(),
                ..Default::default()
            });

        let sprite_array_crop_shader = gpu.create_shader(ShaderConfig {
            name: Some("sprite_array_crop"),
            source: ShaderModuleSource::Single(&gpu.create_shader_module(include_wgsl!(
                "../../static/shader/2d/sprite_array_crop.wgsl"
            ))),
            uniforms: &[UniformField::Camera, UniformField::SpriteArray],
            vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, SpriteArrayCropInstance2D>(),
            ..Default::default()
        });

        let mesh_color_shader = gpu.create_shader(ShaderConfig {
            name: Some("mesh_color"),
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/mesh_color.wgsl")),
            ),
            uniforms: &[UniformField::Camera],
            vertex_buffers: VertexBuffers::vertex::<ColorVertex2D>(),
            ..Default::default()
        });

        let mesh_sprite_shader = gpu.create_shader(ShaderConfig {
            name: Some("mesh_sprite"),
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/mesh_sprite.wgsl")),
            ),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            vertex_buffers: VertexBuffers::vertex::<SpriteVertex2D>(),
            ..Default::default()
        });

        let mesh_sprite_array_shader = gpu.create_shader(ShaderConfig {
            name: Some("mesh_sprite_array"),
            source: ShaderModuleSource::Single(&gpu.create_shader_module(include_wgsl!(
                "../../static/shader/2d/mesh_sprite_array.wgsl"
            ))),
            uniforms: &[UniformField::Camera, UniformField::SpriteArray],
            vertex_buffers: VertexBuffers::vertex::<SpriteArrayVertex2D>(),
            ..Default::default()
        });

        #[cfg(feature = "text")]
        let mesh_text_shader = gpu.create_shader(ShaderConfig {
            name: Some("mesh_text"),
            vertex_buffers: VertexBuffers::vertex::<TextVertex2D>(),
            uniforms: &[UniformField::Camera, UniformField::SpriteArray],
            source: ShaderModuleSource::Single(
                &gpu.create_shader_module(include_wgsl!("../../static/shader/2d/mesh_text.wgsl")),
            ),
            ..Default::default()
        });
        // let rainbow_shader = gpu.create_shader(ShaderConfig::<Vertex2D, Instance2D> {
        //     name: Some("rainbow"),
        //     source: ShaderModuleSource::Separate {
        //         vertex: &shared_assets.vertex_shader_module,
        //         fragment: &gpu
        //             .create_shader_module(include_wgsl!("../../static/shader/2d/rainbow.wgsl")),
        //     },
        //     uniforms: &[UniformField::Camera, UniformField::SingleUniform],
        //     ..Default::default()
        // });

        // let grey_shader = gpu.create_shader(ShaderConfig::<Vertex2D, Instance2D> {
        //     name: Some("grey"),
        //     source: ShaderModuleSource::Separate {
        //         vertex: &shared_assets.vertex_shader_module,
        //         fragment: &gpu
        //             .create_shader_module(include_wgsl!("../../static/shader/2d/grey.wgsl")),
        //     },
        //     uniforms: &[UniformField::Camera, UniformField::Sprite],
        //     ..Default::default()
        // });

        // let blurr_shader = gpu.create_shader(ShaderConfig::<Vertex2D, Instance2D> {
        //     name: Some("blurr"),
        //     source: ShaderModuleSource::Separate {
        //         vertex: &shared_assets.vertex_shader_module,
        //         fragment: &gpu
        //             .create_shader_module(include_wgsl!("../../static/shader/2d/blurr.wgsl")),
        //     },
        //     uniforms: &[UniformField::Camera, UniformField::Sprite],
        //     ..Default::default()
        // });

        let size = gpu.surface_size();
        let times = UniformData::new(gpu, [0.0, 0.0]);

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

        let sprite_mesh = gpu.create_mesh(&MeshData2D::cuboid(Vector2::new(0.5, 0.5)));
        let position_mesh = gpu.create_mesh(&MeshData2D::cuboid(Vector2::new(0.5, 0.5)));

        #[cfg(feature = "framebuffer")]
        let framebuffer = SpriteRenderTarget::new(gpu, size);
        let depth_buffer = DepthBuffer::new(gpu, size, DepthBuffer::DEPTH_FORMAT_3D);

        let missing_sprite = gpu.create_sprite(
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
            sprite_shader,
            color_shader,
            sprite_array_shader,
            sprite_crop_shader,
            sprite_array_crop_shader,
            mesh_sprite_array_shader,
            mesh_color_shader,
            mesh_sprite_shader,
            #[cfg(feature = "text")]
            mesh_text_shader,
            model_shader,
            sprite_mesh,
            depth_buffer,
            position_mesh,
            missing_sprite,

            times,
            unit_camera,
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
