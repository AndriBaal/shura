use shura::{
    physics::{ActiveEvents, ColliderBuilder, RigidBodyBuilder, LockedAxes},
    *,
};

const GAME_SIZE: Vector<f32> = Vector::new(2.8125, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.set_gravity(Vector::new(0.0, -15.0));
        ctx.add_component(Background::new(ctx));
        ctx.add_component(Ground::new(ctx));
        ctx.add_component(Bird::new(ctx));
    }))
}

#[derive(State)]
struct FlappyBird {

}

impl SceneStateController for FlappyBird {}

#[derive(Component)]
struct Bird {
    model: Model,
    sprites: Vec<Sprite>,
    #[base]
    base: BaseComponent,
}

impl Bird {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx.create_sprite(include_bytes!("./sprites/yellowbird-downflap.png"));
        let sprite_size = sprite.size();
        let bird_size = Vector::new(0.3, 0.3 * (sprite_size.y as f32 / sprite_size.x as f32));
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(bird_size)),
            sprites: vec![sprite],
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().locked_axes(LockedAxes::TRANSLATION_LOCKED_X),
                vec![ColliderBuilder::cuboid(bird_size.x, bird_size.y)
                    .active_events(ActiveEvents::COLLISION_EVENTS)],
            ),
        }
    }
}

impl ComponentController for Bird {
    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        for bird in ctx.component_manager.path_mut(&active) {
            let mut body = bird.base_mut().rigid_body_mut().unwrap();
            if ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch)
            {
                body.set_linvel(Vector::new(0.0, 8.0), true);
            }
        }
    }

    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, bird, instance| r.render_sprite(instance, &bird.model, &bird.sprites[0]),
        );
    }
}

#[derive(Component)]
struct Ground {
    model: Model,
    sprite: Sprite,
    #[base]
    base: BaseComponent,
}

impl Ground {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx.create_sprite(include_bytes!("./sprites/base.png"));
        let sprite_size = sprite.size();
        let size = Vector::new(
            GAME_SIZE.x,
            GAME_SIZE.x * (sprite_size.y as f32 / sprite_size.x as f32),
        );
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(size)),
            sprite,
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(Vector::new(0.0, -GAME_SIZE.y + size.y)),
                vec![
                    ColliderBuilder::cuboid(size.x, size.y),
                    ColliderBuilder::segment(
                        Point::new(-GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - size.y),
                        Point::new(GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - size.y),
                    ),
                ],
            ),
        }
    }
}

impl ComponentController for Ground {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, ground, instance| r.render_sprite(instance, &ground.model, &ground.sprite),
        );
    }
}

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
            model: ctx.create_model(ModelBuilder::cuboid(GAME_SIZE)),
            sprite,
            base: BaseComponent::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };
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

struct Pipe {

}
