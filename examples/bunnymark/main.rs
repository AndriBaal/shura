use shura::{log, rand, *};

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Bunny>(ComponentConfig {
                buffer: BufferOperation::Manual,
                ..ComponentConfig::DEFAULT
            })
            .component::<Resources>(ComponentConfig::RESOURCE)
            .system(System::Update(update))
            .system(System::Setup(setup))
            .system(System::Render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.world_camera.set_scaling(WorldCameraScale::Min(3.0));
    ctx.components
        .add_with(ctx.world, |handle| Bunny::new(vector(0.0, 0.0), handle));
    ctx.components.add(ctx.world, Resources::new(ctx));
}

fn update(ctx: &mut Context) {
    const MODIFY_STEP: usize = 1500;
    const GRAVITY: f32 = -2.5;

    let mut bunnies = ctx.components.set::<Bunny>();
    let mut resources = ctx.components.single::<Resources>();

    gui::Window::new("bunnymark")
        .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
        .resizable(false)
        .collapsible(false)
        .show(ctx.gui, |ui| {
            ui.label(format!("FPS: {}", ctx.frame.fps()));
            ui.label(format!("Bunnies: {}", bunnies.len()));
            if ui.button("Clear Bunnies").clicked() {
                bunnies.remove_all(ctx.world);
            }
        });

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

    if let Some(screenshot) = resources.screenshot.take() {
        log::info!("Saving Screenshot!");
        screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
    } else if ctx.input.is_pressed(Key::S) {
        resources.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
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

fn render(res: &ComponentResources, encoder: &mut RenderEncoder) {
    let resources = res.single::<Resources>();
    encoder.render(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            res.render_all::<Bunny>(renderer, |renderer, buffer, instances| {
                renderer.render_sprite(
                    instances,
                    buffer,
                    renderer.world_camera,
                    renderer.unit_model,
                    &resources.bunny_sprite,
                );
            });
        },
    );

    if let Some(screenshot) = &resources.screenshot {
        encoder.copy_target(encoder.defaults.default_target(), screenshot)
    }
}

#[derive(Component)]
struct Resources {
    screenshot: Option<SpriteRenderTarget>,
    bunny_sprite: Sprite,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        let bunny_sprite = ctx.gpu.create_sprite(sprite_file!("./img/wabbit.png"));
        Resources {
            screenshot: None,
            bunny_sprite,
        }
    }
}

#[derive(Component)]
#[parallel_buffer]
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
