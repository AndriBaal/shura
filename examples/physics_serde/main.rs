use shura::log::*;
use shura::physics::*;
use shura::{serde, *};
use std::{fmt, fs};

fn register(ctx: &mut Context) {
    register!(ctx, [PhysicsBox, Player, Floor, PhysicsResources]);
}

#[shura::main]
fn shura_main(config: ShuraConfig) {
    if let Some(save_game) = fs::read("data.binc").ok() {
        config.init(SerializedScene {
            id: 1,
            scene: save_game,
            init: |ctx, s| {
                register(ctx);
                s.deserialize_components_with(ctx, |w, ctx| w.deserialize(FloorVisitor { ctx }));
                s.deserialize_components_with(ctx, |w, ctx| w.deserialize(PlayerVisitor { ctx }));
                s.deserialize_components::<PhysicsBox>(ctx);
                ctx.components
                    .set::<PhysicsResources>()
                    .add(ctx.world, PhysicsResources::new(ctx));
            },
        })
    } else {
        config.init(NewScene {
            id: 1,
            init: |ctx| {
                register(ctx);
                const PYRAMID_ELEMENTS: i32 = 8;
                const MINIMAL_SPACING: f32 = 0.1;
                ctx.world_camera.set_scaling(WorldCameraScale::Max(5.0));
                ctx.world.set_gravity(Vector::new(0.00, -9.81));
                ctx.components
                    .set::<PhysicsResources>()
                    .add(ctx.world, PhysicsResources::new(ctx));

                for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                    for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                        let b = PhysicsBox::new(Vector::new(
                            x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                            y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                        ));
                        ctx.components.set::<PhysicsBox>().add(ctx.world, b);
                    }
                }

                let player = Player::new(ctx);
                let player_handle = ctx.components.set::<Player>().add(ctx.world, player);
                ctx.world_camera.set_target(Some(player_handle));
                let floor = Floor::new(ctx);
                ctx.components.set::<Floor>().add(ctx.world, floor);
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

#[derive(Component, serde::Serialize)]
struct Player {
    #[serde(skip)]
    sprite: Sprite,
    #[serde(skip)]
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

    fn serialize_scene(ctx: &mut Context) {
        info!("Serializing scene!");
        let ser = ctx
            .serialize_scene(|s| {
                s.serialize_components::<Floor>();
                s.serialize_components::<Player>();
                s.serialize_components::<PhysicsBox>();
            })
            .unwrap();
        fs::write("data.binc", ser).expect("Unable to write file");
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
            let cursor = ctx.input.cursor(ctx.world_camera);
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
                let b = PhysicsBox::new(cursor);
                ctx.components.set::<PhysicsBox>().add(ctx.world, b);
            }
        }

        if ctx.input.is_pressed(Key::Z) {
            Self::serialize_scene(ctx);
        }

        if ctx.input.is_pressed(Key::R) {
            if let Some(save_game) = fs::read("data.binc").ok() {
                ctx.scenes.add(SerializedScene {
                    id: 1,
                    scene: save_game,
                    init: |ctx, s| {
                        s.deserialize_components_with(ctx, |w, ctx| {
                            w.deserialize(FloorVisitor { ctx })
                        });
                        s.deserialize_components_with(ctx, |w, ctx| {
                            w.deserialize(PlayerVisitor { ctx })
                        });
                        s.deserialize_components::<PhysicsBox>(ctx);
                        ctx.components
                            .set::<PhysicsResources>()
                            .add(ctx.world, PhysicsResources::new(ctx));
                    },
                });
            }
        }

        let delta = ctx.frame.frame_time();
        let input = &mut ctx.input;

        ctx.components.set::<Self>().for_each_mut(|player| {
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

    fn render<'a>(ctx: &'a Context, renderer: &mut ComponentRenderer<'a>) {
        renderer.resource::<Self>(ctx).render_single(
            renderer,
            RenderCamera::World,
            |r, player, index| r.render_sprite(index, &player.model, &player.sprite),
        );
    }

    fn collision(
        ctx: &mut Context,
        _self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if let Some(b) = ctx.components.set::<PhysicsBox>().get_mut(other_handle) {
            match collision_type {
                CollideType::Started => b.sprite = Vector::new(2, 0),
                CollideType::Stopped => b.sprite = Vector::new(0, 0),
            }
        }
    }

    fn end(ctx: &mut Context, _reason: EndReason) {
        Self::serialize_scene(ctx)
    }
}

#[derive(Component, serde::Serialize)]
struct Floor {
    #[serde(skip)]
    color: Sprite,
    #[serde(skip)]
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

impl ComponentController for Floor {
    const CONFIG: ComponentConfig = ComponentConfig {
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut ComponentRenderer<'a>) {
        renderer.render_single::<Self>(ctx, RenderCamera::World, |r, floor, index| {
            r.render_sprite(index, &floor.model, &floor.color)
        });
    }
}

#[derive(Component, serde::Serialize, serde::Deserialize)]
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
            sprite: vector(0, 0),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render<'a>(ctx: &'a Context, renderer: &mut ComponentRenderer<'a>) {
        let state = renderer.resource::<PhysicsResources>(ctx).single();
        renderer.render_all::<Self>(ctx, RenderCamera::World, |renderer, instance| {
            renderer.render_sprite_sheet(instance, &state.box_model, &state.box_colors);
        });
    }

    fn update(ctx: &mut Context) {
        let cursor_world: Point<f32> = (ctx.input.cursor(ctx.world_camera)).into();
        let remove = ctx.input.is_held(MouseButton::Left) || ctx.input.is_pressed(ScreenTouch);
        ctx.components
            .set::<Self>()
            .for_each_mut(|physics_box| {
                if physics_box.sprite == vector(1, 0) {
                    physics_box.sprite = Vector::new(0, 0);
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
            let mut boxes = ctx.components.set::<Self>();
            if let Some(physics_box) = boxes.get_mut(handle) {
                physics_box.sprite = Vector::new(1, 0);
                if remove {
                    boxes.remove_boxed(ctx.world, handle);
                }
            }
        }
    }
}

struct FloorVisitor<'a> {
    ctx: &'a Context<'a>,
}

impl<'de, 'a> serde::de::Visitor<'de> for FloorVisitor<'a> {
    type Value = Floor;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A Floor")
    }

    fn visit_seq<V: serde::de::SeqAccess<'de>>(self, mut seq: V) -> Result<Floor, V::Error> {
        let collider: ColliderComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Floor {
            collider,
            color: self
                .ctx
                .gpu
                .create_sprite(SpriteBuilder::color(RgbaColor::BLUE)),
            model: self.ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                &Floor::FLOOR_SHAPE,
                Floor::FLOOR_RESOLUTION,
                0.0,
            )),
        })
    }
}

struct PlayerVisitor<'a> {
    ctx: &'a Context<'a>,
}

impl<'de, 'a> serde::de::Visitor<'de> for PlayerVisitor<'a> {
    type Value = Player;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A Player")
    }

    fn visit_seq<V: serde::de::SeqAccess<'de>>(self, mut seq: V) -> Result<Player, V::Error> {
        let body: RigidBodyComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Player {
            body,
            sprite: self.ctx.gpu.create_sprite(sprite_file!("./img/burger.png")),
            model: self.ctx.gpu.create_model(ModelBuilder::from_collider_shape(
                &Player::SHAPE,
                Player::RESOLUTION,
                0.0,
            )),
        })
    }
}
