#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "text")]
use crate::text::{FontBrush, TextPipeline};
use crate::{
    Camera, CameraBuffer, Color, ColorWrites, InstanceBuffer, InstanceData, Isometry, Model,
    ModelBuilder, RenderEncoder, RenderTarget, Shader, ShaderConfig, ShaderField, Sprite,
    SpriteSheet, Uniform, Vector,
};
use std::{borrow::Cow, sync::Mutex};
use wgpu::{util::DeviceExt, BlendState};

pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 0.5;

#[derive(Clone)]
/// Configuration of the [wgpu](https://github.com/gfx-rs/wgpu) limits, features and backend graphics api
pub struct GpuConfig {
    pub backends: wgpu::Backends,
    pub device_features: wgpu::Features,
    pub device_limits: wgpu::Limits,
    pub max_multisample: u8,
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
            max_multisample: 2,
        }
    }
}

/// Holds the connection to the GPU using wgpu. Also has some default buffers, layouts etc.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub commands: Mutex<Vec<wgpu::CommandBuffer>>,
    pub(crate) base: WgpuBase,
}

impl Gpu {
    pub(crate) async fn new(window: &winit::window::Window, config: GpuConfig) -> Self {
        let window_size = window.inner_size();
        let window_size = Vector::new(window_size.width, window_size.height);
        let max_multisample = config.max_multisample;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: config.backends,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: config.device_features,
                    limits: config.device_limits,
                },
                None,
            )
            .await
            .unwrap();

        let config = surface
            .get_default_config(&adapter, window_size.x, window_size.y)
            .expect("Surface unsupported by adapter");

        let sample_flags = adapter.get_texture_format_features(config.format).flags;
        let sample_count = {
            if max_multisample >= 16
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X16)
            {
                16
            } else if max_multisample >= 8
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8)
            {
                8
            } else if max_multisample >= 4
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            {
                4
            } else if max_multisample >= 2
                && sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2)
            {
                2
            } else {
                1
            }
        };

        let base = WgpuBase::new(&device, sample_count);

        surface.configure(&device, &config);

        #[cfg(feature = "log")]
        {
            let adapter_info = adapter.get_info();
            info!("Using GPU: {}", adapter_info.name);
            info!("Using WGPU backend: {:?}", adapter_info.backend);
            info!("Using Multisample X{sample_count}");
            info!("Using TextureFormat: {:?}", config.format);
        }

        let gpu = Self {
            instance,
            queue,
            surface,
            config,
            device,
            adapter,
            base,
            commands: Mutex::new(vec![]),
        };

        return gpu;
    }

    #[cfg(target_os = "android")]
    pub(crate) fn resume(&mut self, window: &winit::window::Window) {
        self.surface = unsafe { self.instance.create_surface(window).unwrap() };
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn resize(&mut self, window_size: Vector<u32>) {
        self.config.width = window_size.x;
        self.config.height = window_size.y;
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn apply_vsync(&mut self, vsync: bool) {
        let new_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        self.config.present_mode = new_mode;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn block(&self, handle: wgpu::SubmissionIndex) {
        self.device
            .poll(wgpu::MaintainBase::WaitForSubmissionIndex(handle));
    }

    pub fn submit(&self) -> wgpu::SubmissionIndex {
        let mut buffers = self.commands.lock().unwrap();
        self.queue.submit(buffers.drain(..))
    }

    pub fn stage(&self, buffer: wgpu::CommandBuffer) {
        let mut buffers = self.commands.lock().unwrap();
        buffers.push(buffer)
    }

    pub fn render_size(&self, scale: f32) -> Vector<u32> {
        Vector::new(
            (self.config.width as f32 * scale) as u32,
            (self.config.height as f32 * scale) as u32,
        )
    }

    pub fn render_size_no_scale(&self) -> Vector<u32> {
        Vector::new(self.config.width, self.config.height)
    }

    pub fn create_render_target(&self, size: Vector<u32>) -> RenderTarget {
        RenderTarget::new(self, size)
    }

    pub fn create_camera_buffer(&self, camera: &Camera) -> CameraBuffer {
        camera.create_buffer(self)
    }

    pub fn create_instance_buffer(&self, instances: &[InstanceData]) -> InstanceBuffer {
        InstanceBuffer::new(self, instances)
    }

    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(self, builder)
    }

    pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
        Sprite::new(self, bytes)
    }

    pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
        Sprite::from_image(self, image)
    }

    pub fn create_empty_sprite(&self, size: Vector<u32>) -> Sprite {
        Sprite::empty(self, size)
    }

    pub fn create_sprite_sheet(&self, bytes: &[u8], sprites: Vector<u32>) -> SpriteSheet {
        SpriteSheet::new(self, bytes, sprites)
    }

    pub fn create_sprite_sheet_from_amount(
        &self,
        bytes: &[u8],
        sprites: Vector<u32>,
    ) -> SpriteSheet {
        SpriteSheet::from_amount(self, bytes, sprites)
    }

    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8], max_chars: u64) -> FontBrush {
        FontBrush::new(self, bytes, max_chars).unwrap()
    }

    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(self, data)
    }

    pub fn create_color(&self, color: Color) -> Sprite {
        Sprite::from_color(self, color)
    }

    pub fn create_color_sheet(&self, colors: &[Color]) -> SpriteSheet {
        SpriteSheet::from_colors(self, colors)
    }

    pub fn create_shader(&self, config: ShaderConfig) -> Shader {
        Shader::new(self, config)
    }

    pub fn create_computed_target<'caller>(
        &self,
        defaults: &GpuDefaults,
        texture_size: Vector<u32>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> RenderTarget {
        return RenderTarget::computed(self, defaults, texture_size, compute);
    }
}

