use shura::{physics::*, prelude::*};

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity_single::<Floor>()
            .entity_single::<Player>()
            .entity::<PhysicsBox>()
            .system(System::render(render))
            .system(System::setup(setup))
            .system(System::update(update))
    });
}

fn setup(ctx: &mut Context) {
    const PYRAMID_ELEMENTS: i32 = 8;
    const MINIMAL_SPACING: f32 = 0.1;
    ctx.world_camera2d.set_scaling(WorldCameraScaling::Max(5.0));
    ctx.world.set_gravity(Vector2::new(0.00, -9.81));

    let mut boxes = ctx.entities.get_mut();
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

    ctx.assets.load_sprite(
        "burger",
        SpriteBuilder::bytes(include_asset_bytes!("physics/burger.png")),
    );
    ctx.assets
        .load_smart_instance_buffer("boxes", SmartInstanceBuffer::<ColorInstance2D>::EVERY_FRAME);
    ctx.assets
        .load_smart_mesh("floor", SmartMesh::<ColorVertex2D>::MANUAL);
    ctx.assets
        .load_smart_mesh("player", SmartMesh::<SpriteVertex2D>::EVERY_FRAME);
}

fn update(ctx: &mut Context) {
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
        if physics_box.color == Color::RED {
            physics_box.color = Color::GREEN;
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
            physics_box.color = Color::RED;
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
                b.color = match event.collision_type {
                    CollisionType::Started => Color::BLUE,
                    CollisionType::Stopped => Color::GREEN,
                }
            }
        }
    });

    ctx.world_camera2d
        .set_translation(*player.body.get_mut(ctx.world).translation());
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(Some(Color::BLACK), |renderer| {
        renderer.draw_color(
            &ctx.assets.smart_instances("boxes"),
            &ctx.default_assets.world_camera2d,
            &ctx.default_assets.position_mesh,
        );

        renderer.draw_sprite_mesh(
            &ctx.default_assets.world_camera2d,
            &ctx.assets.smart_mesh("player"),
            &ctx.assets.get("burger"),
        );

        renderer.draw_color_mesh(
            &ctx.default_assets.world_camera2d,
            &ctx.assets.get::<SmartMesh<ColorVertex2D>>("floor").mesh(),
        );
    })
}

#[derive(Entity)]
#[shura(
    asset = "player", 
    ty = SmartMesh<SpriteVertex2D>,
    action = |player, asset, ctx| asset.push_offset(&player.mesh, player.body.position(ctx.world))
)]
struct Player {
    #[shura(component)]
    body: RigidBodyComponent,
    mesh: MeshData2D<SpriteVertex2D>,
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
        let mesh = MeshData2D::from_collider_shape(&Player::SHAPE, Player::RESOLUTION, 0.0);
        Self {
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(Vector2::new(5.0, 4.0)),
                [collider],
            ),
            mesh,
        }
    }
}

#[derive(Entity)]
#[shura(
    asset = "floor", 
    ty = SmartMesh<ColorVertex2D>,
    action = |floor, asset, ctx| asset.push_offset(&floor.mesh, floor.collider.position(ctx.world))
)]
struct Floor {
    #[shura(component)]
    collider: ColliderComponent,
    mesh: MeshData2D<ColorVertex2D>,
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
            collider: ColliderComponent::new(collider),
            mesh: MeshData2D::from_collider_shape(&Floor::SHAPE, Floor::RESOLUTION, 0.0)
                .set_data(Color::BLUE),
        }
    }
}

#[derive(Entity)]
#[shura(
    asset = "boxes", 
    ty = SmartInstanceBuffer<ColorInstance2D>,
    action = |b, asset, _|asset.push(ColorInstance2D::new(b.body.position(ctx.world), Vector2::new(Self::HALF_BOX_SIZE * 2., Self::HALF_BOX_SIZE * 2.), b.color));
)]
struct PhysicsBox {
    #[shura(component)]
    body: RigidBodyComponent,
    color: Color,
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
            ),
            color: Color::GREEN,
        }
    }
}
