use rand::{thread_rng, Rng};
use shura::*;

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            let manager = BunnyManager::new(ctx);
            ctx.set_global_state(BunnyRessources::new(ctx));
            ctx.add_component(manager);
        },
    });
}

struct BunnyRessources {
    bunny_model: Model,
    bunny_sprite: Sprite,
}

impl BunnyRessources {
    pub fn new(ctx: &Context) -> Self {
        let bunny_model = ctx.create_model(ModelBuilder::cuboid(Vector::new(0.06, 0.09)));
        let bunny_sprite = ctx.create_sprite(include_bytes!("./img/wabbit.png"));
        BunnyRessources {
            bunny_model,
            bunny_sprite,
        }
    }
}

#[derive(Component)]
struct BunnyManager {
    #[component]
    component: BaseComponent,
    screenshot: Option<RenderTarget>,
}

impl BunnyManager {
    pub fn new(ctx: &mut Context) -> BunnyManager {
        ctx.set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
        ctx.set_window_size(Vector::new(800, 600));
        ctx.set_camera_vertical_fov(6.0);
        ctx.add_component(Bunny::new(&ctx));

        #[cfg(target_os = "android")]
        ctx.set_render_scale(0.667);
        BunnyManager {
            component: Default::default(),
            screenshot: None,
        }
    }
}

impl ComponentController for BunnyManager {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1000,
        buffer: BufferOperation::Never,
        ..DEFAULT_CONFIG
    };
    fn update(path: ComponentPath<Self>, ctx: &mut Context) {
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
            for bunny in bunnies.iter().rev() {
                if dead.len() == MODIFY_STEP {
                    break;
                }
                dead.push(*bunny.base().handle().unwrap());
            }
            for handle in dead {
                ctx.remove_component(&handle);
            }
        }

        let window_size = ctx.window_size();
        for bunny in ctx.component_manager.path_mut(&path).iter() {
            if let Some(screenshot) = bunny.screenshot.take() {
                shura::log::info!("Taking Screenshot!");
                screenshot.sprite().save(&ctx.gpu, "test.png").ok();
            } else if ctx.input.is_pressed(Key::S) {
                bunny.screenshot = Some(ctx.gpu.create_render_target(window_size));
            }
        }
    }

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        for bunny in &ctx.path(&active) {
            if let Some(screenshot) = &bunny.screenshot {
                encoder.copy_target(&config, &screenshot);
            }
        }
    }
}

#[derive(Component)]
struct Bunny {
    #[component]
    component: BaseComponent,
    linvel: Vector<f32>,
}
impl Bunny {
    pub fn new(ctx: &Context) -> Bunny {
        let component = PositionBuilder::new()
            .translation(ctx.cursor_camera(&ctx.world_camera))
            .into();
        let linvel = Vector::new(
            thread_rng().gen_range(-2.5..2.5),
            thread_rng().gen_range(-7.5..7.5),
        );
        Bunny { component, linvel }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let fov = ctx.camera_fov() / 2.0;
        let frame = ctx.frame_time();
        for bunny in &mut ctx.path_mut(&active) {
            const GRAVITY: f32 = -2.5;
            let mut linvel = bunny.linvel;
            let mut translation = bunny.translation();

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
            bunny.component.set_translation(translation);
        }
    }

    fn render<'a>(
        _active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (instances, mut renderer) = encoder.renderer(&config);
        let state = ctx.global_state::<BunnyRessources>().unwrap();
        renderer.render_sprite(&state.bunny_model, &state.bunny_sprite);
        renderer.commit(instances);
    }
}
