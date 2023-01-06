#[cfg(feature = "text")]
use crate::text::{CreateFontWgpu, Font};
use crate::{
    Camera, Dimension, InstanceBuffer, Matrix, Shader, ShaderField, ShaderLang, Sprite, Uniform,
};
use log::info;
use std::borrow::Cow;

const SAMPLE_COUNT: u32 = 4;
pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 1.0;

/// Holds the connection to the GPU using wgpu. Also has some default buffers, layouts etc.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub(crate) defaults: GpuDefaults,
}

impl Gpu {
    pub(crate) async fn new(window: &winit::window::Window) -> Self {
        let window_size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
            if let Some(monitor) = window.current_monitor() {
                let size = monitor.size();
                if size.width > 2048 || size.height > 2048 {
                    if size.width > size.height {
                        limits.max_texture_dimension_1d = size.width;
                        limits.max_texture_dimension_2d = size.width;
                    } else {
                        limits.max_texture_dimension_1d = size.height;
                        limits.max_texture_dimension_2d = size.height;
                    }
                }
            }
            limits
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

        let texture_format = surface.get_supported_formats(&adapter)[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        let defaults = GpuDefaults::new(&device, &queue, &config);

        surface.configure(&device, &config);
        let adapter_info = adapter.get_info();

        info!("Using GPU: {}", adapter_info.name);
        info!("Using WGPU backend: {:?}", adapter_info.backend);
        info!("Using TextureFormat: {:?}", texture_format);

        let gpu = Self {
            instance,
            queue,
            surface,
            config,
            device,
            adapter,
            defaults
        };

        return gpu;
    }

    pub(crate) fn update_defaults(&mut self, total_time: f32, delta_time: f32) {
        self.defaults.times.write_wgpu(&self.queue, [total_time, delta_time]);
    }

    pub(crate) fn resize(&mut self, size: Dimension<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        let render_size = self.render_size();
        let defaults = &mut self.defaults;
        defaults.present_msaa = Gpu::create_msaa(
            size,
            self.config.format,
            &self.device,
            defaults.sample_count,
        );
        defaults.target_msaa = Gpu::create_msaa(
            render_size,
            self.config.format,
            &self.device,
            defaults.sample_count,
        );
        defaults.layer_msaa = Gpu::create_msaa(
            render_size,
            self.config.format,
            &self.device,
            defaults.sample_count,
        );
        (defaults.target, defaults.target_view) = Gpu::create_target(
            render_size,
            self.config.format,
            &self.device,
            &defaults.texture_sampler,
            &defaults.sprite_uniform,
        );
        (defaults.layer, defaults.layer_view) = Gpu::create_target(
            render_size,
            self.config.format,
            &self.device,
            &defaults.texture_sampler,
            &defaults.sprite_uniform,
        );
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

    #[inline]
    pub fn defaults(&self) -> &GpuDefaults {
        &self.defaults
    }

    // Rendersize with applied render scale
    pub fn render_size(&self) -> Dimension<u32> {
        Dimension::new(
            (self.config.width as f32 * self.defaults.render_scale) as u32,
            (self.config.height as f32 * self.defaults.render_scale) as u32,
        )
    }

    // Rendersize without applied render scale
    pub fn render_size_no_scale(&self) -> Dimension<u32> {
        Dimension::new(self.config.width, self.config.height)
    }

    pub fn render_scale(&self) -> f32 {
        self.defaults.render_scale
    }

    #[inline]
    pub fn set_render_scale(&mut self, scale: f32) {
        self.defaults.render_scale = scale;
        let frame_size = self.render_size();
        let defaults = &mut self.defaults;
        defaults.target_msaa = Gpu::create_msaa(
            frame_size,
            self.config.format,
            &self.device,
            defaults.sample_count,
        );
        (defaults.target, defaults.target_view) = Gpu::create_target(
            frame_size,
            self.config.format,
            &self.device,
            &defaults.texture_sampler,
            &defaults.sprite_uniform,
        );
        (defaults.layer, defaults.layer_view) = Gpu::create_target(
            frame_size,
            self.config.format,
            &self.device,
            &defaults.texture_sampler,
            &defaults.sprite_uniform,
        );
    }

    // Setters

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

    fn create_msaa(
        size: Dimension<u32>,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        sample_count: u32,
    ) -> wgpu::TextureView {
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: size.into(),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_target(
        size: Dimension<u32>,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        sampler: &wgpu::Sampler,
        sprite_uniform: &wgpu::BindGroupLayout,
    ) -> (Sprite, wgpu::TextureView) {
        let target_texture = device.create_texture(&wgpu::TextureDescriptor {
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
        });

        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let target_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: sprite_uniform,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let sprite = Sprite::from_wgpu(size, target_texture, target_bindgroup, format);
        return (sprite, target_view);
    }
}

/// Holds default buffers, shaders, sprites and layouts needed by shura.
pub struct GpuDefaults {
    pub sprite_uniform: wgpu::BindGroupLayout,
    pub vertex_uniform: wgpu::BindGroupLayout,
    pub fragment_uniform: wgpu::BindGroupLayout,
    pub vertex_wgsl: wgpu::ShaderModule,
    pub vertex_glsl: wgpu::ShaderModule,
    pub texture_sampler: wgpu::Sampler,

    pub sprite: Shader,
    pub rainbow: Shader,
    pub color: Shader,
    pub colored_sprite: Shader,
    pub transparent: Shader,
    pub grey: Shader,
    pub blurr: Shader,

    /// This field holds both total time and the frame time. Both are stored as f32 in the buffer.
    /// The first f32 is the `total_time` and the second f32 is the `delta_time`. In the shader
    /// the struct also needs 2 additional floats which are empty to match the 16 byte alignment
    /// some devices need.
    pub times: Uniform<[f32; 2]>,
    pub single_centered_instance: InstanceBuffer,
    #[cfg(feature = "text")]
    pub default_font: Font,

    pub sample_count: u32,
    pub render_scale: f32,
    pub present_msaa: wgpu::TextureView,
    pub target_msaa: wgpu::TextureView,
    pub target: Sprite,
    pub target_view: wgpu::TextureView,
    pub layer_msaa: wgpu::TextureView,

    /// Additional layer for postproccessing
    pub layer: Sprite,
    pub layer_view: wgpu::TextureView,

    /// Relative Camera
    pub(crate) relative_camera: Camera,
}

impl GpuDefaults {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
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

        let vertex_shaders = (&vertex_glsl, &vertex_wgsl);
        let layouts = (&vertex_uniform, &fragment_uniform, &sprite_uniform);
        let format = config.format;

        let sprite = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/sprite.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let rainbow = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/rainbow.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Uniform],
        );

        let color = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/color.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Uniform],
        );

        let colored_sprite = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/colored_sprite.glsl"),
            ShaderLang::GLSL,
            &[ShaderField::Sprite, ShaderField::Uniform],
        );

        let grey = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/grey.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let blurr = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/blurr.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite],
        );

        let transparent = Shader::new_wgpu(
            device,
            format,
            SAMPLE_COUNT,
            vertex_shaders,
            layouts,
            include_str!("../../res/shader/transparent_sprite.wgsl"),
            ShaderLang::WGSL,
            &[ShaderField::Sprite, ShaderField::Uniform],
        );

        let frame_size = Dimension::new(config.width, config.height);
        let target_msaa = Gpu::create_msaa(frame_size, config.format, &device, SAMPLE_COUNT);
        let present_msaa = Gpu::create_msaa(frame_size, config.format, &device, SAMPLE_COUNT);
        let layer_msaa = Gpu::create_msaa(frame_size, config.format, &device, SAMPLE_COUNT);
        let (target, target_view) = Gpu::create_target(
            frame_size,
            config.format,
            &device,
            &texture_sampler,
            &sprite_uniform,
        );
        let (layer, layer_view) = Gpu::create_target(
            frame_size,
            config.format,
            &device,
            &texture_sampler,
            &sprite_uniform,
        );
        let relative_camera = Camera::new_wgpu(
            device,
            queue,
            &vertex_uniform,
            Default::default(),
            1.0,
            RELATIVE_CAMERA_SIZE,
        );
        let times = Uniform::new_wgpu(device, queue, &fragment_uniform, [0.0, 0.0]);
        let single_centered_instance =
            InstanceBuffer::new_wgpu(device, &[Matrix::new(Default::default())]);

        #[cfg(feature = "text")]
        let default_font = Font::new_font_wgpu(
            device,
            config.format,
            include_bytes!("../../res/font/open_sans_bold.ttf"),
        );

        Self {
            sprite_uniform,
            vertex_uniform,
            fragment_uniform,
            vertex_wgsl,
            vertex_glsl,
            texture_sampler,

            sprite,
            rainbow,
            color,
            colored_sprite,
            transparent,
            grey,
            blurr,
            times,
            single_centered_instance,
            #[cfg(feature = "text")]
            default_font,

            sample_count: SAMPLE_COUNT,
            render_scale: 1.0,
            target_msaa,
            present_msaa,
            layer_msaa,
            target,
            target_view,
            layer,
            layer_view,

            relative_camera,
        }
    }
}
