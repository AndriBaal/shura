use rand::{thread_rng, Rng};
use shura::*;

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            let manager = BunnyManager::new(ctx);
            ctx.set_global_state(BunnyRessources::new(ctx));
            ctx.create_component(manager);
        },
    });
}

struct BunnyRessources {
    bunny_model: Model,
    bunny_sprite: Sprite,
}

impl BunnyRessources {
    pub fn new(ctx: &Context) -> Self {
        let bunny_model = ctx.create_model(ModelBuilder::cuboid(Dimension::new(0.06, 0.09)));
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
}

impl BunnyManager {
    pub fn new(ctx: &mut Context) -> BunnyManager {
        ctx.set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
        ctx.set_window_size(Dimension::new(800, 600));
        ctx.set_camera_vertical_fov(6.0);
        ctx.create_component(Bunny::new(&ctx));

        #[cfg(target_os = "android")]
        ctx.set_render_scale(0.667);
        BunnyManager {
            component: Default::default(),
        }
    }
}

impl ComponentController for BunnyManager {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        render: RenderOperation::Never,
        ..DEFAULT_CONFIG
    };
    fn update(_path: ComponentPath<Self>, ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui(), |ui| {
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
                ctx.create_component(Bunny::new(&ctx));
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
        let mut component = BaseComponent::default();
        component.set_translation(*ctx.cursor_world());
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
            if translation.x >= fov.width {
                linvel.x = -linvel.x;
                translation.x = fov.width;
            } else if translation.x <= -fov.width {
                linvel.x = -linvel.x;
                translation.x = -fov.width;
            }

            if translation.y < -fov.height {
                linvel.y = thread_rng().gen_range(0.0..15.0);
                translation.y = -fov.height;
            } else if translation.y > fov.height {
                linvel.y = -1.0;
                translation.y = fov.height;
            }
            bunny.linvel = linvel;
            bunny.component.set_translation(translation);
        }
    }

    fn render<'a>(
        _active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        instances: Instances,
    ) {
        let state = ctx.global_state::<BunnyRessources>().unwrap();
        renderer.render_sprite(&state.bunny_model, &state.bunny_sprite);
        renderer.commit(instances);
    }
}
