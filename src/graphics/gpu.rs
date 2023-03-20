#[cfg(feature = "text")]
use crate::text::{FontBrush, TextDescriptor};
use crate::{
    BufferedCamera, Camera, CameraBuffer, ColorWrites, InstanceBuffer, Isometry, Matrix, Model,
    ModelBuilder, RenderConfig, RenderEncoder, RenderTarget, ScreenConfig, Shader, ShaderConfig,
    ShaderField, ShaderLang, Sprite, SpriteSheet, Uniform, Vector,
};
use log::info;
use std::borrow::Cow;
use wgpu::BlendState;
pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 0.5;

macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

/// Holds the connection to the GPU using wgpu. Also has some default buffers, layouts etc.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub(crate) base: WgpuBase,
}

impl Gpu {
    pub(crate) async fn new(window: &winit::window::Window) -> Self {
        let window_size = window.inner_size();
        let window_size = Vector::new(window_size.width, window_size.height);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    label: None,
                    limits,
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
            if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8) {
                8
            } else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4) {
                4
            } else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2) {
                2
            } else {
                1
            }
        };

        let base = WgpuBase::new(&device, sample_count);

        surface.configure(&device, &config);
        let adapter_info = adapter.get_info();

        info!("Using GPU: {}", adapter_info.name);
        info!("Using WGPU backend: {:?}", adapter_info.backend);
        info!("Using Multisample X{sample_count}");
        info!("Using TextureFormat: {:?}", config.format);

        let gpu = Self {
            instance,
            queue,
            surface,
            config,
            device,
            adapter,
            base,
        };

        return gpu;
    }

    pub(crate) fn resize(&mut self, size: Vector<u32>) {
        self.config.width = size.x;
        self.config.height = size.y;
        self.surface.configure(&self.device, &self.config);
    }

    #[cfg(target_os = "android")]
    pub(crate) fn resume(&mut self, window: &winit::window::Window) {
        self.surface = unsafe { self.instance.create_surface(window).unwrap() };
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

    pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
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

    pub fn create_sprite_sheet(
        &self,
        bytes: &[u8],
        sprites: Vector<u32>,
        sprite_size: Vector<u32>,
    ) -> SpriteSheet {
        SpriteSheet::new(self, bytes, sprites, sprite_size)
    }

    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8]) -> FontBrush {
        FontBrush::new(self, bytes)
    }

    #[cfg(feature = "text")]
    pub fn create_text(
        &self,
        defaults: &GpuDefaults,
        texture_size: Vector<u32>,
        descriptor: TextDescriptor,
    ) -> RenderTarget {
        let target = self.create_render_target(texture_size);
        let mut encoder = RenderEncoder::new(self);
        let config = RenderConfig {
            camera: &defaults.relative_camera,
            instances: &defaults.single_centered_instance,
            target: &target,
            gpu: self,
            defaults: &defaults,
            msaa: true,
        };
        encoder.render_text(&config, descriptor);
        encoder.submit(self);
        return target;
    }

    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(self, data)
    }

    pub fn create_shader(&self, config: ShaderConfig) -> Shader {
        Shader::new(self, config)
    }

    pub fn create_computed_target<'caller>(
        &self,
        defaults: &GpuDefaults,
        texture_size: Vector<u32>,
        compute: impl for<'any> Fn(&mut RenderEncoder, RenderConfig<'any>, [Where!('caller >= 'any); 0]),
    ) -> RenderTarget {
        return RenderTarget::computed(self, &defaults, texture_size, compute);
    }
}

/// Base Wgpu objects needed to create any further graphics object.
pub struct WgpuBase {
    pub sample_count: u32,
    pub multisample_state: wgpu::MultisampleState,
    pub no_multisample_state: wgpu::MultisampleState,
    pub sprite_uniform: wgpu::BindGroupLayout,
    pub vertex_uniform: wgpu::BindGroupLayout,
    pub fragment_uniform: wgpu::BindGroupLayout,
    pub vertex_wgsl: wgpu::ShaderModule,
    pub vertex_glsl: wgpu::ShaderModule,
    pub texture_sampler: wgpu::Sampler,
}

impl WgpuBase {
    pub fn new(
        device: &wgpu::Device,
        sample_count: u32,
    ) -> Self {
        let sprite_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let fragment_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let vertex_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("matrix_bind_group_layout"),
        });

        let vertex_wgsl =
            device.create_shader_module(wgpu::include_wgsl!("../../res/shader/vertex.wgsl"));
        let vertex_glsl = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vertex_glsl"),
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Borrowed(include_str!("../../res/shader/vertex.glsl")),
                stage: naga::ShaderStage::Vertex,
                defines: Default::default(),
            },
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

        let multisample_state = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let no_multisample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        Self {
            sample_count: sample_count,
            multisample_state,
            no_multisample_state,
            sprite_uniform,
            vertex_uniform,
            fragment_uniform,
            vertex_wgsl,
            vertex_glsl,
            texture_sampler,
        }
    }
}

/// Holds default buffers, shaders, sprites and layouts needed by shura.
pub struct GpuDefaults {
    pub sprite: Shader,
    pub rainbow: Shader,
    pub color: Shader,
    pub colored_sprite: Shader,
    pub transparent: Shader,
    pub grey: Shader,
    pub blurr: Shader,

    pub color_no_msaa: Shader,
    pub sprite_no_msaa: Shader,

