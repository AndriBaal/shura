use shura::{physics::*, prelude::*};

fn deserialized_scene(data: Vec<u8>) -> SerializedScene {
    SerializedScene::new(1, &data)
        .render_group2d("player", RenderGroupUpdate::default())
        .render_group2d("box", RenderGroupUpdate::default())
        .render_group2d(
            "floor",
            RenderGroupUpdate {
                call: BufferCall::Manual,
                ..Default::default()
            },
        )
        .deserialize_entity_single::<Floor>()
        .deserialize_entity_single::<Player>()
        .deserialize_entity::<PhysicsBox>()
        .entity_single::<Assets>()
        .system(System::render(render))
        .system(System::setup(|ctx| {
            ctx.entities.single_mut().set(ctx.world, Assets::new(ctx));
        }))
        .system(System::update(update))
        .system(System::end(end))
}

#[shura::main]
fn app(config: AppConfig) {
    if let Ok(save_game) = load_data_bytes("data.binc") {
        App::run(config, move || deserialized_scene(save_game))
    } else {
        App::run(config, || {
            Scene::new()
                .render_group2d("player", RenderGroupUpdate::default())
                .render_group2d("box", RenderGroupUpdate::default())
                .render_group2d(
                    "floor",
                    RenderGroupUpdate {
                        call: BufferCall::Manual,
                        ..Default::default()
                    },
                )
                .entity_single::<Floor>()
                .entity_single::<Player>()
                .entity_single::<Assets>()
                .entity::<PhysicsBox>()
                .system(System::render(render))
                .system(System::setup(setup))
                .system(System::update(update))
                .system(System::end(end))
        });
    };
}

fn setup(ctx: &mut Context) {
    const PYRAMID_ELEMENTS: i32 = 8;
    const MINIMAL_SPACING: f32 = 0.1;
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Max(5.0));
    ctx.world.set_gravity(Vector2::new(0.00, -9.81));

    let mut boxes = ctx.entities.get_mut::<PhysicsBox>();
    for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
        for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
            let b = PhysicsBox::new(Vector2::new(
                x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
            ));
            boxes.add(ctx.world, b);
        }
    }

    ctx.entities.single_mut().set(ctx.world, Player::new());
    ctx.entities.single_mut().set(ctx.world, Floor::new());
    ctx.entities.single_mut().set(ctx.world, Assets::new(ctx));
}

fn update(ctx: &mut Context) {
    if ctx.input.is_pressed(Key::KeyZ) {
        serialize_scene(ctx);
    }

    if ctx.input.is_pressed(Key::KeyR) {
        if let Ok(save_game) = load_data_bytes("data.binc") {
            let active_scene_id = ctx.scenes.active_scene_id();
            ctx.add_scene(
                active_scene_id + 1,
                deserialized_scene(save_game).system(System::setup(move |ctx| {
                    ctx.remove_scene(active_scene_id).unwrap();
                })),
            );
            ctx.scenes.set_next_active_scene(active_scene_id + 1);
        }
    }

    let mut boxes = ctx.entities.get_mut::<PhysicsBox>();
    let mut player = ctx.entities.single_mut::<Player>().unwrap();

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

    let delta = ctx.time.delta();
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
        if let Some(physics_box) = boxes.get_mut(&handle) {
            physics_box.body.set_color(Color::RED);
            if remove {
                boxes.remove(ctx.world, &handle);
            }
        }
    }

    let body = player.body.get_mut(ctx.world);
    let mut linvel = *body.linvel();

    if ctx.input.is_held(Key::KeyD) {
        linvel.x += 15.0 * delta;
    }

    if ctx.input.is_held(Key::KeyA) {
        linvel.x += -15.0 * delta;
    }

    if ctx.input.is_pressed(Key::KeyW) {
        linvel.y += 15.0;
    }

    if ctx.input.is_pressed(Key::KeyS) {
        linvel.y = -17.0;
    }

    body.set_linvel(linvel, true);

    ctx.world.step(ctx.time.delta()).collisions(|event| {
        if let Some(event) = event.is::<Player, PhysicsBox>(ctx.world) {
            if let Some(b) = boxes.get_mut(&event.entity2) {
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
    let assets = ctx.entities.single::<Assets>().unwrap();
    encoder.render2d(Some(Color::BLACK), |renderer| {
        ctx.group("player", |buffer| {
            renderer.draw_sprite(
                buffer,
                ctx.world_camera2d,
                &assets.player_mesh,
                &assets.player_sprite,
            )
        });

        ctx.group("floor", |buffer| {
            renderer.draw_color(buffer, ctx.world_camera2d, &assets.floor_mesh)
        });

        ctx.group("box", |buffer| {
            renderer.draw_color(buffer, ctx.world_camera2d, &assets.box_mesh);
        });
    })
}

fn end(ctx: &mut Context, reason: EndReason) {
    if reason == EndReason::Close {
        serialize_scene(ctx)
    }
}

fn serialize_scene(ctx: &mut Context) {
    info!("Serializing scene!");
    let ser = ctx
        .serialize_scene(|serializer| {
            serializer
                .serialize_entity_single::<Floor>()
                .serialize_entity_single::<Player>()
                .serialize_entity_single::<PhysicsBox>()
        })
        .unwrap();
    save_data("data.binc", ser).unwrap();
}

#[derive(Entity)]
struct Assets {
    floor_mesh: Mesh2D,
    box_mesh: Mesh2D,
    player_mesh: Mesh2D,
    player_sprite: Sprite,
}

impl Assets {
    pub fn new(ctx: &Context) -> Self {
        Self {
            player_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
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
