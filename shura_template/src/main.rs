use shura::prelude::*;

#[shura::app]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity::<Bunny>()
            .system(System::update(update))
            .system(System::setup(setup))
            .system(System::render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Min(3.0));
    ctx.assets.load_font(
        "font",
        FontBuilder::bytes(include_resource_bytes!("bunnymark/novem.ttf")),
    );
    ctx.assets.load_text_mesh::<&str>("text", "font", &[]);
    ctx.assets.load_sprite(
        "bunny_sprite",
        SpriteBuilder::bytes(include_resource_bytes!("bunnymark/wabbit.png")),
    );
    ctx.entities
        .get_mut::<Bunny>()
        .add(ctx.world, Bunny::new(Default::default()));
}

fn update(ctx: &mut Context) {
    const MODIFY_STEP: usize = 1500;
    const GRAVITY: f32 = -2.5;

    let mut bunnies = ctx.entities.get_mut::<Bunny>();

    if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
        let cursor: Vector2<f32> = ctx.cursor.coords;
        for _ in 0..MODIFY_STEP {
            bunnies.add(ctx.world, Bunny::new(cursor));
        }
    }
    if ctx.input.is_held(MouseButton::Right) {
        let mut dead: Vec<EntityHandle> = vec![];
        for bunny in bunnies.iter().rev() {
            if dead.len() == MODIFY_STEP {
                break;
            }
            dead.push(bunny.handle);
        }
        for handle in dead {
            bunnies.remove(ctx.world, &handle);
        }
    }

    ctx.assets.write_text(
        "text",
        "font",
        &[TextSection {
            color: Color::RED,
            text: format!("FPS: {}\nBunnies: {}", ctx.time.fps(), bunnies.len()),
            size: 0.05,
            horizontal_alignment: TextAlignment::End,
            vertical_alignment: TextAlignment::End,
            ..Default::default()
        }],
    );

    let delta = ctx.time.delta();
    let fov = ctx.world_camera2d.fov();

    bunnies.par_iter_mut().for_each(|bunny| {
        let mut linvel = bunny.linvel;
        let mut translation = bunny.position.translation.vector;

        linvel.y += GRAVITY * delta;
        translation += linvel * delta;
        if translation.x >= fov.x {
            linvel.x = -linvel.x;
            translation.x = fov.x;
        } else if translation.x <= -fov.x {
            linvel.x = -linvel.x;
            translation.x = -fov.x;
        }

        if translation.y < -fov.y {
            linvel.y = gen_range(0.0..15.0);
            translation.y = -fov.y;
        } else if translation.y > fov.y {
            linvel.y = -1.0;
            translation.y = fov.y;
        }
        bunny.linvel = linvel;
        bunny.position.translation.vector = translation;
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(Some(Color::new_rgba(220, 220, 220, 255)), |renderer| {
        renderer.draw_sprite(
            &ctx.write_instance_entities("bunny_instances", |bunny: &Bunny, data| {
                data.push(SpriteInstance2D::new(bunny.position, bunny.scaling, ()))
            }),
            &ctx.default_assets.sprite_mesh,
            &ctx.default_assets.world_camera2d,
            &ctx.assets.sprite("bunny_sprite"),
        );

        renderer.draw_text_mesh(
            &ctx.assets.text_mesh("text"),
            &ctx.default_assets.relative_top_right_camera.0,
            &ctx.assets.font("font"),
        );
    });
}

#[derive(Entity)]
struct Bunny {
    #[shura(component)]
    handle: EntityHandle,
    position: Isometry2<f32>,
    scaling: Vector2<f32>,
    linvel: Vector2<f32>,
}

impl Bunny {
    pub fn new(translation: Vector2<f32>) -> Bunny {
        let scaling = gen_range(0.75_f32..2.0) * vector!(0.12, 0.18);
        let rotation = gen_range(-1.0..1.0);
        let linvel = vector!(gen_range(-2.5..2.5), gen_range(-7.5..7.5));
        Bunny {
            position: Isometry2::new(translation, rotation),
            linvel,
            handle: Default::default(),
            scaling,
        }
    }
}
