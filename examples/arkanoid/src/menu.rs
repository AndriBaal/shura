use crate::game::GameScene;
use shura::{audio::*, text::*, *};

pub struct MenuScene {}
impl SceneController for MenuScene {}

impl MenuScene {
    pub fn new(ctx: &mut Context) -> Self {
        let button = StartButton::new(ctx);
        ctx.set_vsync(true);
        ctx.create_component(None, button);
        return Self {};
    }
}

#[derive(Component)]
pub struct StartButton {
    model: Model,
    text: Sprite,
    start: Sound,
    sink: Sink,
    #[component]
    component: PositionComponent,
}
impl StartButton {
    pub fn new(ctx: &mut Context) -> Self {
        ctx.set_window_title("Arkanoid");
        ctx.set_horizontal_fov(5.0);
        let text = ctx.create_text(TextDescriptor {
            font: None,
            size: Dimension::new(600, 200),
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
        let start = ctx.create_sound(include_bytes!("../res/start.wav"));
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Dimension::new(0.5, 0.167))),
            text,
            sink,
            start,
            component: PositionComponent::new(),
        }
    }
}

impl ComponentController for StartButton {
    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        if ctx.is_pressed(Key::Space)
            || ctx.is_pressed(ScreenTouch)
            || ctx.is_pressed(MouseButton::Left)
        {
            self.sink.append(self.start.decode());
            if !ctx.does_scene_exist("game") {
                ctx.create_scene("game", GameScene::new);
            }
            ctx.set_active_scene("game");
        }
    }

    fn render<'a>(
        &'a self,
        _scene: &DynamicScene,
        renderer: &mut Renderer<'a>,
        instances: Instances,
    ) {
        renderer.render_sprite(&self.model, &self.text);
        renderer.commit(&instances);
    }
}
