use shura::{log, rand, *};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(|| NewScene {
        id: 1,
        init: |ctx| {
            register!(ctx, [Bunny, Resources]);
            ctx.components
                .set::<Resources>()
                .add(ctx.world, Resources::new(ctx));
            ctx.screen_config
                .set_clear_color(Some(RgbaColor::new(220, 220, 220, 255).into()));
            ctx.world_camera.set_scaling(WorldCameraScale::Min(3.0));
            ctx.components
                .set::<Bunny>()
                .add_with(ctx.world, |handle| Bunny::new(vector(0.0, 0.0), handle));
        },
    });
}

#[derive(Resource)]
struct Resources {
    screenshot: Option<SpriteRenderTarget>,
    bunny_sprite: Sprite,
    text: text::Text,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        // let bunny_model = ctx
        //     .gpu
        //     .create_model(ModelBuilder::cuboid(vector(0.06, 0.09)));
        let bunny_sprite = ctx.gpu.create_sprite(sprite_file!("./img/wabbit.png"));
        let font = ctx.gpu.create_font(include_bytes!("./font/novem.ttf"));
        Resources {
            screenshot: None,
            // bunny_model,
            bunny_sprite,
            text: ctx.gpu.create_text(
                &font,
                &[text::TextSection {
                    color: Color::RED,
                    text: format!(
                        "FPS: {}\nBunnies: {}",
                        ctx.frame.fps(),
                        ctx.components.len::<Bunny>()
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

#[derive(Component)]
struct Bunny {
    #[position]
    position: PositionComponent,
    linvel: Vector<f32>,
    handle: ComponentHandle,
}
impl Bunny {
    pub fn new(translation: Vector<f32>, handle: ComponentHandle) -> Bunny {
        let scale = rand::gen_range(0.75_f32..2.0);
        let position = PositionComponent::new()
            .with_translation(translation)
            .with_rotation(rand::gen_range(-1.0..1.0))
            .with_scale(scale * vector(0.12, 0.18));
        let linvel = vector(rand::gen_range(-2.5..2.5), rand::gen_range(-7.5..7.5));
        Bunny {
            position,
            linvel,
            handle,
        }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        const GRAVITY: f32 = -2.5;
        let mut bunnies = ctx.components.set::<Bunny>();
        let mut res = ctx.components.single_ref::<Resources>();

        if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
            let cursor = ctx.cursor;
            for _ in 0..MODIFY_STEP {
                bunnies.add_with(ctx.world, |handle| Bunny::new(cursor, handle));
            }
        }
        if ctx.input.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            if bunnies.len() != 1 {
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
        }

        {
            res.text.write(
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
            if let Some(screenshot) = res.screenshot.take() {
                log::info!("Saving!");
                screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
            } else if ctx.input.is_pressed(Key::S) {
                res.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
            }
        }

        let frame = ctx.frame.frame_time();
        let fov = ctx.world_camera.fov();
        bunnies.buffer_for_each_mut(ctx.world, &ctx.gpu, |bunny| {
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

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        let resources = components.single::<Resources>();
        components.render_all::<Bunny>(|renderer, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                renderer.unit_model,
                &resources.bunny_sprite,
            );
        });
        let renderer = &mut components.renderer;
        renderer.render_text(
            0..1,
            renderer.single_centered_instance,
            renderer.relative_top_right_camera,
            &resources.text,
        );
        if let Some(screenshot) = &resources.screenshot {
            components.screenshot = Some(screenshot);
        }
    }
}
