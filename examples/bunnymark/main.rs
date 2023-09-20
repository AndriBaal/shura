use shura::{log, rand, *, text::TextSection};

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
            text: ctx.gpu.create_text(&font, &[TextSection {
                color: Color::BLACK,
                text: "Testg",
                size: 3.0
            }]),
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
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(format!("FPS: {}", ctx.frame.fps()));
                ui.label(format!("Bunnies: {}", ctx.components.len::<Bunny>()));
                if ui.button("Clear Bunnies").clicked() {
                    ctx.components.remove_all::<Bunny>(ctx.world);
                }
            });

        if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
            let cursor = ctx.cursor;
            for _ in 0..MODIFY_STEP {
                ctx.components
                    .add_with::<Bunny>(ctx.world, |handle| Bunny::new(cursor, handle));
            }
        }
        if ctx.input.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let mut bunnies = ctx.components.set::<Bunny>();
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
            let mut resources = ctx.components.single_mut::<Resources>();
            if let Some(screenshot) = resources.screenshot.take() {
                log::info!("Saving Screenshot!");
                screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
            } else if ctx.input.is_pressed(Key::S) {
                resources.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
            }
        }

        let frame = ctx.frame.frame_time();
        let fov = ctx.world_camera.fov();
        ctx.components
            .buffer_for_each_mut::<Self>(ctx.world, &ctx.gpu, |bunny| {
                // let mut linvel = bunny.linvel;
                // let mut translation = bunny.position.translation();

                // linvel.y += GRAVITY * frame;
                // translation += linvel * frame;
                // if translation.x >= fov.x {
                //     linvel.x = -linvel.x;
                //     translation.x = fov.x;
                // } else if translation.x <= -fov.x {
                //     linvel.x = -linvel.x;
                //     translation.x = -fov.x;
                // }

                // if translation.y < -fov.y {
                //     linvel.y = rand::gen_range(0.0..15.0);
                //     translation.y = -fov.y;
                // } else if translation.y > fov.y {
                //     linvel.y = -1.0;
                //     translation.y = fov.y;
                // }
                // bunny.linvel = linvel;
                // bunny.position.set_translation(translation);
            });
    }

    fn render<'a>(renderer: &mut ComponentRenderer<'a>) {
        let resources = renderer.single::<Resources>();
        renderer.render_all::<Bunny>(renderer.world_camera, |r, instances| {
            r.render_sprite(instances.clone(), r.defaults.unit_model(), &resources.bunny_sprite);
            r.render_text(instances, &resources.text);
        });
        if let Some(screenshot) = &resources.screenshot {
            renderer.screenshot = Some(screenshot);
        }
    }
}
