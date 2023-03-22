use rand::{thread_rng, Rng};
use shura::*;

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_scene_state(BunnyState::new(ctx));
            ctx.set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
            ctx.set_window_size(Vector::new(800, 600));
            ctx.set_camera_vertical_fov(3.0);
            ctx.add_component(Bunny::new(&ctx));
        },
    });
}

struct BunnyState {
    screenshot: Option<RenderTarget>,
    bunny_model: Model,
    bunny_sprite: Sprite,
}

impl BunnyState {
    pub fn new(ctx: &Context) -> Self {
        let bunny_model = ctx.create_model(ModelBuilder::cuboid(Vector::new(0.06, 0.09)));
        let bunny_sprite = ctx.create_sprite(include_bytes!("./img/wabbit.png"));
        BunnyState {
            screenshot: None,
            bunny_model,
            bunny_sprite,
        }
    }
}

impl SceneState for BunnyState {
    fn update(ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(&format!("FPS: {}", ctx.fps()));
                ui.label(format!(
                    "Bunnies: {}",
                    ctx.components::<Bunny>(GroupFilter::All).len()
                ));
                if ui.button("Clear Bunnies").clicked() {
                    ctx.remove_components::<Bunny>(Default::default());
                }
            });

        if ctx.is_held(MouseButton::Left) || ctx.is_held(ScreenTouch) {
            for _ in 0..MODIFY_STEP {
                ctx.add_component(Bunny::new(&ctx));
            }
        }
        if ctx.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let bunnies = ctx.components::<Bunny>(Default::default());
            if bunnies.len() == 1 {
                return;
            }
            for bunny in bunnies.rev() {
                if dead.len() == MODIFY_STEP {
                    break;
                }
                dead.push(bunny.base().handle().unwrap());
            }
            for handle in dead {
                ctx.remove_component(handle);
            }
        }

        let window_size = ctx.window_size();
        let bunny_state = ctx.scene_state.downcast_mut::<Self>().unwrap();
        if let Some(screenshot) = bunny_state.screenshot.take() {
            shura::log::info!("Taking Screenshot!");
            screenshot.sprite().save(&ctx.gpu, "test.png").ok();
        } else if ctx.input.is_pressed(Key::S) {
            bunny_state.screenshot = Some(ctx.gpu.create_render_target(window_size));
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
            .translation(ctx.cursor_camera(&ctx.world_camera))
            .into();
        let linvel = Vector::new(
            thread_rng().gen_range(-2.5..2.5),
            thread_rng().gen_range(-7.5..7.5),
        );
        Bunny { base, linvel }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        const GRAVITY: f32 = -2.5;
        let frame = ctx.frame_time();
        let fov = ctx.camera_fov();
        for bunny in ctx.path_mut(&active) {
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
                linvel.y = thread_rng().gen_range(0.0..15.0);
                translation.y = -fov.y;
            } else if translation.y > fov.y {
                linvel.y = -1.0;
                translation.y = fov.y;
            }
            bunny.linvel = linvel;
            bunny.base.set_translation(translation);
        }
    }

    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.world_renderer();
        let scene = ctx.scene_state::<BunnyState>().unwrap();
        for (instances, _group) in ctx.path_render(&active) {
            renderer.render_sprite(
                instances,
                instances.all_instances(),
                &scene.bunny_model,
                &scene.bunny_sprite,
            )
        }
        drop(renderer);
        if let Some(screenshot) = &scene.screenshot {
            encoder.copy_to_target(&ctx.defaults, &ctx.defaults.target, &screenshot);
        }
    }
}
