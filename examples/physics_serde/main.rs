use shura::{physics::*, prelude::*};

fn deserialized_scene(data: Vec<u8>) -> SerializedScene {
    SerializedScene::new(1, &data)
        .render_group2d("player", RenderGroupConfig::default())
        .render_group2d("box", RenderGroupConfig::default())
        .render_group2d(
            "floor",
            RenderGroupConfig {
                call: BufferCall::Manual,
                ..Default::default()
            },
        )
        .deserialize_single_entity::<Floor>(Default::default())
        .deserialize_single_entity::<Player>(Default::default())
        .deserialize_entities::<PhysicsBox>(Default::default())
        .single_entity::<Resources>(Default::default())
        .system(System::Render(render))
        .system(System::Setup(|ctx| {
            ctx.entities.single().set(ctx.world, Resources::new(ctx));
        }))
        .system(System::Update(update))
        .system(System::End(end))
}

#[shura::main]
fn shura_main(config: AppConfig) {
    if let Ok(save_game) = load_data_bytes("data.binc") {
        App::run(config, move || deserialized_scene(save_game))
    } else {
        App::run(config, || {
            Scene::new()
                .render_group2d("player", RenderGroupConfig::default())
                .render_group2d("box", RenderGroupConfig::default())
                .render_group2d(
                    "floor",
                    RenderGroupConfig {
                        call: BufferCall::Manual,
                        ..Default::default()
                    },
                )
                .single_entity::<Floor>(Default::default())
                .single_entity::<Player>(Default::default())
                .entities::<PhysicsBox>(Default::default())
                .single_entity::<Resources>(Default::default())
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
    ctx.entities.single().set(ctx.world, Resources::new(ctx));

    for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
        for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
            let b = PhysicsBox::new(Vector2::new(
                x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
            ));
            ctx.entities.multiple().add(ctx.world, b);
        }
    }

    let player = Player::new();
    ctx.entities.single().set(ctx.world, player);
    let floor = Floor::new();
    ctx.entities.single().set(ctx.world, floor);
}

fn update(ctx: &mut Context) {
    if ctx.input.is_pressed(Key::Z) {
        serialize_scene(ctx);
    }

    if ctx.input.is_pressed(Key::R) {
        if let Ok(save_game) = load_data_bytes("data.binc") {
            ctx.scenes.add(deserialized_scene(save_game));
        }
    }

    let mut boxes = ctx.entities.multiple::<PhysicsBox>();
    let mut player = ctx.entities.single::<Player>().get_mut().unwrap();

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

    let delta = ctx.time.delta_time();
    let cursor_world: Point2<f32> = ctx.cursor;
    let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
    for physics_box in boxes.iter_mut() {
        if *physics_box.body.color() == Color::RED {
            physics_box.body.set_color(Color::GREEN);
        }
    }
    let mut entity: Option<EntityHandle> = None;
    ctx.world
        .intersections_with_point(&cursor_world, Default::default(), |entity_handle, _| {
            entity = Some(entity_handle);
            false
        });
    if let Some(handle) = entity {
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

    ctx.world.step(ctx.time).collisions(|event| {
        if let Some(event) = event.is::<Player, PhysicsBox>(ctx.world) {
            if let Some(b) = boxes.get_mut(event.entity2) {
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

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let resources = ctx.single::<Resources>().get().unwrap();
    encoder.render2d(Some(Color::BLACK), |renderer| {
        ctx.render(renderer, "player", |renderer, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                ctx.world_camera2d,
                &resources.player_mesh,
                &resources.player_sprite,
            )
        });

        ctx.render(renderer, "floor", |renderer, buffer, instances| {
            renderer.render_color(instances, buffer, ctx.world_camera2d, &resources.floor_mesh)
        });

        ctx.render(renderer, "box", |renderer, buffer, instance| {
            renderer.render_color(instance, buffer, ctx.world_camera2d, &resources.box_mesh);
        });
    })
}

fn end(ctx: &mut Context, reason: EndReason) {
    match reason {
        EndReason::End | EndReason::Removed => serialize_scene(ctx),
        EndReason::Replaced => (),
    }
}

fn serialize_scene(ctx: &mut Context) {
    info!("Serializing scene!");
    let ser = ctx
        .serialize_scene(|serializer| {
            serializer
                .serialize_single_entity::<Floor>()
                .serialize_single_entity::<Player>()
                .serialize_entities::<PhysicsBox>()
        })
        .unwrap();
    save_data("data.binc", ser).unwrap();
}

#[derive(Entity)]
struct Resources {
    floor_mesh: Mesh2D,
    box_mesh: Mesh2D,
    player_mesh: Mesh2D,
    player_sprite: Sprite,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        Self {
            player_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_bytes_res!(
                    "physics/burger.png"
                ))),
            player_mesh: ctx.gpu.create_mesh(&MeshBuilder2D::from_collider_shape(
                &Player::SHAPE,
                Player::RESOLUTION,
                0.0,
            )),
            floor_mesh: ctx.gpu.create_mesh(&MeshBuilder2D::from_collider_shape(
                &Floor::SHAPE,
                Floor::RESOLUTION,
                0.0,
            )),
            box_mesh: ctx.gpu.create_mesh(&MeshBuilder2D::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
        }
    }
}

#[derive(Entity, serde::Serialize, serde::Deserialize)]
#[serde(crate = "shura::serde")]
struct Player {
    #[shura(component = "player")]
    body: RigidBodyComponent,
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
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(Vector2::new(5.0, 4.0)),
                [collider],
            ),
        }
    }
}

#[derive(Entity, serde::Serialize, serde::Deserialize)]
#[serde(crate = "shura::serde")]
struct Floor {
    #[shura(component = "floor")]
    collider: ColliderComponent,
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
            collider: ColliderComponent::new(collider).with_color(Color::BLUE),
        }
    }
}

#[derive(Entity, serde::Serialize, serde::Deserialize)]
#[serde(crate = "shura::serde")]
struct PhysicsBox {
    #[shura(component = "box")]
    body: RigidBodyComponent,
}

impl PhysicsBox {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector2::new(PhysicsBox::HALF_BOX_SIZE, PhysicsBox::HALF_BOX_SIZE),
    };
    pub fn new(instance: Vector2<f32>) -> Self {
        Self {
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(instance),
                [ColliderBuilder::new(SharedShape::new(
                    PhysicsBox::BOX_SHAPE,
                ))],
            )
            .with_color(Color::GREEN),
        }
    }
}