/// Base Wgpu objects needed to create any further graphics object.
pub struct WgpuBase {
    pub sample_count: u32,
    pub multisample: wgpu::MultisampleState,
    pub no_multisample: wgpu::MultisampleState,
    pub sprite_sheet_layout: wgpu::BindGroupLayout,
    pub sprite_layout: wgpu::BindGroupLayout,
    pub camera_layout: wgpu::BindGroupLayout,
    pub uniform_layout: wgpu::BindGroupLayout,
    pub vertex_shader: wgpu::ShaderModule,
    pub texture_sampler: wgpu::Sampler,
    #[cfg(feature = "text")]
    pub text_pipeline: TextPipeline,
}

impl WgpuBase {
    pub fn new(device: &wgpu::Device, sample_count: u32) -> Self {
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

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vertex_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(Shader::VERTEX)),
        });

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let multisample = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let no_multisample = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        #[cfg(feature = "text")]
        let text_pipeline = TextPipeline::new(device, multisample);

        Self {
            sample_count: sample_count,
            multisample,
            sprite_sheet_layout,
            no_multisample,
            sprite_layout,
            camera_layout,
            uniform_layout,
            vertex_shader,
            texture_sampler,
            #[cfg(feature = "text")]
            text_pipeline,
        }
    }
}

/// Holds default buffers, shaders, sprites and layouts needed by shura.
pub struct GpuDefaults {
    pub sprite: Shader,
    pub sprite_sheet: Shader,
    pub sprite_sheet_uniform: Shader,
    pub rainbow: Shader,
    pub grey: Shader,
    pub blurr: Shader,
    pub sprite_no_msaa: Shader,

    pub cuboid_index_buffer: wgpu::Buffer,
    pub triangle_index_buffer: wgpu::Buffer,

    /// This field holds both total time and the frame time. Both are stored as f32 in the buffer.
    /// The first f32 is the `total_time` and the second f32 is the `frame_time`. In the shader
    /// the struct also needs 2 additional floats which are empty to match the 16 byte alignment
    /// some devices need.
    pub times: Uniform<[f32; 2]>,
    /// Camera where the smaller side is always 1.0 and the otherside is scaled to match the window aspect ratio.
    pub relative_camera: (CameraBuffer, Camera),
    pub relative_bottom_left_camera: (CameraBuffer, Camera),
    pub relative_bottom_right_camera: (CameraBuffer, Camera),
    pub relative_top_left_camera: (CameraBuffer, Camera),
    pub relative_top_right_camera: (CameraBuffer, Camera),
    pub unit_camera: (CameraBuffer, Camera),
    pub world_camera: CameraBuffer,
    pub single_centered_instance: InstanceBuffer,
    pub empty_instance: InstanceBuffer,
    pub world_target: RenderTarget,
}

