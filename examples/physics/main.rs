use shura::log::*;
use shura::physics::*;
use shura::*;
use std::{fmt, fs};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 1,
        init: |ctx| {
            const PYRAMID_ELEMENTS: i32 = 8;
            const MINIMAL_SPACING: f32 = 0.1;
            ctx.components.register::<PhysicsBox>();
            ctx.components.register::<Player>();
            ctx.components.register::<Floor>();
            ctx.world_camera.set_scaling(WorldCameraScale::Max(5.0));
            ctx.world.set_gravity(Vector::new(0.00, -9.81));
            ctx.scene_states.insert(PhysicsState::new(ctx));

            for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                    let b = PhysicsBox::new(
                        ctx,
                        Vector::new(
                            x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                            y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                        ),
                    );
                    ctx.components.add(GroupHandle::DEFAULT_GROUP, b);
                }
            }

            let player = Player::new(ctx);
            let player_handle = ctx
                .components
                .add(GroupHandle::DEFAULT_GROUP, player);
            ctx.world_camera.set_target(Some(player_handle));
            let floor = Floor::new(ctx);
            ctx.components
                .add(GroupHandle::DEFAULT_GROUP, floor);
        },
    })
}

#[derive(State)]
struct PhysicsState {
    default_color: Uniform<Color>,
    collision_color: Uniform<Color>,
    hover_color: Uniform<Color>,
    box_model: Model,
}

impl PhysicsState {
    pub fn new(ctx: &Context) -> Self {
        Self {
            default_color: ctx.gpu.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: ctx.gpu.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: ctx.gpu.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
        }
    }

    // fn serialize_scene(ctx: &mut Context) {
    //     info!("Serializing scene!");
    //     let ser = ctx
    //         .serialize_scene(ComponentFilter::All, |s| {
    //             s.serialize_scene_state::<Self>();
    //             s.serialize_components::<Floor>();
    //             s.serialize_components::<Player>();
    //             s.serialize_components::<PhysicsBox>();
    //         })
    //         .unwrap();
    //     fs::write("data.binc", ser).expect("Unable to write file");
    // }
}

impl SceneStateController for PhysicsState {
    fn update(ctx: &mut Context) {
        let scroll = ctx.input.wheel_delta();
        let fov = ctx.world_camera.fov();
        if scroll != 0.0 {
            ctx.world_camera
                .set_scaling(WorldCameraScale::Max(fov.x + scroll / 5.0));
        }

        if ctx.input.is_held(MouseButton::Right) {
            let cursor = ctx.input.cursor(&ctx.world_camera);
            let cursor_pos = Isometry::new(cursor, 0.0);
            if ctx
                .world
                .intersection_with_shape(
                    &cursor_pos,
                    &Cuboid::new(Vector::new(
                        PhysicsBox::HALF_BOX_SIZE,
                        PhysicsBox::HALF_BOX_SIZE,
                    )),
                    Default::default(),
                )
                .is_none()
            {
                let b = PhysicsBox::new(ctx, cursor);
                ctx.components.add(GroupHandle::DEFAULT_GROUP, b);
            }
        }

        // if ctx.is_pressed(Key::Z) {
        //     Self::serialize_scene(ctx);
        // }
    }

    // fn end(ctx: &mut Context) {
    //     Self::serialize_scene(ctx);
    // }
}

#[derive(Component)]
struct Player {
    sprite: Sprite,
    model: Model,
    #[base]
    body: RigidBodyComponent,
}

impl Player {
    const RADIUS: f32 = 0.75;
    const RESOLUTION: u32 = 24;
    const SHAPE: Ball = Ball {
        radius: Self::RADIUS,
    };
    pub fn new(ctx: &mut Context) -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::SHAPE))
            .active_events(ActiveEvents::COLLISION_EVENTS);
        Self {
            sprite: ctx.gpu.create_sprite(include_bytes!("./img/burger.png")),
            model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::RESOLUTION,
                0.0,
            )),
            body: RigidBodyComponent::new(
                ctx.world,
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                [collider],
            ),
        }
    }
}

