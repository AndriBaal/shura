// use egui::Widget;
use shura::prelude::*;
use std::f32::consts::PI;

const SIZE: Vector2<u32> = vector!(800, 800);

#[shura::main]
fn app(mut config: AppConfig) {
    config.gpu.max_samples = 1;
    App::run(config, || {
        Scene::new()
            .system(System::setup(setup))
            .system(System::resize(resize))
            .system(System::render(render))
            .system(System::update(update).priority(SystemPriority::AFTER))
            .render_group2d("background", RenderGroupUpdate::MANUAL)
            // .render_group::<LightInstance>("shadow", RenderGroupUpdate::EVERY_FRAME)
            .render_group::<LightInstance>("light", RenderGroupUpdate::EVERY_FRAME)
            .entity_single::<LightAssets>()
            .entity::<Light>()
    })
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Min(10.0));
    let _ = ctx
        .window
        .request_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));
    // ctx.window.set_resizable(false);
    // ctx.window
    //     .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
    ctx.entities
        .single_mut()
        .set(ctx.world, LightAssets::new(ctx));

    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(
            LightInstance {
                outer_radius: 10.0,
                inner_radius: 0.2,
                color: Color::BLUE,
                ..Default::default()
            },
            false,
        ),
    );
    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(
            LightInstance {
                translation: vector!(0.0, -2.0),
                outer_radius: 12.0,
                inner_radius: 0.2,
                color: Color::RED,
                ..Default::default()
            },
            true,
        ),
    );
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let assets = ctx.entities.single::<LightAssets>().unwrap();
    encoder.render2d(Some(Color::BLACK), |renderer| {
        ctx.group("background", |buffer| {
            renderer.draw_sprite(
                buffer,
                ctx.world_camera2d,
                ctx.unit_mesh,
                &assets.background_sprite,
            );
        });
    });

    encoder.render2d_to(Some(Color::BLACK), &assets.light_map, |renderer| {
        ctx.group::<LightInstance>("light", |buffer| {
            for instance in buffer.instances() {
                renderer.use_instances_with_range(buffer, instance..instance + 1);
                renderer.use_camera(ctx.world_camera2d);
                renderer.use_shader(&assets.light_shader);
                renderer.use_mesh(ctx.unit_mesh);
                renderer.draw();
            }
        });
    });

    encoder.render2d(None, |renderer| {
        renderer.draw_generic(
            &assets.present_shader,
            ctx.single_instance,
            ctx.unit_mesh,
            &[ctx.unit_camera, &assets.light_map],
        );
    });
}

fn resize(ctx: &mut Context) {
    let mut res = ctx.entities.single_mut::<LightAssets>().unwrap();
    res.light_map.resize(&ctx.gpu, ctx.render_size);
}

fn update(ctx: &mut Context) {
    let cam_aabb = ctx.world_camera2d.aabb();
    let mut assets = ctx.entities.single_mut::<LightAssets>().unwrap();

    for light in ctx.entities.get_mut::<Light>().iter_mut() {
        if light.display {
        }

        if light.follow_player {
            light.inner.translation = ctx.cursor.coords;
        }
    }
    // assets.shadow_mesh = Some(ctx.gpu.create_mesh(&shadow_mesh));
}

#[derive(Entity)]
pub struct LightAssets {
    light_map: SpriteRenderTarget,
    shadow_stencil: DepthBuffer,
    light_shader: Shader,
    present_shader: Shader,
    background_sprite: Sprite,
    #[shura(component = "background")]
    background: PositionComponent2D,
}

impl LightAssets {
    pub fn new(ctx: &Context) -> Self {
        Self {
            present_shader: ctx.gpu.create_shader(ShaderConfig {
                source: ShaderModuleSource::Separate {
                    vertex: &ctx.gpu.shared_assets().vertex_shader_module,
                    fragment: &ctx
                        .gpu
                        .create_shader_module(include_wgsl!("../../static/shader/2d/sprite.wgsl")),
                },
                uniforms: &[UniformField::Camera, UniformField::Sprite],
                blend: BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::Zero,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent::REPLACE,
                },
                ..Default::default()
            }),
            light_shader: ctx.gpu.create_shader(ShaderConfig {
                source: ShaderModuleSource::Single(
                    &ctx.gpu
                        .create_shader_module(include_asset_wgsl!("lighting/light.wgsl")),
                ),
                uniforms: &[UniformField::Camera],
                buffers: &[Vertex2D::LAYOUT, LightInstance::LAYOUT],
                blend: BlendState::ALPHA_BLENDING,
                ..Default::default()
            }),
            background_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "lighting/level.png"
                ))),
            background: PositionComponent2D::new().with_scaling(Vector2::new(10.0, 10.0) * 2.0),
            light_map: ctx.gpu.create_render_target(ctx.render_size),
            shadow_stencil: ctx
                .gpu
                .create_depth_buffer(ctx.render_size, TextureFormat::Stencil8),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightInstance {
    translation: Vector2<f32>,
    rotation: f32,
    color: Color,
    sector: Vector2<f32>,
    inner_radius: f32,
    outer_radius: f32,
    inner_magnification: f32,
    outer_magnification: f32,
    side_falloff_magnification: f32,
}

impl Default for LightInstance {
    fn default() -> Self {
        Self {
            translation: vector!(0.0, 0.0),
            rotation: 0.0,
            outer_radius: 10.0,
            color: Color::WHITE,
            sector: vector![-PI, PI],
            inner_radius: 0.5,
            inner_magnification: 1.1,
            outer_magnification: 1.1,
            side_falloff_magnification: 0.2,
        }
    }
}

impl Instance for LightInstance {
    const ATTRIBUTES: &'static [VertexAttribute] = &vertex_attr_array![
        2 => Float32x2,
        3 => Float32,
        4 => Float32x4,
        5 => Float32x2,
        6 => Float32,
        7 => Float32,
        8 => Float32,
        9 => Float32,
        10 => Float32,
    ];
}

#[derive(Entity)]
struct Light {
    #[shura(component = "light")]
    inner: LightInstance,
    display: bool,
    follow_player: bool,
}

impl Light {
    pub fn new(instance: LightInstance, follow_player: bool) -> Self {
        assert!(instance.inner_radius <= 1.0 && instance.inner_radius >= 0.0);
        // assert_ne!(instance.inner_magnification, 1.0);
        // assert_ne!(instance.outer_magnification, 1.0);
        Self {
            inner: instance,
            display: follow_player,
            follow_player,
        }
    }
}
