use shura::{audio::*, log::info, physics::*, rand::gen_range, *};

const GAME_SIZE: Vector2<f32> = Vector2::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Background>(ComponentConfig {
                buffer: BufferConfig::Manual,
                storage: ComponentStorage::Single,
                ..ComponentConfig::DEFAULT
            })
            .component::<Ground>(ComponentConfig::SINGLE)
            .component::<Pipe>(ComponentConfig::DEFAULT)
            .component::<Bird>(ComponentConfig::SINGLE)
            .component::<FlappyManager>(ComponentConfig::RESOURCE)
            .system(System::Update(update))
            .system(System::Setup(setup))
            .system(System::Render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.components
        .add(ctx.world, FlappyManager::new(&ctx.gpu, ctx.audio));
    ctx.components.add(ctx.world, Background::new(ctx));
    ctx.components.add(ctx.world, Ground::new(&ctx.gpu));
    ctx.components
        .add(ctx.world, Bird::new(&ctx.gpu, ctx.audio));

    ctx.world.set_gravity(Vector2::new(0.0, -15.0));
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Vertical(GAME_SIZE.y));
}

fn update(ctx: &mut Context) {
    let fps = ctx.frame.fps();
    let delta = ctx.frame.frame_time();

    let mut manager = ctx.components.single::<FlappyManager>();
    let mut bird = ctx.components.single::<Bird>();
    let mut pipes = ctx.components.set::<Pipe>();
    pipes.retain(ctx.world, |pipe, world| {
        let x = pipe.body.get(world).translation().x;
        if !pipe.point_awarded && x < 0.0 {
            pipe.point_awarded = true;
            manager.score += 1;
            manager.point_sink.append(manager.point_sound.decode())
        }
        if x <= -GAME_SIZE.x {
            info!("Removing Pipe!");
            return false;
        }
        return true;
    });

    if !manager.started
        && (ctx.input.is_pressed(Key::Space)
            || ctx.input.is_pressed(MouseButton::Left)
            || ctx.input.is_pressed(ScreenTouch))
    {
        manager.started = true;
        bird.body.get_mut(ctx.world).set_gravity_scale(1.0, true);
    }

    if manager.started {
        manager.spawn_timer += delta;
        if manager.score > manager.high_score {
            manager.high_score = manager.score;
        }

        if manager.spawn_timer >= Pipe::SPAWN_TIME {
            manager.spawn_timer = 0.0;
            pipes.add(ctx.world, Pipe::new());
            info!("Spawning new pipe!");
        }
    }

    gui::Window::new("Flappy Bird")
        .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
        .resizable(false)
        .collapsible(false)
        .show(&ctx.gui.clone(), |ui| {
            ui.label(&format!("FPS: {}", fps));
            ui.label(format!("Score: {}", manager.score));
            ui.label(format!("High Score: {}", manager.high_score));
        });

    bird.body
        .set_index((ctx.frame.total_time() * 7.0 % 3.0) as u32);
    if ctx.input.is_pressed(Key::Space)
        || ctx.input.is_pressed(MouseButton::Left)
        || ctx.input.is_pressed(ScreenTouch)
    {
        bird.sink = ctx.audio.create_sink();
        bird.sink.append(bird.wing_sound.decode());
        bird.body
            .get_mut(ctx.world)
            .set_linvel(Vector2::new(0.0, 5.0), true);
    }

    ctx.world.step(ctx.frame).collisions(|event| {
        if event.is::<Pipe, Bird>(ctx.world).is_some()
            || event.is::<Ground, Bird>(ctx.world).is_some()
        {
            if event.started() {
                pipes.remove_all(ctx.world);
                {
                    bird.sink = ctx.audio.create_sink();
                    bird.sink.append(bird.hit_sound.decode());
                    let bird_body = bird.body.get_mut(ctx.world);
                    bird_body.set_linvel(Default::default(), true);
                    bird_body.set_translation(Default::default(), true);
                    bird_body.set_gravity_scale(0.0, true);
                }

                manager.score = 0;
                manager.spawn_timer = 0.0;
                manager.started = false;
            }
        }
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let manager = res.single::<FlappyManager>();
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            res.render_single::<Background>(renderer, |renderer, background, buffer, instance| {
                renderer.render_sprite(
                    instance,
                    buffer,
                    res.world_camera2d,
                    &background.mesh,
                    &background.sprite,
                )
            });

            res.render_single::<Ground>(renderer, |renderer, ground, buffer, instance| {
                renderer.render_sprite(
                    instance,
                    buffer,
                    res.world_camera2d,
                    &ground.mesh,
                    &ground.sprite,
                )
            });
            res.render::<Pipe>(renderer, |renderer, buffer, instances| {
                renderer.render_sprite(
                    instances,
                    buffer,
                    res.world_camera2d,
                    &manager.top_pipe_mesh,
                    &manager.pipe_sprite,
                );
                renderer.render_sprite(
                    instances,
                    buffer,
                    res.world_camera2d,
                    &manager.bottom_pipe_mesh,
                    &manager.pipe_sprite,
                );
            });

            res.render_single::<Bird>(renderer, |renderer, bird, buffer, instance| {
                renderer.render_sprite_sheet(
                    instance,
                    buffer,
                    res.world_camera2d,
                    &bird.mesh,
                    &bird.sprite_sheet,
                )
            });
        },
    );
}

