use shura::{
    audio::{AudioManager, AudioSink, Sound},
    log::info,
    physics::*,
    rand::gen_range,
    *,
};

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        register!(ctx, [Background, Ground, Pipe, Bird]);
        ctx.scene_states
            .insert(FlappyState::new(ctx.gpu, ctx.audio));
        ctx.components
            .add(GroupHandle::DEFAULT_GROUP, Background::new(ctx));
        ctx.components
            .add(GroupHandle::DEFAULT_GROUP, Ground::new(ctx.world, ctx.gpu));
        ctx.components.add(
            GroupHandle::DEFAULT_GROUP,
            Bird::new(ctx.world, ctx.gpu, ctx.audio),
        );
        ctx.world.set_physics_priority(Some(10));
        ctx.world.set_gravity(Vector::new(0.0, -15.0));
        ctx.world_camera
            .set_scaling(WorldCameraScale::Vertical(GAME_SIZE.y));
    }))
}

#[derive(State)]
struct FlappyState {
    top_pipe_model: Model,
    bottom_pipe_model: Model,
    pipe_sprite: Sprite,
    high_score: u32,
    score: u32,
    spawn_timer: f32,
    started: bool,
    point_sink: AudioSink,
    point_sound: Sound,
}

impl FlappyState {
    pub fn new(gpu: &Gpu, audio: &AudioManager) -> Self {
        return Self {
            top_pipe_model: gpu.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation::new(180.0_f32.to_radians())),
            ),
            bottom_pipe_model: gpu.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS).vertex_translation(Vector::new(
                    0.0,
                    -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                )),
            ),
            point_sink: audio.create_sink(),
            point_sound: audio.create_sound(include_bytes!("./audio/point.wav")),
            pipe_sprite: gpu.create_sprite(include_bytes!("./sprites/pipe-green.png")),
            spawn_timer: Pipe::SPAWN_TIME,
            score: 0,
            high_score: 0,
            started: false,
        };
    }
}

impl SceneStateController for FlappyState {
    fn update(ctx: &mut Context) {
        let fps = ctx.frame.fps();
        let delta = ctx.frame.frame_time();
        let scene = ctx.scene_states.get_mut::<Self>();
        if !scene.started
            && (ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch))
        {
            scene.started = true;
            for bird in ctx.components.iter_mut::<Bird>(ComponentFilter::All) {
                bird.body.get_mut(ctx.world).set_gravity_scale(1.0, true);
            }
        }

        if scene.started {
            scene.spawn_timer += delta;
            if scene.score > scene.high_score {
                scene.high_score = scene.score;
            }

            if scene.spawn_timer >= Pipe::SPAWN_TIME {
                scene.spawn_timer = 0.0;
                ctx.components
                    .add(GroupHandle::DEFAULT_GROUP, Pipe::new(ctx.world));
                info!("Spawning new pipe!");
            }
        }

        gui::Window::new("Flappy Bird")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(&format!("FPS: {}", fps));
                ui.label(format!("Score: {}", scene.score));
                ui.label(format!("High Score: {}", scene.high_score));
            });
    }
}

#[derive(Component)]
struct Bird {
    #[base]
    body: RigidBodyComponent,
    model: Model,
    sprite: SpriteSheet,
    sink: AudioSink,
    hit_sound: Sound,
    wing_sound: Sound,
}

impl Bird {
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.3, 0.21176472);
    pub fn new(world: &mut World, gpu: &Gpu, audio: &AudioManager) -> Self {
        Self {
            body: RigidBodyComponent::new(
                world,
                RigidBodyBuilder::dynamic()
                    .locked_axes(LockedAxes::TRANSLATION_LOCKED_X)
                    .lock_rotations()
                    .gravity_scale(0.0),
                [
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .active_events(ActiveEvents::COLLISION_EVENTS)
                        .sensor(true),
                ],
            ),

            model: gpu.create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS)),
            sprite: gpu.create_sprite_sheet(
                include_bytes!("./sprites/yellowbird.png"),
                Vector::new(3, 1),
            ),
            sink: audio.create_sink(),
            hit_sound: audio.create_sound(include_bytes!("./audio/hit.wav")),
            wing_sound: audio.create_sound(include_bytes!("./audio/wing.wav")),
        }
    }
}

