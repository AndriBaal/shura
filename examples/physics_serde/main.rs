use shura::log::*;
use shura::physics::*;
use shura::{serde::*, *};
use std::fs;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    if let Some(save_game) = fs::read("data.binc").ok() {
        config.init(move || {
            SerializedScene::new(1, &save_game, Player::deserialize_scene, Player::init_scene)
        })
    } else {
        config.init(|| NewScene {
            id: 1,
            init: |ctx| {
                register!(ctx, [PhysicsBox, Player, Floor, PhysicsResources]);
                const PYRAMID_ELEMENTS: i32 = 8;
                const MINIMAL_SPACING: f32 = 0.1;
                ctx.world_camera.set_scaling(WorldCameraScale::Max(5.0));
                ctx.world.set_gravity(Vector::new(0.00, -9.81));
                ctx.components.add(ctx.world, PhysicsResources::new(ctx));

                for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                    for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                        let b = PhysicsBox::new(Vector::new(
                            x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                            y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                        ));
                        ctx.components.add(ctx.world, b);
                    }
                }

                let player = Player::new();
                let player_handle = ctx.components.add(ctx.world, player);
                ctx.world_camera.set_target(Some(player_handle));
                let floor = Floor::new();
                ctx.components.add(ctx.world, floor);
            },
        })
    };
}

#[derive(Component)]
struct PhysicsResources {
    box_colors: SpriteSheet,
    box_model: Model,
}

impl ComponentController for PhysicsResources {
    const CONFIG: ComponentConfig = ComponentConfig::RESOURCE;
}

impl PhysicsResources {
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

fn player_model() -> Model {
    let gpu = GLOBAL_GPU.get().unwrap();
    gpu.create_model(ModelBuilder::from_collider_shape(
        &Player::SHAPE,
        Player::RESOLUTION,
        0.0,
    ))
}

fn player_sprite() -> Sprite {
    let gpu = GLOBAL_GPU.get().unwrap();
    gpu.create_sprite(sprite_file!("./img/burger.png"))
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct Player {
    #[serde(skip)]
    #[serde(default = "player_sprite")]
    sprite: Sprite,
    #[serde(skip)]
    #[serde(default = "player_model")]
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

    pub fn new() -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::SHAPE))
            .active_events(ActiveEvents::COLLISION_EVENTS);
        Self {
            sprite: player_sprite(),
            model: player_model(),
            body: RigidBodyComponent::new(
                RigidBodyBuilder::dynamic().translation(Vector::new(9.0, 4.0)),
                [collider],
            ),
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
        fs::write("data.binc", ser).expect("Unable to write file");
    }

    fn deserialize_scene(des: &mut SceneDeserializer) {
        register!(des.scene, [PhysicsBox, Player, Floor, PhysicsResources]);
        des.deserialize::<Floor>();
        des.deserialize::<Player>();
        des.deserialize::<PhysicsBox>();
    }

    fn init_scene(ctx: &mut Context) {
        ctx.components.add(ctx.world, PhysicsResources::new(ctx));
    }
}

impl ComponentController for Player {
    const CONFIG: ComponentConfig = ComponentConfig {
        storage: ComponentStorage::Single,
        end: EndOperation::Always,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
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
                ctx.components.add(ctx.world, b);
            }
        }

        if ctx.input.is_pressed(Key::Z) {
            Self::serialize_scene(ctx);
        }

        if ctx.input.is_pressed(Key::R) {
            if let Some(save_game) = fs::read("data.binc").ok() {
                ctx.scenes.add(SerializedScene::new(
                    1,
                    &save_game,
                    Player::deserialize_scene,
                    Player::init_scene,
                ));
            }
        }

        let delta = ctx.frame.frame_time();
        let input = &mut ctx.input;

        ctx.components.for_each_mut::<Self>(|player| {
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
        });
    }

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        components.render_single::<Self>(|renderer, player, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                &player.model,
                &player.sprite,
            )
        });
    }

    fn collision(
        ctx: &mut Context,
        _self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if let Some(mut b) = ctx.components.get_mut::<PhysicsBox>(other_handle) {
            b.sprite = match collision_type {
                CollideType::Started => 2,
                CollideType::Stopped => 0,
            }
        }
    }

    fn end(ctx: &mut Context, _reason: EndReason) {
        Self::serialize_scene(ctx)
    }
}

fn floor_model() -> Model {
    let gpu = GLOBAL_GPU.get().unwrap();
    gpu.create_model(ModelBuilder::from_collider_shape(
        &Floor::FLOOR_SHAPE,
        Floor::FLOOR_RESOLUTION,
        0.0,
    ))
}

fn floor_sprite() -> Sprite {
    let gpu = GLOBAL_GPU.get().unwrap();
    gpu.create_sprite(SpriteBuilder::color(RgbaColor::BLUE))
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct Floor {
    #[serde(skip)]
    #[serde(default = "floor_sprite")]
    color: Sprite,
    #[serde(skip)]
    #[serde(default = "floor_model")]
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
    pub fn new() -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::FLOOR_SHAPE))
            .translation(Vector::new(0.0, -1.0));
        Self {
            color: floor_sprite(),
            model: floor_model(),
            collider: ColliderComponent::new(collider),
        }
    }
}

impl ComponentController for Floor {
    const CONFIG: ComponentConfig = ComponentConfig {
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        components.render_single::<Self>(|renderer, floor, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                &floor.model,
                &floor.color,
            )
        });
    }
}

#[derive(Component, ::serde::Serialize, ::serde::Deserialize)]
struct PhysicsBox {
    #[position]
    body: RigidBodyComponent,
    #[buffer]
    sprite: SpriteSheetIndex,
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
            sprite: 0,
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        let state = components.single::<PhysicsResources>();
        components.render_all::<Self>(|renderer, buffer, instance| {
            renderer.render_sprite_sheet(
                instance,
                buffer,
                renderer.world_camera,
                &state.box_model,
                &state.box_colors,
            );
        });
    }

    fn update(ctx: &mut Context) {
        let cursor_world: Point<f32> = (ctx.cursor).into();
        let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
        let mut boxes = ctx.components.set::<Self>();
        boxes.for_each_mut(|physics_box| {
            if physics_box.sprite == 1 {
                physics_box.sprite = 0;
            }
        });
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
            if let Some(physics_box) = boxes.get_mut(handle) {
                physics_box.sprite = 1;
                if remove {
                    boxes.remove(ctx.world, handle);
                }
            }
        }
    }
}
