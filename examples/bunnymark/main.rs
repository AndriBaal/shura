use shura::{log::*, rand::*, *};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.components.register::<Bunny>();
            ctx.scene_states.insert(BunnyState::new(ctx));
            ctx.screen_config
                .set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
            ctx.world_camera.set_scaling(WorldCameraScale::Min(3.0));
            ctx.components
                .add_with(|handle| Bunny::new(Vector::new(0.0, 0.0), handle));
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

#[derive(Component)]
struct Bunny {
    #[base]
    base: PositionComponent,
    linvel: Vector<f32>,
    handle: ComponentHandle,
}
impl Bunny {
    pub fn new(translation: Vector<f32>, handle: ComponentHandle) -> Bunny {
        let base = PositionBuilder::new().translation(translation).into();
        let linvel = Vector::new(gen_range(-2.5..2.5), gen_range(-7.5..7.5));
        Bunny {
            base,
            linvel,
            handle,
        }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
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
                    ctx.screen_config.set_render_scale(0.5);
                    ctx.components.remove_all::<Bunny>();
                }
            });

        if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
            let cursor = ctx.input.cursor(&ctx.world_camera);
            for _ in 0..MODIFY_STEP {
                ctx.components.add_with(|handle| Bunny::new(cursor, handle));
            }
        }
        if ctx.input.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let bunnies = ctx.components.set::<Bunny>();
            if bunnies.len() == 1 {
                return;
            }
            for bunny in bunnies.iter().rev() {
                if dead.len() == MODIFY_STEP {
                    break;
                }
                dead.push(bunny.handle);
            }
            for handle in dead {
                ctx.components.remove_boxed(handle);
            }
        }

        let bunny_state = ctx.scene_states.get_mut::<BunnyState>();
        if let Some(screenshot) = bunny_state.screenshot.take() {
            info!("Taking Screenshot!");
            screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
        } else if ctx.input.is_pressed(Key::S) {
            bunny_state.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
        }

        let frame = ctx.frame.frame_time();
        let fov = ctx.world_camera.fov();
        ctx.components.for_each_mut::<Self>(|bunny| {
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
            bunny.base.set_translation(translation);
        });
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_states.get::<BunnyState>();
        ctx.components
            .render_all::<Self>(encoder, RenderConfig::WORLD, |r, instances| {
                r.render_sprite(instances, &scene.bunny_model, &scene.bunny_sprite)
            });
        if let Some(screenshot) = &scene.screenshot {
            encoder.copy_to_target(&ctx.defaults.world_target, &screenshot);
        }
    }
}
