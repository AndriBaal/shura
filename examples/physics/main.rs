use shura::physics::*;
use shura::*;

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Floor>(ComponentConfig::SINGLE)
            .component::<Player>(ComponentConfig::SINGLE)
            .component::<PhysicsBox>(Default::default())
            .component::<Resources>(ComponentConfig::RESOURCE)
            .system(System::Render(render))
            .system(System::Setup(setup))
            .system(System::Update(update))
    });
}

fn setup(ctx: &mut Context) {
    const PYRAMID_ELEMENTS: i32 = 8;
    const MINIMAL_SPACING: f32 = 0.1;
    ctx.world_camera.set_scaling(WorldCameraScale::Max(5.0));
    ctx.world.set_gravity(Vector::new(0.00, -9.81));
    ctx.components.add(ctx.world, Resources::new(ctx));

    for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
        for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
            let b = PhysicsBox::new(Vector::new(
                x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
            ));
            ctx.components.add(ctx.world, b);
        }
    }

    let player = Player::new(ctx);
    let player_handle = ctx.components.add(ctx.world, player);
    ctx.world_camera.set_target(Some(WorldCameraTarget {
        target: player_handle,
        ..Default::default()
    }));
    let floor = Floor::new(ctx);
    ctx.components.add(ctx.world, floor);
}

fn update(ctx: &mut Context) {
    let mut boxes = ctx.components.set::<PhysicsBox>();
    let mut player = ctx.components.single::<Player>();

    let scroll = ctx.input.wheel_delta();
    let fov = ctx.world_camera.fov();
    if scroll != 0.0 {
        ctx.world_camera
            .set_scaling(WorldCameraScale::Max(fov.x + scroll / 5.0));
    }

    if ctx.input.is_held(MouseButton::Right) {
        if ctx
            .world
            .intersection_with_shape(
                &ctx.cursor.into(),
                &Cuboid::new(Vector::new(
                    PhysicsBox::HALF_BOX_SIZE,
                    PhysicsBox::HALF_BOX_SIZE,
                )),
                Default::default(),
            )
            .is_none()
        {
            let b = PhysicsBox::new(ctx.cursor);
            boxes.add(ctx.world, b);
        }
    }
    let delta = ctx.frame.frame_time();
    let cursor_world: Point<f32> = (ctx.cursor).into();
    let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
    boxes.for_each_mut(|physics_box| {
        if *physics_box.body.index() == 1 {
            physics_box.body.set_index(0);
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
            physics_box.body.set_index(1);
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
                b.body.set_index(match event.collision_type {
                    CollisionType::Started => 2,
                    CollisionType::Stopped => 0,
                })
            }
        }
    });
}

fn render(res: &ComponentResources, encoder: &mut RenderEncoder) {
    let resources = res.single::<Resources>();
    encoder.render(Some(Color::BLACK), |renderer| {
        res.render_single::<Player>(renderer, |renderer, player, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                &player.model,
                &player.sprite,
            )
        });

        res.render_single::<Floor>(renderer, |renderer, floor, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                &floor.model,
                &floor.color,
            )
        });

        res.render_all::<PhysicsBox>(renderer, |renderer, buffer, instance| {
            renderer.render_sprite_sheet(
                instance,
                buffer,
                renderer.world_camera,
                &resources.box_model,
                &resources.box_colors,
            );
        });
    })
}

#[derive(Component)]
struct Resources {
    box_colors: SpriteSheet,
    box_model: Model,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        let box_colors = ctx.gpu.create_sprite_sheet(SpriteSheetBuilder::colors(&[
            RgbaColor::new(0, 255, 0, 255),
            RgbaColor::new(255, 0, 0, 255),
            RgbaColor::new(0, 0, 255, 255),
            RgbaColor::new(0, 0, 255, 255),
        ]));
        Self {
            box_model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
            box_colors,
        }
    }
}

#[derive(Component)]
struct Player {
    sprite: Sprite,
    model: Model,
    #[position]
    body: RigidBodyComponent,
}

impl Player {
    const RADIUS: f32 = 0.75;
    const RESOLUTION: u32 = 24;
    const SHAPE: Ball = Ball {
        radius: Self::RADIUS,
    };

    pub fn new(ctx: &Context) -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::SHAPE))
            .active_events(ActiveEvents::COLLISION_EVENTS);
        Self {
            sprite: ctx.gpu.create_sprite(sprite_file!("./img/burger.png")),
            model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::RESOLUTION,
                0.0,
            )),
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                [collider],
            ),
        }
    }
}

#[derive(Component)]
struct Floor {
    color: Sprite,
    model: Model,
    #[position]
    collider: ColliderComponent,
}

impl Floor {
    const FLOOR_RESOLUTION: u32 = 12;
    const FLOOR_SHAPE: RoundCuboid = RoundCuboid {
        inner_shape: Cuboid {
            half_extents: Vector::new(20.0, 0.4),
        },
        border_radius: 0.5,
    };
    pub fn new(ctx: &Context) -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::FLOOR_SHAPE))
            .translation(Vector::new(0.0, -1.0));
        Self {
            color: ctx.gpu.create_sprite(SpriteBuilder::color(RgbaColor::BLUE)),
            model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::FLOOR_RESOLUTION,
                0.0,
            )),
            collider: ColliderComponent::new(collider),
        }
    }
}

#[derive(Component)]
struct PhysicsBox {
    #[position]
    body: RigidBodyComponent,
}

impl PhysicsBox {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector::new(PhysicsBox::HALF_BOX_SIZE, PhysicsBox::HALF_BOX_SIZE),
    };
    pub fn new(position: Vector<f32>) -> Self {
        Self {
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(position),
                [ColliderBuilder::new(SharedShape::new(
                    PhysicsBox::BOX_SHAPE,
                ))],
            ),
        }
    }
}
