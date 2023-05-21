use shura::{rand::gen_range, *};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.components.register::<Bunny>();
            ctx.scene_states.insert(BunnyState::new(ctx));
            ctx.screen_config
                .set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
            ctx.world_camera
                .set_scaling(WorldCameraScale::Min(3.0), ctx.window_size);
            ctx.components
                .add(GroupHandle::DEFAULT_GROUP, Bunny::new(&ctx));
        },
    });
}

#[derive(State)]
struct BunnyState {
    screenshot: Option<RenderTarget>,
    bunny_model: Model,
    bunny_sprite: Sprite,
}

impl BunnyState {
    pub fn new(ctx: &Context) -> Self {
        let bunny_model = ctx
            .gpu
            .create_model(ModelBuilder::cuboid(Vector::new(0.06, 0.09)));
        let bunny_sprite = ctx.gpu.create_sprite(include_bytes!("./img/wabbit.png"));
        BunnyState {
            screenshot: None,
            bunny_model,
            bunny_sprite,
        }
    }
}

impl SceneStateController for BunnyState {
    fn update(ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(&format!("FPS: {}", ctx.frame.fps()));
                ui.label(format!(
                    "Bunnies: {}",
                    ctx.components.len::<Bunny>(ComponentFilter::All)
                ));
                if ui.button("Clear Bunnies").clicked() {
                    ctx.components.remove_all::<Bunny>(ComponentFilter::All);
                }
            });

        if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
            for _ in 0..MODIFY_STEP {
                ctx.components
                    .add(GroupHandle::DEFAULT_GROUP, Bunny::new(&ctx));
            }
        }
        if ctx.input.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let bunnies = ctx.components.set::<Bunny>(Default::default());
            if bunnies.len() == 1 {
                return;
            }
            for bunny in bunnies.iter().rev() {
                if dead.len() == MODIFY_STEP {
                    break;
                }
                dead.push(bunny.base().handle());
            }
            for handle in dead {
                ctx.components.remove_boxed(handle);
            }
        }

        let bunny_state = ctx.scene_states.get_mut::<Self>();
        if let Some(screenshot) = bunny_state.screenshot.take() {
            shura::log::info!("Taking Screenshot!");
            screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
        } else if ctx.input.is_pressed(Key::S) {
            bunny_state.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
        }
    }
}

#[derive(Component)]
struct Bunny {
    #[base]
    base: BaseComponent,
    linvel: Vector<f32>,
}
impl Bunny {
    pub fn new(ctx: &Context) -> Bunny {
        let base = PositionBuilder::new()
            .translation(ctx.input.cursor_camera(ctx.window_size, &ctx.world_camera))
            .into();
        let linvel = Vector::new(gen_range(-2.5..2.5), gen_range(-7.5..7.5));
        Bunny { base, linvel }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn update(ctx: &mut Context) {
        const GRAVITY: f32 = -2.5;
        let frame = ctx.frame.frame_time();
        let fov = ctx.world_camera.fov();
        for bunny in ctx.components.iter_mut::<Self>(ComponentFilter::Active) {
            let mut linvel = bunny.linvel;
            let mut translation = bunny.base.translation();

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
            bunny.set_translation(translation);
        }
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_states.get::<BunnyState>();
        encoder.render_all::<Self>(ctx, RenderConfig::WORLD, |r, instances| {
            r.render_sprite(instances, &scene.bunny_model, &scene.bunny_sprite)
        });
        if let Some(screenshot) = &scene.screenshot {
            encoder.copy_to_target(&ctx.defaults.world_target, &screenshot);
        }
    }
}
