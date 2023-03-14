use shura::{audio::*, text::*, *};

pub const MENU_SCENE_ID: u32 = 1;
pub const GAME_SCENE_ID: u32 = 2;

fn main() {
    Shura::init(NewScene::new(MENU_SCENE_ID, |ctx| {
        let button = StartButton::new(ctx);
        ctx.set_vsync(true);
        ctx.create_component(button);
    }));
}

#[derive(Component)]
pub struct StartButton {
    model: Model,
    text: Sprite,
    start: Sound,
    sink: Sink,
    #[component]
    component: BaseComponent,
}
impl StartButton {
    pub fn new(ctx: &mut Context) -> Self {
        ctx.set_window_title("Arkanoid");
        ctx.set_camera_horizontal_fov(5.0);
        ctx.set_physics_priority(None);
        let text = ctx.create_text(TextDescriptor {
            font: ctx.create_font(include_bytes!("")),
            size: Vector::new(600, 200),
            clear_color: Some(Color::new_rgba(0, 0, 0, 255)),
            sections: vec![TextSection {
                position: Vector::new(0.0, 0.0),
                text: vec![Text::new("Press to start!")
                    .with_scale(96.0)
                    .with_color(Color::new_rgba(255, 255, 255, 255))],
                ..TextSection::default()
            }],
        });
        let sink = ctx.create_sink();
        let start = ctx.create_sound(include_bytes!("./audio/start.wav"));
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.167))),
            text,
            sink,
            start,
            component: BaseComponent::default(),
        }
    }
}

impl ComponentController for StartButton {
    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        if ctx.is_pressed(Key::Space)
            || ctx.is_pressed(ScreenTouch)
            || ctx.is_pressed(MouseButton::Left)
        {
            for button in ctx.path_mut(&active).iter() {
                button.sink.append(button.start.decode());
                // if !ctx.does_scene_exist("game") {
                //     ctx.create_scene("game", GameScene::new);
                // }
                // ctx.set_active_scene("game");
            }
        }
    }

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        _all_instances: Instances,
    ) {
        for (instance, button) in ctx.path_render(&active).iter() {
            renderer.render_sprite(&button.model, &button.text);
            renderer.commit(instance);
        }
    }
}
