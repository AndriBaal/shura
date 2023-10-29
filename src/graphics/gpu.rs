use wgpu::include_wgsl;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "text")]
use crate::text::{Font, Text, TextSection};
use crate::{
    Camera2D, InstanceBuffer, Instance2D, Isometry2, Model, ModelBuilder, RenderEncoder,
    RenderTarget, Shader, ShaderConfig, ShaderModule, ShaderModuleDescriptor, ShaderModuleSoure,
    Sprite, SpriteBuilder, SpriteRenderTarget, SpriteSheet, SpriteSheetBuilder, SpriteSheetIndex,
    SurfaceRenderTarget, Uniform, UniformField, Vector2, Vertex,  Model2D, InstanceBuffer2D, Instance, ModelBuilder2D,
};
use std::{ops::Deref, sync::Mutex};

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
            max_multisample: 1,
        }
    }
}

/// Holds the connection to the GPU using wgpu. Also has some default buffers, layouts etc.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub(crate) surface: Mutex<wgpu::Surface>,
    pub(crate) config: Mutex<wgpu::SurfaceConfiguration>,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) base: WgpuDefaultResources,
}

impl Gpu {
    pub(crate) async fn new(window: &winit::window::Window, config: GpuConfig) -> Self {
        let window_size = window.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);
        let max_multisample = config.max_multisample;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: config.backends,
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
            .expect("Invalid Graphics Backend!");

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

        let config = if cfg!(target_arch = "wasm32") {
            surface
                .get_default_config(&adapter, window_size.x, window_size.y)
                .expect("Surface unsupported by adapter")
        } else {
            let mut config = surface
                .get_default_config(&adapter, window_size.x, window_size.y)
                .expect("Surface unsupported by adapter");
            config.usage |= wgpu::TextureUsages::COPY_SRC;
            config
        };

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

        let base = WgpuDefaultResources::new(&device, config.format, sample_count);

        surface.configure(&device, &config);

        #[cfg(feature = "log")]
        {
            let adapter_info = adapter.get_info();
            info!("Using GPU: {}", adapter_info.name);
            info!("Using WGPU backend: {:?}", adapter_info.backend);
            info!("Using multisample X{sample_count}");
            info!("Using texture format: {:?}", config.format);
            info!("Using Present mode: {:?}", config.present_mode);
        }

        let gpu = Self {
            instance,
            queue,
            device,
            adapter,
            base,
            format: config.format,
            surface: Mutex::new(surface),
            config: Mutex::new(config),
        };