impl ComponentController for Player {
    fn update(ctx: &mut Context) {
        let delta = ctx.frame.frame_time();
        let input = &mut ctx.input;

        for player in &mut ctx.components.iter_mut::<Self>(ComponentFilter::Active) {
            let body = player.body.get_mut(ctx.world);
            let mut linvel = *body.linvel();

            if input.is_held(Key::D) {
                linvel.x += 15.0 * delta;
            }

            if input.is_held(Key::A) {
                linvel.x += -15.0 * delta;
            }

            if input.is_pressed(Key::W) {
                linvel.y += 15.0;
            }

            if input.is_pressed(Key::S) {
                linvel.y = -17.0;
            }

            body.set_linvel(linvel, true);
        }
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::WORLD, |r, player, index| {
            r.render_sprite(index, &player.model, &player.sprite)
        })
    }

    fn collision(
        ctx: &mut Context,
        _self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if let Some(b) = ctx.components.get_mut::<PhysicsBox>(other_handle) {
            b.collided = collision_type == CollideType::Started;
        }
    }
}

#[derive(Component)]
struct Floor {
    color: Uniform<Color>,
    model: Model,
    #[base]
    collider: ColliderComponent,
}

impl Floor {
    const FLOOR_RESOLUTION: u32 = 12;
    const FLOOR_SHAPE: RoundCuboid = RoundCuboid {
        inner_shape: Cuboid {
            half_extents: Vector::new(20.0, 0.4),
        },
        border_radius: 0.1,
    };
    pub fn new(ctx: &mut Context) -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::FLOOR_SHAPE))
            .translation(Vector::new(0.0, -1.0));
        Self {
            color: ctx.gpu.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            model: ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::FLOOR_RESOLUTION,
                0.0,
            )),
            collider: ColliderComponent::new(ctx.world, collider),
        }
    }
}

impl ComponentController for Floor {
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::WORLD, |r, floor, index| {
            r.render_color(index, &floor.model, &floor.color)
        })
    }
}

#[derive(Component)]
struct PhysicsBox {
    collided: bool,
    hovered: bool,
    #[base]
    body: RigidBodyComponent,
}

impl PhysicsBox {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector::new(PhysicsBox::HALF_BOX_SIZE, PhysicsBox::HALF_BOX_SIZE),
    };
    pub fn new(ctx: &mut Context, position: Vector<f32>) -> Self {
        Self {
            collided: false,
            hovered: false,
            body: RigidBodyComponent::new(
                ctx.world,
                RigidBodyBuilder::dynamic().translation(position),
                [ColliderBuilder::new(SharedShape::new(
                    PhysicsBox::BOX_SHAPE,
                ))],
            ),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::WORLD);
        let state = ctx.scene_states.get::<PhysicsState>();
        for (buffer, boxes) in ctx.components.iter_render::<Self>(ComponentFilter::Active) {
            let mut ranges = vec![];
            let mut last = 0;
            for (i, b) in boxes.clone() {
                if b.collided {
                    ranges.push((&state.default_color, last..i.index));
                    ranges.push((&state.collision_color, i.index..i.index + 1));
                    last = i.index + 1;
                } else if b.hovered {
                    ranges.push((&state.default_color, last..i.index));
                    ranges.push((&state.hover_color, i.index..i.index + 1));
                    last = i.index + 1;
                }
            }
            ranges.push((&state.default_color, last..buffer.len()));
            renderer.use_instances(buffer);
            for (color, r) in ranges {
                renderer.render_color(r, &state.box_model, color)
            }
        }
    }

    fn update(ctx: &mut Context) {
        let cursor_world: Point<f32> = (ctx.input.cursor(&ctx.world_camera)).into();
        let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
        for physics_box in ctx.components.iter_mut::<Self>(ComponentFilter::Active) {
            physics_box.hovered = false;
        }
        let mut component: Option<ComponentHandle> = None;
        ctx.world.intersections_with_point(
            &cursor_world,
            Default::default(),
            |component_handle, _| {
                component = Some(component_handle);
                false
            },
        );
        if let Some(handle) = component {
            if let Some(physics_box) = ctx.components.get_mut::<Self>(handle) {
                physics_box.hovered = true;
                if remove {
                    ctx.components.remove_boxed(handle);
                }
            }
        }
    }
}
