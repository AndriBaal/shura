use shura::{log::*, physics::*, serde::*, *};

#[shura::main]
fn shura_main(config: AppConfig) {
    if let Some(save_game) = std::fs::read("data.binc").ok() {
        App::run(config, move || {
            SerializedScene::new(1, &save_game)
                .deserialize::<Floor>()
                .deserialize::<Player>()
                .deserialize::<PhysicsBox>()
                .component::<Resources>(ComponentConfig::RESOURCE)
                .system(System::Render(render))
                .system(System::Setup(|ctx| {
                    ctx.components.add(ctx.world, Resources::new(ctx));
                }))
                .system(System::Update(update))
                .system(System::End(end))
        })
    } else {
        App::run(config, || {
            NewScene::new(1)
                .component::<Floor>(ComponentConfig::SINGLE)
                .component::<Player>(ComponentConfig::SINGLE)
                .component::<PhysicsBox>(Default::default())
                .component::<Resources>(ComponentConfig::RESOURCE)
                .system(System::Render(render))
                .system(System::Setup(setup))
                .system(System::Update(update))
                .system(System::End(end))
        });
    };
}

fn setup(ctx: &mut Context) {
    const PYRAMID_ELEMENTS: i32 = 8;
    const MINIMAL_SPACING: f32 = 0.1;
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Max(5.0));
    ctx.world.set_gravity(Vector2::new(0.00, -9.81));
    ctx.components.add(ctx.world, Resources::new(ctx));

    for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
        for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
            let b = PhysicsBox::new(Vector2::new(
                x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
            ));
            ctx.components.add(ctx.world, b);
        }
    }

    let player = Player::new();
    ctx.components.add(ctx.world, player);
    let floor = Floor::new();
    ctx.components.add(ctx.world, floor);
}

fn update(ctx: &mut Context) {
    if ctx.input.is_pressed(Key::Z) {
        serialize_scene(ctx);
    }

    if ctx.input.is_pressed(Key::R) {
        if let Some(save_game) = std::fs::read("data.binc").ok() {
            ctx.scenes.add(
                SerializedScene::new(1, &save_game)
                    .deserialize::<Floor>()
                    .deserialize::<Player>()
                    .deserialize::<PhysicsBox>()
                    .component::<Resources>(ComponentConfig::RESOURCE)
                    .system(System::Render(render))
                    .system(System::Setup(|ctx| {
                        ctx.components.add(ctx.world, Resources::new(ctx));
                    }))
                    .system(System::Update(update))
                    .system(System::End(end)),
            );
        }
    }

    let mut boxes = ctx.components.set::<PhysicsBox>();
    let mut player = ctx.components.single::<Player>();

    let scroll = ctx.input.wheel_delta();
    let fov = ctx.world_camera2d.fov();
    if scroll != 0.0 {
        ctx.world_camera2d
            .set_scaling(WorldCameraScaling::Max(fov.x + scroll / 5.0));
    }

    if ctx.input.is_held(MouseButton::Right)
        && ctx
            .world
            .intersection_with_shape(
                &ctx.cursor.into(),
                &Cuboid::new(Vector2::new(
                    PhysicsBox::HALF_BOX_SIZE,
                    PhysicsBox::HALF_BOX_SIZE,
                )),
                Default::default(),
            )
            .is_none()
    {
        let b = PhysicsBox::new(ctx.cursor.coords);
        boxes.add(ctx.world, b);
    }

    let delta = ctx.frame.frame_time();
    let cursor_world: Point2<f32> = (ctx.cursor).into();
    let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
    boxes.for_each_mut(|physics_box| {
        if *physics_box.body.color() == Color::RED {
            physics_box.body.set_color(Color::GREEN);
        }
    });
    let mut component: Option<ComponentHandle> = None;
    ctx.world
        .intersections_with_point(&cursor_world, Default::default(), |component_handle, _| {
            component = Some(component_handle);
            false
        });
    if let Some(handle) = component {
        if let Some(physics_box) = boxes.get_mut(handle) {
            physics_box.body.set_color(Color::RED);
            if remove {
                boxes.remove(ctx.world, handle);
            }
        }
    }

    let body = player.body.get_mut(ctx.world);
    let mut linvel = *body.linvel();

    if ctx.input.is_held(Key::D) {
        linvel.x += 15.0 * delta;
    }

    if ctx.input.is_held(Key::A) {
        linvel.x += -15.0 * delta;
    }

    if ctx.input.is_pressed(Key::W) {
        linvel.y += 15.0;
    }

    if ctx.input.is_pressed(Key::S) {
        linvel.y = -17.0;
    }

    body.set_linvel(linvel, true);

    ctx.world.step(ctx.frame).collisions(|event| {
        if let Some(event) = event.is::<Player, PhysicsBox>(ctx.world) {
            if let Some(b) = boxes.get_mut(event.component2) {
                b.body.set_color(match event.collision_type {
                    CollisionType::Started => Color::BLUE,
                    CollisionType::Stopped => Color::GREEN,
                })
            }
        }
    });

    ctx.world_camera2d
        .set_translation(*player.body.get_mut(ctx.world).translation());
}