        return gpu;
    }

    #[cfg(target_os = "android")]
    pub(crate) fn resume(&self, window: &winit::window::Window) {
        let config = self.config.lock().unwrap();
        let mut surface = self.surface.lock().unwrap();
        *surface = unsafe { self.instance.create_surface(window).unwrap() };
        surface.configure(&self.device, &config);
    }

    pub(crate) fn resize(&self, window_size: Vector2<u32>) {
        let mut config = self.config.lock().unwrap();
        let surface = self.surface.lock().unwrap();
        config.width = window_size.x;
        config.height = window_size.y;
        surface.configure(&self.device, &config);
    }

    pub(crate) fn apply_vsync(&self, vsync: bool) {
        let mut config = self.config.lock().unwrap();
        let surface = self.surface.lock().unwrap();
        let new_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        config.present_mode = new_mode;
        surface.configure(&self.device, &config);
    }

    pub fn base(&self) -> &WgpuDefaultResources {
        &self.base
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn sample_count(&self) -> u32 {
        self.base.sample_count
    }

    pub fn render_size(&self) -> Vector2<u32> {
        let config = self.config.lock().unwrap();
        Vector2::new(config.width, config.height)
    }

    pub fn block(&self, handle: wgpu::SubmissionIndex) {
        self.device
            .poll(wgpu::MaintainBase::WaitForSubmissionIndex(handle));
    }

    pub fn submit(&self, encoder: RenderEncoder) -> wgpu::SubmissionIndex {
        self.queue.submit(std::iter::once(encoder.finish()))
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

    pub fn create_instance_buffer<I: Instance>(
        &self,
        instances: &[I],
    ) -> InstanceBuffer<I> {
        InstanceBuffer::new(self, instances)
    }

    pub fn create_model<V: Vertex>(&self, builder: impl ModelBuilder<Vertex = V>) -> Model<V> {
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
    pub fn create_font(&self, data: &'static [u8]) -> Font {
        Font::new(self, data)
    }

    #[cfg(feature = "text")]
    pub fn create_text<S: AsRef<str>>(&self, font: &Font, sections: &[TextSection<S>]) -> Text {
        Text::new(self, font, sections)
    }

    pub fn create_computed_target<'caller, D: Deref<Target = [u8]>>(
        &self,
        defaults: &DefaultResources,
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> SpriteRenderTarget {
        return SpriteRenderTarget::computed(self, defaults, sprite, compute);
    }
}

/// Base Wgpu objects needed to create any further graphics object.
pub struct WgpuDefaultResources {
    pub sample_count: u32,
    pub vertex_shader_module: ShaderModule,
    pub multisample: wgpu::MultisampleState,
    pub sprite_sheet_layout: wgpu::BindGroupLayout,
    pub sprite_layout: wgpu::BindGroupLayout,
    pub camera_layout: wgpu::BindGroupLayout,
    pub single_uniform_layout: wgpu::BindGroupLayout,
}

impl WgpuDefaultResources {
    pub fn new(device: &wgpu::Device, _format: wgpu::TextureFormat, sample_count: u32) -> Self {
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

        let multisample = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let vertex_shader_module =
            device.create_shader_module(include_wgsl!("../../res/shader/vertex.wgsl"));

        Self {
            vertex_shader_module,
            sample_count: sample_count,
            multisample,
            sprite_sheet_layout,
            sprite_layout,
            camera_layout,
            single_uniform_layout,
        }
    }
}

/// Holds default buffers, shaders, sprites and layouts needed by shura.
pub struct DefaultResources {
    pub sprite: Shader,
    pub sprite_sheet: Shader,
    pub color: Shader,
    pub rainbow: Shader,
    pub grey: Shader,
    #[cfg(feature = "text")]
    pub text: Shader,
    pub blurr: Shader,

    pub unit_model: Model2D,

    /// This field holds both total time and the frame time. Both are stored as f32 in the buffer.
    /// The first f32 is the `total_time` and the second f32 is the `frame_time`. In the shader
    /// the struct also needs 2 additional floats which are empty to match the 16 byte alignment
    /// some devices need.
    pub times: Uniform<[f32; 2]>,
    /// Camera2D where the smaller side is always 1.0 and the otherside is scaled to match the window aspect ratio.
    pub relative_camera: Camera2D,
    pub relative_bottom_left_camera: Camera2D,
    pub relative_bottom_right_camera: Camera2D,
    pub relative_top_left_camera: Camera2D,
    pub relative_top_right_camera: Camera2D,
    pub unit_camera: Camera2D,
    pub index: [Uniform<SpriteSheetIndex>; 10],
    pub centered_instance: InstanceBuffer2D,

    pub surface: SurfaceRenderTarget,
    #[cfg(feature = "framebuffer")]
    pub framebuffer: SpriteRenderTarget,
}

impl DefaultResources {
    pub(crate) fn new(gpu: &Gpu, window_size: Vector2<u32>) -> Self {
        let sprite_sheet = gpu.create_shader(ShaderConfig {
            name: Some("sprite_sheet"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu
                    .create_shader_module(include_wgsl!("../../res/shader/sprite_sheet.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            ..Default::default()
        });

        #[cfg(feature = "text")]
        let text = gpu.create_shader(ShaderConfig {
            name: Some("text"),
            uniforms: &[UniformField::Camera, UniformField::SpriteSheet],
            source: ShaderModuleSoure::Single(
                &gpu.create_shader_module(include_wgsl!("../../res/shader/text.wgsl")),
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

        let color = gpu.create_shader(ShaderConfig {
            name: Some("color"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!("../../res/shader/color.wgsl")),
            },
            uniforms: &[UniformField::Camera],
            ..Default::default()
        });

        let sprite = gpu.create_shader(ShaderConfig {
            name: Some("sprite"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!("../../res/shader/sprite.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let rainbow = gpu.create_shader(ShaderConfig {
            name: Some("rainbow"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!("../../res/shader/rainbow.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::SingleUniform],
            ..Default::default()
        });

        let grey = gpu.create_shader(ShaderConfig {
            name: Some("grey"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!("../../res/shader/grey.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let blurr = gpu.create_shader(ShaderConfig {
            name: Some("blurr"),
            source: ShaderModuleSoure::Seperate {
                vertex: &gpu.base.vertex_shader_module,
                fragment: &gpu.create_shader_module(include_wgsl!("../../res/shader/blurr.wgsl")),
            },
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            ..Default::default()
        });

        let size = gpu.render_size();
        let times = Uniform::new(gpu, [0.0, 0.0]);
        let centered_instance = gpu.create_instance_buffer(&[Instance2D::default()]);

        let fov = Self::relative_fov(window_size);

        let relative_bottom_left_camera = Camera2D::new_buffer(gpu, Isometry2::new(fov, 0.0), fov);
        let relative_bottom_right_camera =
            Camera2D::new_buffer(gpu, Isometry2::new(Vector2::new(-fov.x, fov.y), 0.0), fov);
        let relative_top_right_camera = Camera2D::new_buffer(gpu, Isometry2::new(-fov, 0.0), fov);
        let relative_top_left_camera =
            Camera2D::new_buffer(gpu, Isometry2::new(Vector2::new(fov.x, -fov.y), 0.0), fov);
        let relative_camera = Camera2D::new_buffer(gpu, Default::default(), fov);
        let unit_camera = Camera2D::new_buffer(gpu, Default::default(), Vector2::new(0.5, 0.5));

        let unit_model = gpu.create_model(ModelBuilder2D::cuboid(Vector2::new(0.5, 0.5)));

        let surface = SurfaceRenderTarget::new(gpu, size);

        #[cfg(feature = "framebuffer")]
        let framebuffer = SpriteRenderTarget::new(gpu, size);

        let index = (0..10)
            .map(|i| gpu.create_uniform(i))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Self {
            surface,
            sprite_sheet,
            unit_model,
            unit_camera,
            #[cfg(feature = "text")]
            text,
            sprite,
            rainbow,
            grey,
            blurr,
            times,
            centered_instance,
            relative_camera,
            relative_bottom_left_camera,
            relative_bottom_right_camera,
            relative_top_left_camera,
            relative_top_right_camera,
            color,
            index,

            #[cfg(feature = "framebuffer")]
            framebuffer,
        }
    }

    #[cfg(feature = "framebuffer")]
    pub(crate) fn apply_render_scale(&mut self, gpu: &Gpu, scale: f32) {
        let size = gpu.render_size().cast::<f32>() * scale;
        let size = Vector2::new(size.x as u32, size.y as u32);
        if self.framebuffer.size() != size {
            self.framebuffer = gpu.create_render_target(size);
        }
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, window_size: Vector2<u32>) {
        self.surface.resize(gpu, window_size);

        #[cfg(feature = "framebuffer")]
        self.framebuffer.resize(gpu, window_size);

        let fov = Self::relative_fov(window_size);
        self.relative_bottom_left_camera = Camera2D::new_buffer(gpu, Isometry2::new(fov, 0.0), fov);
        self.relative_bottom_right_camera =
            Camera2D::new_buffer(gpu, Isometry2::new(Vector2::new(-fov.x, fov.y), 0.0), fov);
        self.relative_top_right_camera = Camera2D::new_buffer(gpu, Isometry2::new(-fov, 0.0), fov);
        self.relative_top_left_camera =
            Camera2D::new_buffer(gpu, Isometry2::new(Vector2::new(fov.x, -fov.y), 0.0), fov);
        self.relative_camera = Camera2D::new_buffer(gpu, Isometry2::default(), fov);
    }

    pub(crate) fn buffer(&mut self, gpu: &Gpu, total_time: f32, frame_time: f32) {
        self.times.write(&gpu, [total_time, frame_time]);
    }

    pub fn unit_model(&self) -> &Model2D {
        return &self.unit_model;
    }

    pub fn default_target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return &self.framebuffer;
        #[cfg(not(feature = "framebuffer"))]
        return &self.surface;
    }

    fn relative_fov(window_size: Vector2<u32>) -> Vector2<f32> {
        let yx = window_size.y as f32 / window_size.x as f32;
        let xy = window_size.x as f32 / window_size.y as f32;
        let scale = yx.max(xy) / 2.0;
        return if window_size.x > window_size.y {
            Vector2::new(scale, RELATIVE_CAMERA_SIZE)
        } else {
            Vector2::new(RELATIVE_CAMERA_SIZE, scale)
        };
    }
}
