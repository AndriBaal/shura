use shura::{
    log::info,
    physics::{ActiveEvents, ColliderBuilder, LockedAxes, RigidBodyBuilder},
    rand::{thread_rng, Rng},
    *,
};

// Inspired by: https://github.com/bones-ai/rust-flappy-bird-ai

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.set_gravity(Vector::new(0.0, -20.0));
        ctx.set_scene_state(FlappyBird::new(ctx));
        ctx.set_window_size(Vector::new(800, 600));
        ctx.set_window_resizable(false);
        ctx.add_component(Background::new(ctx));
        ctx.add_component(Ground::new(ctx));
        ctx.add_component(Bird::new(ctx));
    }))
}

#[derive(State)]
struct FlappyBird {
    pipe_model: Model,
    pipe_sprite: Sprite,
    last_spawn: f32,
}

impl FlappyBird {
    const SPAWN_TIME: f32 = 3.5;
    const HOLE_SIZE: f32 = 2.2;
    const MIN_PIPE_Y: f32 = 0.5;
    pub fn new(ctx: &Context) -> Self {
        return Self {
            pipe_model: ctx.create_model(ModelBuilder::cuboid(Pipe::SIZE)),
            pipe_sprite: ctx.create_sprite(include_bytes!("./sprites/pipe-green.png")),
            last_spawn: 0.0,
        };
    }
}

impl SceneStateController for FlappyBird {
    fn update(ctx: &mut Context) {
        let mut new_pipe = false;
        let total_time = ctx.total_time();
        let scene = ctx.scene_state_mut::<Self>();

        if total_time > scene.last_spawn + Self::SPAWN_TIME {
            scene.last_spawn = total_time;
            new_pipe = true;
        }

        if new_pipe {
            let under_y = thread_rng().gen_range(
                -GAME_SIZE.y + Self::MIN_PIPE_Y + Ground::SIZE.y * 2.0
                    ..GAME_SIZE.y - Self::MIN_PIPE_Y - Self::HOLE_SIZE,
            );
            let (_, pipe1) = ctx.add_component(Pipe::new(
                Vector::new(GAME_SIZE.x, under_y - Pipe::SIZE.y),
                false,
            ));
            let (_, pipe2) = ctx.add_component(Pipe::new(
                Vector::new(GAME_SIZE.x, under_y + Pipe::SIZE.y + Self::HOLE_SIZE),
                true,
            ));
            info!(
                "Spawning new pipes with ids: [{}, {}]",
                pipe1.id(),
                pipe2.id()
            );
        }
    }
}

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
                RigidBodyBuilder::dynamic()
                    .locked_axes(LockedAxes::TRANSLATION_LOCKED_X)
                    .lock_rotations(),
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
                body.set_linvel(Vector::new(0.0, 6.0), true);
            }
        }
    }

    fn collision(
            ctx: &mut Context,
            self_handle: ComponentHandle,
            other_handle: ComponentHandle,
            self_collider: physics::ColliderHandle,
            other_collider: physics::ColliderHandle,
            collision_type: physics::CollideType,
        ) {
        ctx.remove_components::<Pipe>(Default::default());
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
    const SIZE: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Self::SIZE)),
            sprite: ctx.create_sprite(include_bytes!("./sprites/base.png")),
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed()
                    .translation(Vector::new(0.0, -GAME_SIZE.y + Self::SIZE.y)),
                vec![
                    ColliderBuilder::cuboid(Self::SIZE.x, Self::SIZE.y),
                    ColliderBuilder::segment(
                        Point::new(-GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - Self::SIZE.y),
                        Point::new(GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - Self::SIZE.y),
                    ),
                ],
            )
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

#[derive(Component)]
struct Pipe {
    #[base]
    base: BaseComponent,
    point_awarded: bool,
}

impl Pipe {
    const PIPE_SPEED: f32 = -2.0;
    const SIZE: Vector<f32> = Vector::new(0.65, 4.0);
    pub fn new(translation: Vector<f32>, top_down: bool) -> Self {
        return Self {
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::kinematic_velocity_based()
                    .translation(translation)
                    .linvel(Vector::new(Self::PIPE_SPEED, 0.0))
                    .rotation(if top_down { std::f32::consts::PI } else { 0.0 }),
                vec![ColliderBuilder::cuboid(Self::SIZE.x, Self::SIZE.y)],
            ),
            point_awarded: if top_down {
                true
            } else {
                false
            }
        };
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        let mut to_remove: Vec<ComponentHandle> = vec![];
        let bird_pos = ctx
            .components::<Bird>(Default::default())
            .next()
            .unwrap()
            .base
            .translation();
        for pipe in ctx.path_mut(active) {
            let x = pipe.base.translation().x;
            if x <= -GAME_SIZE.x {
                let handle = pipe.base.handle();
                to_remove.push(handle);
                info!("Removing Pipe with id: {}", handle.id());
            }

            if !pipe.point_awarded && x < bird_pos.x {
                // TODO: Add Point
                pipe.point_awarded = true;
                info!("Earned 1 Point!");
            }
        }

        for handle in to_remove {
            ctx.remove_component(handle);
        }
    }
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_state::<FlappyBird>();
        ctx.render_all(active, encoder, RenderConfig::default(), |r, instances| {
            r.render_sprite(instances, &scene.pipe_model, &scene.pipe_sprite)
        });
    }
}
