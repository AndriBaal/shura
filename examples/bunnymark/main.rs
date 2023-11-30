use shura::{log, rand, text::*, *};

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(
        config,
        || {
            NewScene::new(1)
                .component::<Instance2D>("bunny", BufferConfig::EveryFrame)
                .entity::<Bunny>(EntityConfig::DEFAULT)
                .entity::<Resources>(EntityConfig::RESOURCE)
                .system(System::Update(update))
                .system(System::Setup(setup))
                .system(System::Render(render))
        },
    );
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Min(3.0));
    ctx.entities
        .add_with(ctx.world, |handle| Bunny::new(vector2(0.0, 0.0), handle));
    ctx.entities.add(ctx.world, Resources::new(ctx));
}

fn update(ctx: &mut Context) {
    const MODIFY_STEP: usize = 1500;
    const GRAVITY: f32 = -2.5;

    let mut bunnies = ctx.entities.set::<Bunny>();
    let mut resources = ctx.entities.single::<Resources>();

    if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
        let cursor: Vector2<f32> = ctx.cursor.coords;
        for _ in 0..MODIFY_STEP {
            bunnies.add_with(ctx.world, |handle| Bunny::new(cursor, handle));
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

    resources.text.write(
        &ctx.gpu,
        &[text::TextSection {
            color: Color::RED,
            text: format!("FPS: {}\nBunnies: {}", ctx.frame.fps(), bunnies.len()),
            size: 0.05,
            horizontal_alignment: text::TextAlignment::End,
            vertical_alignment: text::TextAlignment::End,
            ..Default::default()
        }],
    );

    if let Some(screenshot) = resources.screenshot.take() {
        log::info!("Saving Screenshot!");
        let bytes = screenshot.sprite().to_bytes(&ctx.gpu);
        save_data("screenshot.png", bytes).unwrap();
    } else if ctx.input.is_pressed(Key::S) {
        resources.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
    }

    let frame = ctx.frame.frame_time();
    let fov = ctx.world_camera2d.fov();
    bunnies.for_each_mut(|bunny| {
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
            linvel.y = rand::gen_range(0.0..15.0);
            translation.y = -fov.y;
        } else if translation.y > fov.y {
            linvel.y = -1.0;
            translation.y = fov.y;
        }
        bunny.linvel = linvel;
        bunny.position.set_translation(translation);
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let resources = ctx.single::<Resources>();
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            ctx.render_all(renderer, "bunny", |renderer, buffer, instances| {
                renderer.render_sprite(
                    instances,
                    buffer,
                    ctx.world_camera2d,
                    ctx.unit_mesh,
                    &resources.bunny_sprite,
                );
            });

            renderer.render_text(
                0..1,
                ctx.centered_instance,
                ctx.relative_top_right_camera,
                &resources.text,
            );
        },
    );

    if let Some(screenshot) = &resources.screenshot {
        encoder.copy_target(encoder.defaults.default_target(), screenshot)
    }
}

#[derive(Entity)]
struct Resources {
    screenshot: Option<SpriteRenderTarget>,
    bunny_sprite: Sprite,
    text: Text,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        let bunny_sprite = ctx
            .gpu
            .create_sprite(SpriteBuilder::bytes(include_bytes_res!(
                "bunnymark/wabbit.png"
            )));
        let font = ctx.gpu.create_font(FontBuilder::bytes(include_bytes_res!(
            "bunnymark/novem.ttf"
        )));
        Resources {
            screenshot: None,
            bunny_sprite,
            text: ctx.gpu.create_text(
                &font,
                &[text::TextSection {
                    color: Color::RED,
                    text: format!(
                        "FPS: {}\nBunnies: {}",
                        ctx.frame.fps(),
                        ctx.entities.set::<Bunny>().len()
                    ),
                    size: 0.05,
                    horizontal_alignment: text::TextAlignment::End,
                    vertical_alignment: text::TextAlignment::End,
                    ..Default::default()
                }],
            ),
        }
    }
}

#[derive(Entity)]
struct Bunny {
    #[shura(component = "bunny")]
    position: PositionComponent2D,
    linvel: Vector2<f32>,
    handle: EntityHandle,
}

impl Bunny {
    pub fn new(translation: Vector2<f32>, handle: EntityHandle) -> Bunny {
        let scaling = rand::gen_range(0.75_f32..2.0);
        let position = PositionComponent2D::new()
            .with_translation(translation)
            .with_rotation(rand::gen_range(-1.0..1.0))
            .with_scaling(scaling * vector2(0.12, 0.18));
        let linvel = vector2(rand::gen_range(-2.5..2.5), rand::gen_range(-7.5..7.5));
        Bunny {
            position,
            linvel,
            handle,
        }
    }
}