impl ComponentController for Bird {
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, bird, instance| {
            let index = (ctx.frame.total_time() * 7.0 % 3.0) as usize;
            r.render_sprite(instance, &bird.model, &bird.sprite[index])
        });
    }

    fn update(ctx: &mut Context) {
        for bird in ctx.components.iter_mut::<Self>(ComponentFilter::Active) {
            if ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch)
            {
                bird.sink = ctx.audio.create_sink();
                bird.sink.append(bird.wing_sound.decode());
                bird.body
                    .get_mut(ctx.world)
                    .set_linvel(Vector::new(0.0, 5.0), true);
            }
        }
    }

    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        _other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if collision_type == CollideType::Started {
            ctx.components.remove_all::<Pipe>(ComponentFilter::All);
            {
                let bird = ctx.components.get_mut::<Self>(self_handle).unwrap();
                bird.sink = ctx.audio.create_sink();
                bird.sink.append(bird.hit_sound.decode());
                let bird_body = bird.body.get_mut(ctx.world);
                bird_body.set_linvel(Default::default(), true);
                bird_body.set_translation(Default::default(), true);
                bird_body.set_gravity_scale(0.0, true);
            }

            let state = ctx.scene_states.get_mut::<FlappyState>();
            state.score = 0;
            state.spawn_timer = 0.0;
            state.started = false;
        }
    }
}

#[derive(Component)]
struct Ground {
    model: Model,
    sprite: Sprite,
    #[base]
    collider: ColliderComponent,
}

impl Ground {
    const HALF_EXTENTS: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(world: &mut World, gpu: &Gpu) -> Self {
        let pos = Vector::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y);
        Self {
            model: gpu
                .create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS).vertex_translation(pos)),
            sprite: gpu.create_sprite(include_bytes!("./sprites/base.png")),
            collider: ColliderComponent::new(
                world,
                ColliderBuilder::compound(vec![
                    (
                        pos.into(),
                        SharedShape::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y),
                    ),
                    (
                        Vector::new(0.0, GAME_SIZE.y).into(),
                        SharedShape::segment(
                            Point::new(-GAME_SIZE.x, 0.0),
                            Point::new(GAME_SIZE.x, 0.0),
                        ),
                    ),
                ]),
            ),
        }
    }
}

impl ComponentController for Ground {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, ground, instance| {
            r.render_sprite(instance, &ground.model, &ground.sprite)
        });
    }
}

#[derive(Component)]
struct Background {
    model: Model,
    sprite: Sprite,
    #[base]
    base: PositionComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx
            .gpu
            .create_sprite(include_bytes!("./sprites/background-night.png"));
        Self {
            model: ctx.gpu.create_model(ModelBuilder::cuboid(GAME_SIZE)),
            sprite,
            base: PositionComponent::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        buffer: BufferOperation::Manual,
        ..DEFAULT_CONFIG
    };
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, background, instance| {
            r.render_sprite(instance, &background.model, &background.sprite)
        });
    }
}

#[derive(Component)]
struct Pipe {
    #[base]
    body: RigidBodyComponent,
    point_awarded: bool,
}

impl Pipe {
    const PIPE_SPEED: f32 = -3.0;
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.65, 4.0);
    const HALF_HOLE_SIZE: f32 = 1.1;
    const MIN_PIPE_Y: f32 = 0.25;
    const SPAWN_TIME: f32 = 3.0;
    pub fn new(world: &mut World) -> Self {
        let y = gen_range(
            -GAME_SIZE.y + Self::MIN_PIPE_Y + Pipe::HALF_HOLE_SIZE + Ground::HALF_EXTENTS.y * 2.0
                ..GAME_SIZE.y - Self::MIN_PIPE_Y - Pipe::HALF_HOLE_SIZE,
        );
        Self {
            point_awarded: false,
            body: RigidBodyComponent::new(
                world,
                RigidBodyBuilder::kinematic_velocity_based()
                    .translation(Vector::new(GAME_SIZE.x, y))
                    .linvel(Vector::new(Self::PIPE_SPEED, 0.0)),
                [
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .translation(Vector::new(
                            0.0,
                            -Pipe::HALF_HOLE_SIZE - Self::HALF_EXTENTS.y,
                        )),
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .translation(Vector::new(
                            0.0,
                            Pipe::HALF_HOLE_SIZE + Self::HALF_EXTENTS.y,
                        )),
                ],
            ),
        }
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(ctx: &mut Context) {
        let state = ctx.scene_states.get_mut::<FlappyState>();
        ctx.components
            .retain::<Self>(ComponentFilter::Active, |pipe| {
                let x = pipe.body.get(ctx.world).translation().x;
                if !pipe.point_awarded && x < 0.0 {
                    pipe.point_awarded = true;
                    state.score += 1;
                    state.point_sink.append(state.point_sound.decode())
                }
                if x <= -GAME_SIZE.x {
                    info!("Removing Pipe!");
                    return false;
                }
                return true;
            });
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_states.get::<FlappyState>();
        encoder.render_all::<Self>(ctx, RenderConfig::default(), |r, instances| {
            r.render_sprite(instances.clone(), &scene.top_pipe_model, &scene.pipe_sprite);
            r.render_sprite(instances, &scene.bottom_pipe_model, &scene.pipe_sprite);
        });
    }
}
