#![windows_subsystem = "windows"]

use rand::{thread_rng, Rng};
use shura::*;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    init("bunnymark", |ctx| {
        ctx.set_clear_color(Some(Color::new_rgba(220, 220, 220, 255)));
        ctx.set_window_size(Dimension::new(800, 600));
        ctx.set_vertical_fov(3.0);

        let bunny_model = ctx.create_model(ModelBuilder::cuboid(Dimension::new(0.06, 0.09)));
        let bunny_sprite = ctx.create_sprite(include_bytes!("../img/wabbit.png"));

        ctx.create_component(None, Bunny::new(ctx));

        #[cfg(target_os = "android")]
        ctx.set_render_scale(0.66);

        GameScene {
            bunny_model,
            bunny_sprite,
        }
    });
}

struct GameScene {
    bunny_model: Model,
    bunny_sprite: Sprite,
}

impl SceneController for GameScene {
    fn update(&mut self, ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui(), |ui| {
                ui.label(&format!("FPS: {}", ctx.fps()));
                ui.label(format!("Bunnies: {}", ctx.components::<Bunny>(None).len()));
                if ui.button("Clear Bunnies").clicked() {
                    ctx.remove_components::<Bunny>(None);
                }
            });

        if ctx.is_held(MouseButton::Left) || ctx.is_held(ScreenTouch) {
            for _ in 0..MODIFY_STEP {
                ctx.create_component(None, Bunny::new(ctx));
            }
        }
        if ctx.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let bunnies = ctx.components::<Bunny>(None);
            if bunnies.len() == 1 {
                return;
            }
            for bunny in bunnies.iter().rev() {
                if dead.len() == MODIFY_STEP {
                    break;
                }
                dead.push(*bunny.inner().handle());
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
    component: PositionComponent,
    linvel: Vector<f32>,
}
impl Bunny {
    pub fn new(ctx: &Context) -> Bunny {
        let mut component = PositionComponent::new();
        component.set_translation(*ctx.cursor_world());
        let linvel = Vector::new(
            thread_rng().gen_range(-2.5..2.5),
            thread_rng().gen_range(-7.5..7.5),
        );
        Bunny { component, linvel }
    }
}

impl ComponentController for Bunny {
    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        const GRAVITY: f32 = -2.5;
        let fov = ctx.camera_fov();
        let delta = ctx.delta_time();
        let mut linvel = self.linvel;
        let mut translation = *self.translation();

        linvel.y += GRAVITY * delta;
        translation += linvel * delta;
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
        self.linvel = linvel;
        self.component.set_translation(translation);
    }

    fn render_grouped<'a>(
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        _: ComponentSet<DynamicComponent>,
        instances: Instances,
    ) {
        let scene = scene.downcast_ref::<GameScene>().unwrap();
        renderer.render_sprite(&scene.bunny_model, &scene.bunny_sprite);
        renderer.commit(&instances);
    }

    fn config() -> &'static ComponentConfig {
        static CONFIG: ComponentConfig = ComponentConfig {
            priority: 1,
            render: RenderOperation::Grouped,
            ..ComponentConfig::default()
        };
        return &CONFIG;
    }
}
