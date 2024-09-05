use shura::prelude::*;
pub struct LightPlugin {}

impl Plugin for LightPlugin {
    fn init<S: SceneCreator>(&mut self, scene: S) -> S {
        scene
            .system(System::resize(resize))
            .system(System::setup(load_assets))
            .system(System::render(render))
            .system(System::render(apply_render).priority(SystemPriority::LAST))
            .system(System::update(update).priority(SystemPriority::LAST))
    }
}

fn load_assets(ctx: &mut Context) {
    ctx.assets.load_shader(
        "present_shader",
        ShaderConfig {
            source: ShaderModuleSource::Single(
                &ctx.gpu
                    .create_shader_module(include_wgsl!("../../static/shader/2d/mesh_sprite.wgsl")),
            ),
            uniforms: &[UniformField::Camera, UniformField::Sprite],
            blend: BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::Zero,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::REPLACE,
            },
            vertex_buffers: VertexBuffers::vertex::<SpriteVertex2D>(),
            ..Default::default()
        },
    );

    let bind_group_layout =
        ctx.gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Bind Group Layout"),
            });
    ctx.assets.load_shader(
        "light_shader",
        ShaderConfig {
            source: ShaderModuleSource::Single(
                &ctx.gpu
                    .create_shader_module(include_resource_wgsl!("lighting/light.wgsl")),
            ),
            uniforms: &[
                UniformField::Camera,
                UniformField::Custom(&bind_group_layout),
            ],
            blend: BlendState::ALPHA_BLENDING,
            vertex_buffers: VertexBuffers::instance::<SpriteVertex2D, LightInstance2D>(),
            ..Default::default()
        },
    );
    ctx.assets
        .load_uniform_empty::<Shadow>("shadows", bind_group_layout.into(), 10);
    ctx.assets.load_render_target("light_map", ctx.render_size);
}

fn resize(ctx: &mut Context) {
    ctx.assets
        .render_target_mut("light_map")
        .resize(&ctx.gpu, ctx.render_size);
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d_to(
        Some(Color::new(0.007, 0.007, 0.007, 1.0)),
        &*ctx.assets.render_target("light_map"),
        |renderer| {
            renderer.draw(
                &ctx.assets.shader("light_shader"),
                &ctx.write_instance_components(
                    "light_instances",
                    |light: &LightComponent, data| {
                        data.push(LightInstance2D(Instance2D::new(
                            light.position,
                            vector![light.outer_radius, light.outer_radius],
                            LightData {
                                color: light.color,
                                sector: light.sector,
                                inner_radius: light.inner_radius,
                                inner_magnification: light.inner_magnification,
                                outer_magnification: light.outer_magnification,
                                side_falloff_magnification: light.side_falloff_magnification,
                                shadow_range: light.shadow_range,
                            },
                        )))
                    },
                ),
                &ctx.default_assets.sprite_mesh,
                &[
                    &ctx.default_assets.world_camera2d,
                    &*ctx.assets.uniform::<Shadow>("shadows"),
                ],
            );
        },
    );
}

fn apply_render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(None, |renderer| {
        renderer.draw_mesh(
            &ctx.assets.shader("present_shader"),
            &ctx.default_assets.sprite_mesh,
            &[
                &ctx.default_assets.unit_camera.0,
                ctx.assets.render_target("light_map").sprite(),
            ],
        );
    });
}

fn update(ctx: &mut Context) {
    let mut shadows = vec![];
    ctx.entities
        .components_each_mut::<LightComponent>(|_, light| {
            let light_translation = light.position.translation.vector;
            let light_aabb = AABB::from_center(
                light_translation,
                vector![light.outer_radius, light.outer_radius],
            );
            let start = shadows.len() as u32;
            let mut end = start;
            for mesh in &[] {
                if !mesh.aabb().intersects(&light_aabb) {
                    continue;
                }
                let vertices = mesh.vertices();

                let mut leftmost = 0;
                let mut rightmost = 0;

                for (i, v) in vertices[1..].iter().enumerate() {
                    let ray = v.pos - light_translation;
                    fn det(v1: Vector2<f32>, v2: Vector2<f32>) -> f32 {
                        return v1.x * v2.y - v1.y * v2.x;
                    }
                    if det(ray, vertices[leftmost].pos - light_translation) < 0.0 {
                        leftmost = i;
                    } else if det(ray, vertices[rightmost].pos - light_translation) > 0.0 {
                        rightmost = i;
                    }
                }

                for i in leftmost..rightmost {
                    shadows.push(Shadow {
                        light_center: light_translation,
                        start: vertices[i].pos,
                        end: vertices[(i + 1) % vertices.len()].pos,
                    });
                }
                end += 1;
            }
            light.shadow_range = vector![start, end];
        });
    ctx.assets
        .uniform_mut::<Shadow>("shadows")
        .write(&ctx.gpu, &shadows);
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightData {
    color: Color,
    sector: Vector2<f32>,
    inner_radius: f32,
    inner_magnification: f32,
    outer_magnification: f32,
    side_falloff_magnification: f32,
    shadow_range: Vector2<u32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightInstance2D(Instance2D<LightData>);

impl Instance for LightInstance2D {
    const ATTRIBUTES: &'static [VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Uint32x2,
    ];
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct Shadow {
    light_center: Vector2<f32>,
    start: Vector2<f32>,
    end: Vector2<f32>,
}

pub struct LightOccluder {}

#[derive(Component)]
pub struct LightComponent {
    pub position: Isometry2<f32>,
    pub outer_radius: f32,
    pub color: Color,
    pub sector: Vector2<f32>,
    pub inner_radius: f32,
    pub inner_magnification: f32,
    pub outer_magnification: f32,
    pub side_falloff_magnification: f32,
    pub shadow_range: Vector2<u32>,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            position: Default::default(),
            outer_radius: 1.0,
            color: Color::WHITE,
            sector: vector![0.0, 0.0],
            inner_radius: 0.5,
            inner_magnification: 1.1,
            outer_magnification: 1.1,
            side_falloff_magnification: 0.2,
            shadow_range: vector![0, 0],
        }
    }
}
