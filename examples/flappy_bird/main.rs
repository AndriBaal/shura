use rodio::Sink;
use shura::{
    audio::Sound,
    log::info,
    physics::{
        ActiveEvents, CollideType, ColliderBuilder, ColliderHandle, LockedAxes, RigidBodyBuilder,
    },
    rand::gen_range,
    *,
};

// Inspired by: https://github.com/bones-ai/rust-flappy-bird-ai

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.insert_scene_state(FlappyState::new(ctx));
        ctx.add_component(Background::new(ctx));
        ctx.add_component(Ground::new(ctx));
        ctx.add_component(Bird::new(ctx));
        ctx.set_physics_priority(Some(10));
        ctx.set_camera_scale(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.set_gravity(Vector::new(0.0, -15.0));
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
    point_sink: Sink,
    point_sound: Sound,
}

impl FlappyState {
    pub fn new(ctx: &Context) -> Self {
        return Self {
            top_pipe_model: ctx.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation::new(180.0_f32.to_radians())),
            ),
            bottom_pipe_model: ctx.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS).vertex_translation(Vector::new(
                    0.0,
                    -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                )),
            ),
            point_sink: ctx.create_sink(),
            point_sound: ctx.create_sound(include_bytes!("./audio/point.wav")),
            pipe_sprite: ctx.create_sprite(include_bytes!("./sprites/pipe-green.png")),
            spawn_timer: Pipe::SPAWN_TIME,
            score: 0,
            high_score: 0,
            started: false,
        };
    }

    fn spawn_pipes(&mut self, component_manager: &mut ComponentManager) {
        self.spawn_timer = 0.0;
        let pipe = component_manager.add_component(Pipe::new());
        info!("Spawning new pipes with id: {}]", pipe.id());
    }
}

impl SceneStateController for FlappyState {
    fn update(ctx: &mut Context) {
        let fps = ctx.fps();
        let scene = ctx.scene_states.get_mut::<Self>();
        let delta = ctx.frame_manager.frame_time();
        if !scene.started
            && (ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch))
        {
            scene.started = true;
            for bird in ctx
                .component_manager
                .components_mut::<Bird>(ComponentFilter::All)
            {
                bird.body_mut().set_gravity_scale(1.0, true);
            }
        }

        if scene.started {
            scene.spawn_timer += delta;
            if scene.score > scene.high_score {
                scene.high_score = scene.score;
            }

            if scene.spawn_timer >= Pipe::SPAWN_TIME {
                scene.spawn_pipes(ctx.component_manager);
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
    base: BaseComponent,
    model: Model,
    sprite: SpriteSheet,
    sink: Sink,
    hit_sound: Sound,
    wing_sound: Sound,
}

impl Bird {
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.3, 0.21176472);
    pub fn new(ctx: &Context) -> Self {
        Self {
            base: BaseComponent::new_body(
                RigidBodyBuilder::dynamic()
                    .locked_axes(LockedAxes::TRANSLATION_LOCKED_X)
                    .lock_rotations()
                    .gravity_scale(0.0),
                &[
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y)
                        .active_events(ActiveEvents::COLLISION_EVENTS)
                        .sensor(true),
                ],
            ),

            model: ctx.create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS)),
            sprite: ctx.create_sprite_sheet(
                include_bytes!("./sprites/yellowbird.png"),
                Vector::new(3, 1),
            ),
            sink: ctx.create_sink(),
            hit_sound: ctx.create_sound(include_bytes!("./audio/hit.wav")),
            wing_sound: ctx.create_sound(include_bytes!("./audio/wing.wav")),
        }
    }
}

impl ComponentController for Bird {
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, bird, instance| {
                let index = (ctx.total_time() * 7.0 % 3.0) as usize;
                r.render_sprite(instance, &bird.model, &bird.sprite[index])
            },
        );
    }

    fn update(active: &ActiveComponents<Self>, ctx: &mut Context) {
        for bird in ctx.component_manager.active_mut(active) {
            if ctx.input.is_pressed(Key::Space)
                || ctx.input.is_pressed(MouseButton::Left)
                || ctx.input.is_pressed(ScreenTouch)
            {
                bird.sink = Sink::try_new(&ctx.audio_handle).unwrap();
                bird.sink.append(bird.wing_sound.decode());
                bird.body_mut().set_linvel(Vector::new(0.0, 5.0), true);
            }
        }
    }

    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        _other_handle: ComponentHandle,
        _other_type: ComponentTypeId,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if collision_type == CollideType::Started {
            ctx.remove_components::<Pipe>(ComponentFilter::All);
            {
                let bird = ctx
                    .component_manager
                    .component_mut::<Self>(self_handle)
                    .unwrap();
                bird.sink = Sink::try_new(&ctx.audio_handle).unwrap();
                bird.sink.append(bird.hit_sound.decode());
                let mut bird_body = bird.body_mut();
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
    base: BaseComponent,
}

impl Ground {
    const HALF_EXTENTS: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS)),
            sprite: ctx.create_sprite(include_bytes!("./sprites/base.png")),
            base: BaseComponent::new_body(
                RigidBodyBuilder::fixed()
                    .translation(Vector::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y)),
                &[
                    ColliderBuilder::cuboid(Self::HALF_EXTENTS.x, Self::HALF_EXTENTS.y),
                    ColliderBuilder::segment(
                        Point::new(
                            -GAME_SIZE.x,
                            GAME_SIZE.y + GAME_SIZE.y - Self::HALF_EXTENTS.y,
                        ),
                        Point::new(
                            GAME_SIZE.x,
                            GAME_SIZE.y + GAME_SIZE.y - Self::HALF_EXTENTS.y,
                        ),
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
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
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
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
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
        return Self {
            point_awarded: false,
            base: BaseComponent::new_body(
                RigidBodyBuilder::kinematic_velocity_based()
                    .translation(Vector::new(GAME_SIZE.x, y))
                    .linvel(Vector::new(Self::PIPE_SPEED, 0.0)),
                &[
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
        };
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(active: &ActiveComponents<Self>, ctx: &mut Context) {
        let mut to_remove: Vec<ComponentHandle> = vec![];
        let state = ctx.scene_states.get_mut::<FlappyState>();
        for pipe in ctx.component_manager.active_mut(active) {
            let x = pipe.base.translation().x;
            if !pipe.point_awarded && x < 0.0 {
                pipe.point_awarded = true;
                state.score += 1;
                state.point_sink.append(state.point_sound.decode())
            }
            if x <= -GAME_SIZE.x {
                let handle = pipe.base.handle();
                to_remove.push(handle);
                info!("Removing Pipe with id: {}", handle.id());
            }
        }

        for handle in to_remove {
            ctx.remove_component(handle);
        }
    }

    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_state::<FlappyState>();
        ctx.render_all(active, encoder, RenderConfig::default(), |r, instances| {
            r.render_sprite(instances.clone(), &scene.top_pipe_model, &scene.pipe_sprite);
            r.render_sprite(instances, &scene.bottom_pipe_model, &scene.pipe_sprite);
        });
    }
}
