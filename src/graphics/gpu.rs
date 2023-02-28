#[cfg(feature = "text")]
use crate::text::{CreateFont, CreateText, Font, TextDescriptor};
use crate::{
    Camera, CameraBuffers, Color, Dimension, InstanceBuffer, Instances, Matrix, Model,
    ModelBuilder, Renderer, Shader, ShaderField, ShaderLang, Sprite, SpriteSheet, Uniform,
};
use log::info;
use std::borrow::Cow;

pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 1.0;

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
                    features: wgpu::Features::empty(),
                    label: None,
                    limits,
                },
                None,
            )
            .await
            .unwrap();

        let config = surface
            .get_default_config(&adapter, window_size.width, window_size.height)
            .expect("Surface unsupported by adapter");

        let base = WgpuBase::new(&device);

        surface.configure(&device, &config);
        let adapter_info = adapter.get_info();

        info!("Using GPU: {}", adapter_info.name);
        info!("Using WGPU backend: {:?}", adapter_info.backend);
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

    pub(crate) fn resize(&mut self, size: Dimension<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            })
    }

    pub(crate) fn finish_enocder(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    #[cfg(target_os = "android")]
    pub(crate) fn resume(&mut self, window: &winit::window::Window) {
        self.surface = unsafe { self.instance.create_surface(window) };
        self.surface.configure(&self.device, &self.config);
    }

    #[inline]
    pub fn is_vsync(&self) -> bool {
        self.config.present_mode == wgpu::PresentMode::AutoVsync
    }

    pub fn render_size(&self, scale: f32) -> Dimension<u32> {
        Dimension::new(
            (self.config.width as f32 * scale) as u32,
            (self.config.height as f32 * scale) as u32,
        )
    }

    pub fn render_size_no_scale(&self) -> Dimension<u32> {
        Dimension::new(self.config.width, self.config.height)
    }

    /// Tries to enable or disable vSync. The default is always vSync to be on.
    /// So every device supports vSync but not every device supports no vSync.
    pub fn set_vsync(&mut self, vsync: bool) {
        if vsync {
            self.config.present_mode = wgpu::PresentMode::AutoVsync;
        } else {
            self.config.present_mode = wgpu::PresentMode::AutoNoVsync;
        }
        self.surface.configure(&self.device, &self.config);
    }

    fn create_msaa(&self, size: Dimension<u32>) -> wgpu::TextureView {
        let sample_count = self.base.sample_count;
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: size.into(),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        self.device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_target(&self, size: Dimension<u32>) -> (Sprite, wgpu::TextureView) {
        let format = self.config.format;
        let target_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Target"),
            size: size.into(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let target_bindgroup = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.base.sprite_uniform,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.base.texture_sampler),
                },
            ],
            label: Some("target_bind_group"),
        });

        let sprite = Sprite::from_wgpu(size, target_texture, target_bindgroup, format);
        return (sprite, target_view);
    }

    #[inline]
    pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
        InstanceBuffer::new(self, instances)
    }

    #[inline]
    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(self, builder)
    }

    #[inline]
    pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
        Sprite::new(self, bytes)
    }

    #[inline]
    pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
        Sprite::from_image(self, image)
    }

    #[inline]
    pub fn create_empty_sprite(&self, size: Dimension<u32>) -> Sprite {
        Sprite::empty(self, size)
    }

    #[inline]
    pub fn create_sprite_sheet(
        &self,
        bytes: &[u8],
        sprites: Dimension<u32>,
        sprite_size: Dimension<u32>,
    ) -> SpriteSheet {
        SpriteSheet::new(self, bytes, sprites, sprite_size)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8]) -> Font {
        Font::new_simple(self, bytes)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_text(&mut self, descriptor: TextDescriptor) -> Sprite {
        Sprite::new_text(self, descriptor)
    }

    #[inline]
    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(self, data)
    }

    #[inline]
    pub fn create_shader(
        &self,
        code: &str,
        shader_type: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Shader {
        Shader::new(self, code, shader_type, shader_fields)
    }

    #[inline]
    pub fn create_custom_shader(
        &self,
        shader_lang: ShaderLang,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> Shader {
        Shader::new_custom(self, shader_lang, descriptor)
    }

    #[inline]
    pub fn create_computed_sprite<'caller, F>(
        &self,
        defaults: &GpuDefaults,
        instances: &InstanceBuffer,
        camera: &CameraBuffers,
        texture_size: Dimension<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) -> Sprite
    where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        return Sprite::computed(
            self,
            &defaults,
            instances,
            camera,
            texture_size,
            clear_color,
            compute,
        );
    }
}