impl GpuDefaults {
    pub(crate) fn new(gpu: &Gpu, window_size: Vector<u32>) -> Self {
        let sprite_no_msaa = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::SPRITE,
            shader_fields: &[ShaderField::Sprite],
            msaa: false,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
            render_to_surface: true,
        });

        let sprite_sheet = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::SPRITE_SHEET,
            shader_fields: &[ShaderField::SpriteSheet],
            ..Default::default()
        });

        let sprite_sheet_uniform = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::SPRITE_SHEET_UNIFORM,
            shader_fields: &[ShaderField::SpriteSheet, ShaderField::Uniform],
            ..Default::default()
        });

        let sprite = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::SPRITE,
            shader_fields: &[ShaderField::Sprite],
            ..Default::default()
        });

        let rainbow = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::RAINBOW,
            shader_fields: &[ShaderField::Uniform],
            ..Default::default()
        });

        let grey = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::GREY,
            shader_fields: &[ShaderField::Sprite],
            ..Default::default()
        });

        let blurr = gpu.create_shader(ShaderConfig {
            fragment_source: Shader::BLURR,
            shader_fields: &[ShaderField::Sprite],
            ..Default::default()
        });

        let size = gpu.render_size(1.0);
        let world_target = gpu.create_render_target(size);
        let times = Uniform::new(gpu, [0.0, 0.0]);
        let single_centered_instance = gpu.create_instance_buffer(&[InstanceData::new(
            Default::default(),
            Vector::new(1.0, 1.0),
            Vector::new(0, 0),
        )]);
        let empty_instance = gpu.create_instance_buffer(&[]);

        let fov = Self::relative_fov(window_size);

        let camera = Camera::new(Isometry::new(fov, 0.0), fov);
        let relative_bottom_left_camera = (camera.create_buffer(gpu), camera);

        let camera = Camera::new(Isometry::new(Vector::new(-fov.x, fov.y), 0.0), fov);
        let relative_bottom_right_camera = (camera.create_buffer(gpu), camera);

        let camera = Camera::new(Isometry::new(-fov, 0.0), fov);
        let relative_top_right_camera = (camera.create_buffer(gpu), camera);

        let camera = Camera::new(Isometry::new(Vector::new(fov.x, -fov.y), 0.0), fov);
        let relative_top_left_camera = (camera.create_buffer(gpu), camera);

        let camera = Camera::new(Default::default(), fov);
        let world_camera = camera.create_buffer(gpu);
        let relative_camera = (camera.create_buffer(gpu), camera);

        let camera = Camera::new(Default::default(), Vector::new(0.5, 0.5));
        let unit_camera = (camera.create_buffer(gpu), camera);

        let cuboid_index_buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cuboid_index_buffer"),
                    contents: bytemuck::cast_slice(&ModelBuilder::CUBOID_INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let triangle_index_buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangle_index_buffer"),
                    contents: bytemuck::cast_slice(&ModelBuilder::TRIANGLE_INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                });

        Self {
            sprite_sheet_uniform,
            sprite_sheet,
            cuboid_index_buffer,
            triangle_index_buffer,
            unit_camera,
            sprite,
            rainbow,
            sprite_no_msaa,
            grey,
            blurr,
            times,
            single_centered_instance,
            empty_instance,
            relative_camera,
            relative_bottom_left_camera,
            relative_bottom_right_camera,
            relative_top_left_camera,
            relative_top_right_camera,
            world_camera,
            world_target,
        }
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, window_size: Vector<u32>) {
        let fov = Self::relative_fov(window_size);
        self.relative_bottom_left_camera.1 = Camera::new(Isometry::new(fov, 0.0), fov);
        self.relative_bottom_right_camera.1 =
            Camera::new(Isometry::new(Vector::new(-fov.x, fov.y), 0.0), fov);
        self.relative_top_right_camera.1 = Camera::new(Isometry::new(-fov, 0.0), fov);
        self.relative_top_left_camera.1 =
            Camera::new(Isometry::new(Vector::new(fov.x, -fov.y), 0.0), fov);
        self.relative_camera.1 = Camera::new(Isometry::default(), fov);

        self.relative_bottom_left_camera
            .1
            .write_buffer(gpu, &mut self.relative_bottom_left_camera.0);
        self.relative_bottom_right_camera
            .1
            .write_buffer(gpu, &mut self.relative_bottom_right_camera.0);
        self.relative_top_right_camera
            .1
            .write_buffer(gpu, &mut self.relative_top_right_camera.0);
        self.relative_top_left_camera
            .1
            .write_buffer(gpu, &mut self.relative_top_left_camera.0);
        self.relative_camera
            .1
            .write_buffer(gpu, &mut self.relative_camera.0);
    }

    pub(crate) fn buffer(
        &mut self,
        active_scene_camera: &mut Camera,
        gpu: &Gpu,
        total_time: f32,
        frame_time: f32,
    ) {
        active_scene_camera.write_buffer(gpu, &mut self.world_camera);
        self.times.write(&gpu, [total_time, frame_time]);
    }

    pub(crate) fn apply_render_scale(&mut self, gpu: &Gpu, scale: f32) {
        let size = gpu.render_size(scale);
        if self.world_target.size() != size {
            self.world_target = gpu.create_render_target(size);
        }
    }

    fn relative_fov(window_size: Vector<u32>) -> Vector<f32> {
        let yx = window_size.y as f32 / window_size.x as f32;
        let xy = window_size.x as f32 / window_size.y as f32;
        let scale = yx.max(xy) / 2.0;
        return if window_size.x > window_size.y {
            Vector::new(scale, RELATIVE_CAMERA_SIZE)
        } else {
            Vector::new(RELATIVE_CAMERA_SIZE, scale)
        };
    }
}
