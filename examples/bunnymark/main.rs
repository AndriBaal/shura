use shura::prelude::*;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .render_group2d("bunny", RenderGroupConfig::default())
            .entity::<Bunny>(EntityStorage::Multiple, EntityScope::Global)
            .entity::<Assets>(EntityStorage::Single, EntityScope::Scene)
            .system(System::update(update))
            .system(System::setup(setup))
            .system(System::render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Min(3.0));
    ctx.entities
        .multiple::<Bunny>()
        .add(ctx.world, Bunny::new(Default::default()));
    ctx.entities
        .single::<Assets>()
        .set(ctx.world, Assets::new(ctx));
}

fn update(ctx: &mut Context) {
    const MODIFY_STEP: usize = 1500;
    const GRAVITY: f32 = -2.5;

    let mut bunnies = ctx.entities.multiple::<Bunny>();
    let mut assets = ctx.entities.single::<Assets>().get().unwrap();

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
            bunnies.remove(ctx.world, handle);
        }
    }

    assets.text.write(
        &ctx.gpu,
        &[TextSection {
            color: Color::RED,
            text: format!("FPS: {}\nBunnies: {}", ctx.time.fps(), bunnies.len()),
            size: 0.05,
            horizontal_alignment: TextAlignment::End,
            vertical_alignment: TextAlignment::End,
            ..Default::default()
        }],
    );

    if let Some(screenshot) = assets.screenshot.take() {
        log::info!("Saving Screenshot!");
        let bytes = screenshot.sprite().to_bytes(&ctx.gpu);
        save_data("screenshot.png", bytes).unwrap();
    } else if ctx.input.is_pressed(Key::KeyS) {
        assets.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
    }

    let frame = ctx.time.delta();
    let fov = ctx.world_camera2d.fov();
    for bunny in bunnies.iter_mut() {
        let mut linvel = bunny.linvel;
        let mut translation = bunny.position.translation();

        linvel.y += GRAVITY * frame;
        translation += linvel * frame;
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
        bunny.position.set_translation(translation);
    }
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let assets = ctx.entities.single::<Assets>().get().unwrap();
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            ctx.render(renderer, "bunny", |renderer, buffer, instances| {
                renderer.render_sprite(
                    instances,
                    buffer,
                    ctx.world_camera2d,
                    ctx.unit_mesh,
                    &assets.bunny_sprite,
                );
            });

            renderer.render_text_mesh(
                0..1,
                ctx.centered_instance,
                ctx.relative_top_right_camera,
                &assets.text,
            );
        },
    );

    if let Some(screenshot) = &assets.screenshot {
        encoder.copy_target(ctx.target(), screenshot)
    }
}

#[derive(Entity)]
struct Assets {
    screenshot: Option<SpriteRenderTarget>,
    bunny_sprite: Sprite,
    text: TextMesh,
}

impl Assets {
    pub fn new(ctx: &Context) -> Self {
        let bunny_sprite = ctx
            .gpu
            .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                "bunnymark/wabbit.png"
            )));
        let font = ctx.gpu.create_font(FontBuilder::bytes(include_asset_bytes!(
            "bunnymark/novem.ttf"
        )));
        Assets {
            screenshot: None,
            bunny_sprite,
            text: ctx.gpu.create_text_mesh::<&str>(&font, &[]),
        }
    }
}

#[derive(Entity)]
struct Bunny {
    #[shura(handle)]
    handle: &EntityHandle,
    #[shura(component = "bunny")]
    position: PositionComponent2D,
    linvel: Vector2<f32>,
}

impl Bunny {
    pub fn new(translation: Vector2<f32>) -> Bunny {
        let scaling = gen_range(0.75_f32..2.0);
        let position = PositionComponent2D::new()
            .with_translation(translation)
            .with_rotation(gen_range(-1.0..1.0))
            .with_scaling(scaling * vector!(0.12, 0.18));
        let linvel = vector!(gen_range(-2.5..2.5), gen_range(-7.5..7.5));
        Bunny {
            position,
            linvel,
            handle: Default::default(),
        }
    }
}