/// Base Wgpu objects needed to create any further graphics object.
pub struct WgpuBase {
    pub sample_count: u32,
    pub multisample_state: wgpu::MultisampleState,
    pub sprite_uniform: wgpu::BindGroupLayout,
    pub vertex_uniform: wgpu::BindGroupLayout,
    pub fragment_uniform: wgpu::BindGroupLayout,
    pub vertex_wgsl: wgpu::ShaderModule,
    pub vertex_glsl: wgpu::ShaderModule,
    pub texture_sampler: wgpu::Sampler,
}

impl WgpuBase {
    pub fn new(device: &wgpu::Device) -> Self {
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

        let sample_count = 4;
        // let sample_flags = adapter.get_texture_format_features(config.format).flags;
        // let sample_count = {
        //     if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8) {
        //         8
        //     } else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4) {
        //         4
        //     } else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2) {
        //         2
        //     } else {
        //         1
        //     }
        // };

        let multisample_state = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        Self {
            sample_count: sample_count,
            multisample_state,
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

    /// This field holds both total time and the frame time. Both are stored as f32 in the buffer.
    /// The first f32 is the `total_time` and the second f32 is the `frame_time`. In the shader
    /// the struct also needs 2 additional floats which are empty to match the 16 byte alignment
    /// some devices need.
    pub times: Uniform<[f32; 2]>,
    pub relative_camera: CameraBuffers,
    pub world_camera: CameraBuffers,
    pub single_centered_instance: InstanceBuffer,
    pub present_msaa: wgpu::TextureView,
    pub target_msaa: wgpu::TextureView,
    pub target: Sprite,
    pub target_view: wgpu::TextureView,
    pub layer_msaa: wgpu::TextureView,

    /// Additional layer for postproccessing
    pub layer: Sprite,
    pub layer_view: wgpu::TextureView,
}

impl GpuDefaults {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let sprite = Shader::new(
            gpu,
            include_str!("../../res/shader/sprite.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let rainbow = Shader::new(
            gpu,
            include_str!("../../res/shader/rainbow.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Uniform],
        );

        let color = Shader::new(
            gpu,
            include_str!("../../res/shader/color.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Uniform],
        );

        let colored_sprite = Shader::new(
            gpu,
            include_str!("../../res/shader/colored_sprite.glsl"),
            ShaderLang::GLSL,
            &[ShaderField::Sprite, ShaderField::Uniform],
        );

        let grey = Shader::new(
            gpu,
            include_str!("../../res/shader/grey.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let blurr = Shader::new(
            gpu,
            include_str!("../../res/shader/blurr.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let transparent = Shader::new(
            gpu,
            include_str!("../../res/shader/transparent_sprite.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite, ShaderField::Uniform],
        );

        let size = gpu.render_size(1.0);
        let target_msaa = gpu.create_msaa(size);
        let present_msaa = gpu.create_msaa(size);
        let layer_msaa = gpu.create_msaa(size);
        let (target, target_view) = gpu.create_target(size);
        let (layer, layer_view) = gpu.create_target(size);
        let times = Uniform::new(gpu, [0.0, 0.0]);
        let single_centered_instance = InstanceBuffer::new(gpu, &[Matrix::new(Default::default())]);

        let relative_and_default_camera =
            &Camera::new(Default::default(), 1.0, RELATIVE_CAMERA_SIZE);
        let relative_camera = CameraBuffers::new(gpu, &relative_and_default_camera);
        let world_camera = CameraBuffers::new(gpu, &relative_and_default_camera);

        Self {
            sprite,
            rainbow,
            color,
            colored_sprite,
            transparent,
            grey,
            blurr,
            times,
            single_centered_instance,
            relative_camera,
            world_camera,

            target_msaa,
            present_msaa,
            layer_msaa,
            target,
            target_view,
            layer,
            layer_view,
        }
    }

    pub(crate) fn buffer(
        &mut self,
        active_scene_camera: &Camera,
        gpu: &Gpu,
        total_time: f32,
        frame_time: f32,
    ) {
        self.world_camera.write(&gpu, active_scene_camera);
        self.times.write(&gpu, [total_time, frame_time]);
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, scale: f32) {
        let size = gpu.render_size(scale);
        self.present_msaa = gpu.create_msaa(size);
        self.target_msaa = gpu.create_msaa(size);
        self.layer_msaa = gpu.create_msaa(size);
        (self.target, self.target_view) = gpu.create_target(size);
        (self.layer, self.layer_view) = gpu.create_target(size);
    }

    pub(crate) fn apply_render_scale(&mut self, gpu: &Gpu, scale: f32) {
        let size = gpu.render_size(scale);
        self.target_msaa = gpu.create_msaa(size);
        (self.target, self.target_view) = gpu.create_target(size);
        (self.layer, self.layer_view) = gpu.create_target(size);
    }
}