#[derive(Component)]
struct FlappyManager {
    top_pipe_mesh: Mesh2D,
    bottom_pipe_mesh: Mesh2D,
    pipe_sprite: Sprite,
    high_score: u32,
    score: u32,
    spawn_timer: f32,
    started: bool,
    point_sink: AudioSink,
    point_sound: Sound,
}

impl FlappyManager {
    pub fn new(gpu: &Gpu, audio: &AudioManager) -> Self {
        return Self {
            top_pipe_mesh: gpu.create_mesh(
                &MeshBuilder2D::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector2::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation2::new(180.0_f32.to_radians()))
                    .apply(),
            ),
            bottom_pipe_mesh: gpu.create_mesh(
                &MeshBuilder2D::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector2::new(
                        0.0,
                        -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                    ))
                    .apply()
                    .apply(),
            ),
            point_sink: audio.create_sink(),
            point_sound: audio.create_sound(include_bytes_res!("flappy_bird/audio/point.wav")),
            pipe_sprite: gpu.create_sprite(SpriteBuilder::bytes(include_bytes_res!(
                "flappy_bird/sprites/pipe-green.png"
            ))),
            spawn_timer: Pipe::SPAWN_TIME,
            score: 0,
            high_score: 0,
            started: false,
        };
    }
}

#[derive(Component)]
struct Bird {
    #[shura(instance)]
    body: RigidBodyComponent,
    mesh: Mesh2D,
    sprite_sheet: SpriteSheet,
    sink: AudioSink,
    hit_sound: Sound,
    wing_sound: Sound,
}

impl Bird {
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(0.3, 0.21176472);
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

            mesh: gpu.create_mesh(&MeshBuilder2D::cuboid(Self::HALF_EXTENTS)),
            sprite_sheet: gpu.create_sprite_sheet(SpriteSheetBuilder::bytes(
                include_bytes_res!("flappy_bird/sprites/yellowbird.png",),
                Vector2::new(17, 12),
            )),
            sink: audio.create_sink(),
            hit_sound: audio.create_sound(include_bytes_res!("flappy_bird/audio/hit.wav")),
            wing_sound: audio.create_sound(include_bytes_res!("flappy_bird/audio/wing.wav")),
        }
    }
}

#[derive(Component)]
struct Ground {
    mesh: Mesh2D,
    sprite: Sprite,
    #[shura(instance)]
    collider: ColliderComponent,
}

impl Ground {
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(gpu: &Gpu) -> Self {
        let pos = Vector2::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y);
        Self {
            mesh: gpu.create_mesh(
                &MeshBuilder2D::cuboid(Self::HALF_EXTENTS)
                    .vertex_translation(pos)
                    .apply(),
            ),
            sprite: gpu.create_sprite(SpriteBuilder::bytes(include_bytes_res!(
                "flappy_bird/sprites/base.png"
            ))),
            collider: ColliderComponent::new(ColliderBuilder::compound(vec![
                (
                    pos.into(),
                    SharedShape::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y),
                ),
                (
                    Vector2::new(0.0, GAME_SIZE.y).into(),
                    SharedShape::segment(
                        Point2::new(-GAME_SIZE.x, 0.0),
                        Point2::new(GAME_SIZE.x, 0.0),
                    ),
                ),
            ])),
        }
    }
}

#[derive(Component)]
struct Background {
    mesh: Mesh2D,
    sprite: Sprite,
    #[shura(instance)]
    position: PositionInstance2D,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx
            .gpu
            .create_sprite(SpriteBuilder::bytes(include_bytes_res!(
                "flappy_bird/sprites/background-night.png"
            )));
        Self {
            mesh: ctx.gpu.create_mesh(&MeshBuilder2D::cuboid(GAME_SIZE)),
            sprite,
            position: PositionInstance2D::default(),
        }
    }
}

#[derive(Component)]
struct Pipe {
    #[shura(instance)]
    body: RigidBodyComponent,
    point_awarded: bool,
}

impl Pipe {
    const PIPE_SPEED: f32 = -3.0;
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(0.65, 4.0);
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
                    .translation(Vector2::new(GAME_SIZE.x, y))
                    .linvel(Vector2::new(Self::PIPE_SPEED, 0.0)),
                [
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .translation(Vector2::new(
                            0.0,
                            -Pipe::HALF_HOLE_SIZE - Self::HALF_EXTENTS.y,
                        )),
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .translation(Vector2::new(
                            0.0,
                            Pipe::HALF_HOLE_SIZE + Self::HALF_EXTENTS.y,
                        )),
                ],
            ),
        }
    }
}