    /// This field holds both total time and the frame time. Both are stored as f32 in the buffer.
    /// The first f32 is the `total_time` and the second f32 is the `frame_time`. In the shader
    /// the struct also needs 2 additional floats which are empty to match the 16 byte alignment
    /// some devices need.
    pub times: Uniform<[f32; 2]>,
    pub relative_camera: BufferedCamera,
    pub relative_bottom_left_camera: BufferedCamera,
    pub relative_bottom_right_camera: BufferedCamera,
    pub relative_top_left_camera: BufferedCamera,
    pub relative_top_right_camera: BufferedCamera,
    pub world_camera: CameraBuffer,
    pub single_centered_instance: InstanceBuffer,
    pub empty_instance: InstanceBuffer,
    pub target: RenderTarget,
}

impl GpuDefaults {
    pub(crate) fn new(gpu: &Gpu, window_size: Vector<u32>) -> Self {
        let sprite = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/sprite.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Sprite],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let sprite_no_msaa = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/sprite.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Sprite],
            msaa: false,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let rainbow = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/rainbow.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Uniform],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let color = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/color.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Uniform],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });


        let color_no_msaa = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/color.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Uniform],
            msaa: false,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let colored_sprite = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/colored_sprite.glsl"),
            shader_lang: ShaderLang::GLSL,
            shader_fields: &[ShaderField::Sprite, ShaderField::Uniform],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let grey = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/grey.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Sprite],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let blurr = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/blurr.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Sprite],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let transparent = gpu.create_shader(ShaderConfig {
            fragment_source: include_str!("../../res/shader/transparent_sprite.wgsl"),
            shader_lang: ShaderLang::WGSL,
            shader_fields: &[ShaderField::Sprite, ShaderField::Uniform],
            msaa: true,
            blend: BlendState::ALPHA_BLENDING,
            write_mask: ColorWrites::ALL,
        });

        let size = gpu.render_size(1.0);
        let target = gpu.create_render_target(size);
        let times = Uniform::new(gpu, [0.0, 0.0]);
        let single_centered_instance =
            gpu.create_instance_buffer(&[Matrix::new(Default::default())]);
        let empty_instance = gpu.create_instance_buffer(&[]);

        let yx = window_size.y as f32 / window_size.x as f32;
        let xy = window_size.x as f32 / window_size.y as f32;
        let scale = yx.max(xy) / 2.0;
        let fov = if window_size.x > window_size.y {
            Vector::new(scale, RELATIVE_CAMERA_SIZE)
        } else {
            Vector::new(RELATIVE_CAMERA_SIZE, scale)
        };

        let relative_bottom_left_camera =
            BufferedCamera::new(gpu, Camera::new(Isometry::new(fov, 0.0), fov));
        let relative_bottom_right_camera = BufferedCamera::new(
            gpu,
            Camera::new(
                Isometry::new(Vector::new(-fov.x, fov.y), 0.0),
                fov,
            ),
        );
        let relative_top_right_camera =
            BufferedCamera::new(gpu, Camera::new(Isometry::new(-fov, 0.0), fov));
        let relative_top_left_camera = BufferedCamera::new(
            gpu,
            Camera::new(
                Isometry::new(Vector::new(fov.x, -fov.y), 0.0),
                fov,
            ),
        );

        let relative_cam = Camera::new(Default::default(), fov);
        let world_camera = relative_cam.create_buffer(gpu);
        let relative_camera = BufferedCamera::new(gpu, relative_cam);

        Self {
            sprite,
            rainbow,
            color,
            color_no_msaa,
            sprite_no_msaa,
            colored_sprite,
            transparent,
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
            target,
        }
    }

    pub(crate) fn resize(
        &mut self,
        gpu: &Gpu,
        window_size: Vector<u32>,
        screen_config: &ScreenConfig,
    ) {
        self.apply_render_scale(&gpu, screen_config.render_scale());
        let yx = window_size.y as f32 / window_size.x as f32;
        let xy = window_size.x as f32 / window_size.y as f32;
        let scale = yx.max(xy) / 2.0;
        let fov = if window_size.x > window_size.y {
            Vector::new(scale, RELATIVE_CAMERA_SIZE)
        } else {
            Vector::new(RELATIVE_CAMERA_SIZE, scale)
        };
        self.relative_bottom_left_camera
            .write(gpu, Camera::new(Isometry::new(fov, 0.0), fov));
        self.relative_bottom_right_camera.write(
            gpu,
            Camera::new(
                Isometry::new(Vector::new(-fov.x, fov.y), 0.0),
                fov,
            ),
        );
        self.relative_top_right_camera
            .write(gpu, Camera::new(Isometry::new(-fov, 0.0), fov));
        self.relative_top_left_camera.write(
            gpu,
            Camera::new(
                Isometry::new(Vector::new(fov.x, -fov.y), 0.0),
                fov,
            ),
        );
        self.relative_camera
            .write(gpu, Camera::new(Isometry::default(), fov));
    }

    pub(crate) fn buffer(
        &self,
        active_scene_camera: &Camera,
        gpu: &Gpu,
        total_time: f32,
        frame_time: f32,
    ) {
        active_scene_camera.write_buffer(gpu, &self.world_camera);
        self.times.write(&gpu, [total_time, frame_time]);
    }

    pub(crate) fn apply_render_scale(&mut self, gpu: &Gpu, scale: f32) {
        let size = gpu.render_size(scale);
        if *self.target.size() != size {
            self.target = gpu.create_render_target(size);
        }
    }
}
