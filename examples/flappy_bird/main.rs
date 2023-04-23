use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Min(5.0));
        ctx.add_component(Background::new(ctx));
    }))
}

struct FlappyBird {}

impl SceneState for FlappyBird {}

#[derive(Component)]
struct Background {
    model: Model,
    sprite: Sprite,
    #[base]
    base: BaseComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx.create_sprite(include_bytes!("./sprites/background-night.png"));
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Vector::new(
                5.0 * (sprite.size().x as f32 / sprite.size().y as f32),
                5.0,
            ))),
            sprite,
            base: BaseComponent::default(),
        }
    }
}

impl ComponentController for Background {
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, background, instance| {
                r.render_sprite(instance, &background.model, &background.sprite)
            },
        );
    }
}