fn render(res: &ComponentResources, encoder: &mut RenderEncoder) {
    let resources = res.single::<Resources>();
    encoder.render(Some(Color::BLACK), |renderer| {
        res.render_single::<Player>(renderer, |renderer, _player, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                res.world_camera2d,
                &resources.player_mesh,
                &resources.player_sprite,
            )
        });

        res.render_single::<Floor>(renderer, |renderer, _floor, buffer, instances| {
            renderer.render_color(
                instances,
                buffer,
                res.world_camera2d,
                &resources.floor_mesh,
            )
        });

        res.render_all::<PhysicsBox>(renderer, |renderer, buffer, instance| {
            renderer.render_color(instance, buffer, res.world_camera2d, &resources.box_mesh);
        });
    })
}

fn end(ctx: &mut Context, reason: EndReason) {
    match reason {
        EndReason::EndProgram | EndReason::RemoveScene => serialize_scene(ctx),
        EndReason::Replaced => (),
    }
}

fn serialize_scene(ctx: &mut Context) {
    info!("Serializing scene!");
    let ser = ctx
        .serialize_scene(|s| {
            s.serialize::<Floor>();
            s.serialize::<Player>();
            s.serialize::<PhysicsBox>();
        })
        .unwrap();
    std::fs::write("data.binc", ser).expect("Unable to write file");
}

#[derive(Component)]
struct Resources {
    floor_mesh: Mesh2D,
    box_mesh: Mesh2D,
    player_mesh: Mesh2D,
    player_sprite: Sprite,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        Self {
            player_sprite: ctx.gpu.create_sprite(SpriteBuilder::file("burger.png")),
            player_mesh: ctx.gpu.create_mesh(MeshBuilder2D::from_collider_shape(
                &Player::SHAPE,
                Player::RESOLUTION,
                0.0,
            )),
            floor_mesh: ctx.gpu.create_mesh(MeshBuilder2D::from_collider_shape(
                &Floor::SHAPE,
                Floor::RESOLUTION,
                0.0,
            )),
            box_mesh: ctx.gpu.create_mesh(MeshBuilder2D::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
        }
    }
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct Player {
    #[shura(instance)]
    body: RigidBodyInstance,
}

impl Player {
    const RADIUS: f32 = 0.75;
    const RESOLUTION: u32 = 24;
    const SHAPE: Ball = Ball {
        radius: Self::RADIUS,
    };

    pub fn new() -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::SHAPE))
            .active_events(ActiveEvents::COLLISION_EVENTS);
        Self {
            body: RigidBodyInstance::new(
                RigidBodyBuilder::dynamic().translation(Vector2::new(5.0, 4.0)),
                [collider],
            ),
        }
    }
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct Floor {
    #[shura(instance)]
    collider: ColliderInstance,
}

impl Floor {
    const RESOLUTION: u32 = 12;
    const SHAPE: RoundCuboid = RoundCuboid {
        inner_shape: Cuboid {
            half_extents: Vector2::new(20.0, 0.4),
        },
        border_radius: 0.5,
    };
    pub fn new() -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::SHAPE))
            .translation(Vector2::new(0.0, -1.0));
        Self {
            collider: ColliderInstance::new(collider).with_color(Color::BLUE),
        }
    }
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct PhysicsBox {
    #[shura(instance)]
    body: RigidBodyInstance,
}

impl PhysicsBox {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector2::new(PhysicsBox::HALF_BOX_SIZE, PhysicsBox::HALF_BOX_SIZE),
    };
    pub fn new(instance: Vector2<f32>) -> Self {
        Self {
            body: RigidBodyInstance::new(
                RigidBodyBuilder::dynamic().translation(instance),
                [ColliderBuilder::new(SharedShape::new(
                    PhysicsBox::BOX_SHAPE,
                ))],
            )
            .with_color(Color::GREEN),
        }
    }
}
