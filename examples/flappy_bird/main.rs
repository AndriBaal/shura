use shura::{audio::*, log::info, physics::*, rand::gen_range, *};

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        register!(ctx.components, ctx.groups, [Background, Ground, Pipe, Bird]);
        ctx.scene_states
            .insert(FlappyState::new(&ctx.gpu, ctx.audio));
        ctx.components.add(ctx.world, Background::new(ctx));
        ctx.components.add(ctx.world, Ground::new(&ctx.gpu));
        ctx.components
            .add(ctx.world, Bird::new(&ctx.gpu, ctx.audio));
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
            point_sound: audio.create_sound(load_file!("./audio/point.wav")),
            pipe_sprite: gpu.create_sprite(sprite_file!("./sprites/pipe-green.png")),
            spawn_timer: Pipe::SPAWN_TIME,
            score: 0,
            high_score: 0,
            started: false,
        };
    }
}

#[derive(Component)]
struct Bird {
    #[base]
    body: RigidBodyComponent,
    #[buffer]
    sprite: SpriteSheetIndex,
    model: Model,
    sprite_sheet: SpriteSheet,
    sink: AudioSink,
    hit_sound: Sound,
    wing_sound: Sound,
}

impl Bird {
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.3, 0.21176472);
    pub fn new(gpu: &Gpu, audio: &AudioManager) -> Self {
        Self {
            body: RigidBodyComponent::new(
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
            sprite_sheet: gpu.create_sprite_sheet(sprite_sheet_file!(
                "./sprites/yellowbird.png",
                Vector::new(17, 12),
            )),
            sink: audio.create_sink(),
            hit_sound: audio.create_sound(load_file!("./audio/hit.wav")),
            wing_sound: audio.create_sound(load_file!("./audio/wing.wav")),
            sprite: Default::default(),
        }
    }
}

impl ComponentController for Bird {
    const CONFIG: ComponentConfig = ComponentConfig {
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };

    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components
            .render_single::<Self>(renderer, RenderCamera::World, |r, bird, instance| {
                r.render_sprite_sheet(instance, &bird.model, &bird.sprite_sheet)
            });
    }

    fn update(ctx: &mut Context) {
        let fps = ctx.frame.fps();
        let delta = ctx.frame.frame_time();
        let scene = ctx.scene_states.get_mut::<FlappyState>();
        if !scene.started
            && (ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch))
        {
            scene.started = true;
            for bird in ctx.components.iter_mut::<Bird>() {
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
                ctx.components.add(ctx.world, Pipe::new());
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

        let bird = ctx.components.single_mut::<Self>().unwrap();
        bird.sprite = Vector::new((ctx.frame.total_time() * 7.0 % 3.0) as u32, 0);
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

    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        _other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if collision_type == CollideType::Started {
            ctx.components.remove_all::<Pipe>(ctx.world);
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
    pub fn new(gpu: &Gpu) -> Self {
        let pos = Vector::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y);
        Self {
            model: gpu
                .create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS).vertex_translation(pos)),
            sprite: gpu.create_sprite(sprite_file!("./sprites/base.png")),
            collider: ColliderComponent::new(ColliderBuilder::compound(vec![
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
            ])),
        }
    }
}

impl ComponentController for Ground {
    const CONFIG: ComponentConfig = ComponentConfig {
        update_priority: 2,
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components.render_single::<Self>(
            renderer,
            RenderCamera::World,
            |r, ground, instance| r.render_sprite(instance, &ground.model, &ground.sprite),
        );
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
            .create_sprite(sprite_file!("./sprites/background-night.png"));
        Self {
            model: ctx.gpu.create_model(ModelBuilder::cuboid(GAME_SIZE)),
            sprite,
            base: PositionComponent::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        update_priority: 1,
        render_priority: 1,
        buffer: BufferOperation::Manual,
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components.render_single::<Self>(
            renderer,
            RenderCamera::World,
            |r, background, instance| {
                r.render_sprite(instance, &background.model, &background.sprite)
            },
        );
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
    pub fn new() -> Self {
        let y = gen_range(
            -GAME_SIZE.y + Self::MIN_PIPE_Y + Pipe::HALF_HOLE_SIZE + Ground::HALF_EXTENTS.y * 2.0
                ..GAME_SIZE.y - Self::MIN_PIPE_Y - Pipe::HALF_HOLE_SIZE,
        );
        Self {
            point_awarded: false,
            body: RigidBodyComponent::new(
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
        update_priority: 3,
        ..ComponentConfig::DEFAULT
    };
    fn update(ctx: &mut Context) {
        let state = ctx.scene_states.get_mut::<FlappyState>();
        ctx.components.retain::<Self>(ctx.world, |pipe, world| {
            let x = pipe.body.get(world).translation().x;
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

    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        let scene = ctx.scene_states.get::<FlappyState>();
        ctx.components
            .render_all::<Self>(renderer, RenderCamera::World, |r, instances| {
                r.render_sprite(instances.clone(), &scene.top_pipe_model, &scene.pipe_sprite);
                r.render_sprite(instances, &scene.bottom_pipe_model, &scene.pipe_sprite);
            });
    }
}
