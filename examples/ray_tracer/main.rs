use shura::prelude::*;

const SIZE: Vector2<u32> = vector!(800, 800);

#[shura::main]
fn app(mut config: AppConfig) {
    config.gpu.max_samples = 1;
    App::run(config, || {
        Scene::new()
            .system(System::setup(setup))
            .system(System::update(update))
            .system(System::resize(resize))
            .system(System::render(render))
            .render_group2d("background", RenderGroupUpdate::MANUAL)
            .render_group2d("light", RenderGroupUpdate::EVERY_FRAME)
            .render_group2d("test", RenderGroupUpdate::EVERY_FRAME)
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
    ctx.window.set_resizable(false);
    ctx.window
        .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
    ctx.entities
        .single_mut()
        .set(ctx.world, LightAssets::new(ctx));

    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(vector!(0.0, 0.0), Color::BLUE, 5.0, true),
    );
    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(vector!(0.0, -2.0), Color::RED, 6.0, false),
    );

}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let assets = ctx.entities.single::<LightAssets>().get_ref().unwrap();
    encoder.render2d(Some(Color::BLACK), |renderer| {
        ctx.render(renderer, "background", |renderer, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                ctx.world_camera2d,
                ctx.unit_mesh,
                &assets.background_sprite,
            );
        });
    });

    encoder.render2d_to(Some(Color::BLACK), &assets.light_map, |renderer| {
        ctx.render::<Instance2D>(renderer, "light", |renderer, buffer, instances| {
            renderer.use_instances(buffer);
            renderer.use_camera(ctx.world_camera2d);
            renderer.use_shader(&assets.light_shader);
            renderer.use_mesh(&assets.light_mesh);
            renderer.draw(instances);
        });
    });

    encoder.render2d(None, |renderer| {
        renderer.use_mesh(ctx.unit_mesh);
        renderer.use_camera(ctx.unit_camera);
        renderer.use_instances(ctx.centered_instance);
        renderer.use_shader(&assets.present_shader);
        renderer.use_sprite(assets.light_map.sprite(), 1);
        renderer.draw(0..1);

        // ctx.render::<Instance2D>(renderer, "test", |renderer, buffer, instances| {
        //     renderer.use_instances(buffer);
        //     renderer.use_camera(ctx.world_camera2d);
        //     renderer.use_shader(&assets.light_shader);
        //     renderer.use_mesh(&assets.light_mesh);
        //     renderer.draw(instances);
        // });
    });
}

fn resize(ctx: &mut Context) {
    let mut res = ctx.entities.single_mut::<LightAssets>().get_ref().unwrap();
    res.light_map.resize(&ctx.gpu, ctx.window_size);
}

fn update(ctx: &mut Context) {
    for light in ctx.entities.get_mut::<Light>().iter_mut() {
        if light.follow_player {
            light.pos.set_translation(ctx.cursor.coords);
            light.pos2.set_translation(ctx.cursor.coords);
        }
    }
    // let mut res = ctx.entities.single_mut::<LightAssets>().get_ref().unwrap();
    // let bytes = res.light_map.sprite().to_bytes(&ctx.gpu);
    // save_data("screenshot.png", bytes).unwrap();
}
#[derive(Entity)]
pub struct LightAssets {
    light_map: SpriteRenderTarget,
    light_shader: Shader,
    light_mesh: Mesh2D,
    present_shader: Shader,
    background_sprite: Sprite,
    #[shura(component = "background")]
    background: PositionComponent2D,
}

impl LightAssets {
    pub fn new(ctx: &Context) -> Self {
        Self {
            light_mesh: ctx.gpu.create_mesh(&MeshBuilder2D::ball(1.0, 24)),
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
                source: ShaderModuleSource::Separate {
                    vertex: &ctx.gpu.shared_assets().vertex_shader_module,
                    fragment: &ctx
                        .gpu
                        .create_shader_module(include_asset_wgsl!("lighting/light.wgsl")),
                },
                uniforms: &[UniformField::Camera],
                ..Default::default()
            }),
            light_map: ctx.gpu.create_render_target(ctx.window_size),
            background_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "lighting/level.png"
                ))),
            background: PositionComponent2D::new().with_scaling(Vector2::new(10.0, 10.0) * 2.0),
        }
    }
}

#[derive(Entity)]
struct Light {
    #[shura(component = "light")]
    pos: PositionComponent2D,
    #[shura(component = "test")]
    pos2: PositionComponent2D,
    follow_player: bool,
}

impl Light {
    pub fn new(translation: Vector2<f32>, color: Color, radius: f32, follow_player: bool) -> Self {
        Self {
            pos: PositionComponent2D::new()
                .with_translation(translation)
                .with_scaling(vector!(radius, radius))
                .with_color(color),
            pos2: PositionComponent2D::new()
                .with_translation(translation)
                .with_scaling(vector!(radius, radius) / 6.0)
                .with_color(color),
            follow_player,
        }
    }
}
