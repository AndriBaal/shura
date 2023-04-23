use shura::log::*;
use shura::physics::*;
use shura::*;
use std::{fmt, fs};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    if let Some(save_game) = fs::read("data.binc").ok() {
        config.init(SerializedScene {
            id: 1,
            scene: save_game,
            init: |ctx, s| {
                s.deserialize_components_with(ctx, |w, ctx| w.deserialize(FloorVisitor { ctx }));
                s.deserialize_components_with(ctx, |w, ctx| w.deserialize(PlayerVisitor { ctx }));
                s.deserialize_components::<PhysicsBox>(ctx);
                s.deserialize_scene_state_with(ctx, |w, ctx| {
                    w.deserialize(PhysicsStateVisitor { ctx })
                });
            },
        })
    } else {
        config.init(NewScene {
            id: 1,
            init: |ctx| {
                const PYRAMID_ELEMENTS: i32 = 8;
                const MINIMAL_SPACING: f32 = 0.1;
                ctx.set_camera_scale(WorldCameraScale::Max(5.0));
                ctx.set_gravity(Vector::new(0.00, -9.81));
                ctx.set_scene_state(PhysicsState::new(ctx));

                for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                    for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                        ctx.add_component(PhysicsBox::new(Vector::new(
                            x as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                            y as f32 * (PhysicsBox::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                        )));
                    }
                }

                let (_, player_handle) = ctx.add_component(Player::new(ctx));
                ctx.set_camera_target(Some(player_handle));
                ctx.add_component(Floor::new(ctx));
            },
        })
    };
}

#[derive(State, serde::Serialize)]
struct PhysicsState {
    #[serde(skip)]
    default_color: Uniform<Color>,
    #[serde(skip)]
    collision_color: Uniform<Color>,
    #[serde(skip)]
    hover_color: Uniform<Color>,
    #[serde(skip)]
    box_model: Model,
}

impl PhysicsState {
    pub fn new(ctx: &Context) -> Self {
        Self {
            default_color: ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: ctx.create_model(ModelBuilder::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
        }
    }

    fn serialize_scene(ctx: &mut Context) {
        info!("Serializing scene!");
        let ser = ctx
            .serialize_scene(GroupFilter::All, |s| {
                s.serialize_scene_state::<Self>();
                s.serialize_components::<Floor>();
                s.serialize_components::<Player>();
                s.serialize_components::<PhysicsBox>();
            })
            .unwrap();
        fs::write("data.binc", ser).expect("Unable to write file");
    }
}

impl SceneState for PhysicsState {
    fn update(ctx: &mut Context) {
        let scroll = ctx.wheel_delta();
        let fov = ctx.camera_fov();
        if scroll != 0.0 {
            ctx.set_camera_scale(WorldCameraScale::Max(fov.x + scroll / 5.0));
        }

        if ctx.is_held(MouseButton::Right) {
            let cursor = ctx.cursor_camera(&ctx.world_camera);
            let cursor_pos = Isometry::new(cursor, 0.0);
            if ctx
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
                ctx.add_component(PhysicsBox::new(cursor));
            }
        }

        if ctx.is_pressed(Key::Z) {
            Self::serialize_scene(ctx);
        }
    }

    fn end(ctx: &mut Context) {
        Self::serialize_scene(ctx);
    }
}

#[derive(Component, serde::Serialize)]
struct Player {
    #[serde(skip)]
    sprite: Sprite,
    #[serde(skip)]
    model: Model,
    #[base]
    base: BaseComponent,
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
            sprite: ctx.create_sprite(include_bytes!("./img/burger.png")),
            model: ctx.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::RESOLUTION,
                0.0,
            )),
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                vec![collider],
            ),
        }
    }
}

impl ComponentController for Player {
    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        let delta = ctx.frame_time();
        let input = &mut ctx.input;

        for player in &mut ctx.component_manager.path_mut(&active) {
            let mut body = player.base.rigid_body_mut().unwrap();
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

    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(active, encoder, RenderConfig::WORLD, |r, player, index| {
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
        if let Some(b) = ctx.component_mut::<PhysicsBox>(other_handle) {
            b.collided = collision_type == CollideType::Started;
        }
    }
}

#[derive(Component, serde::Serialize)]
struct Floor {
    #[serde(skip)]
    color: Uniform<Color>,
    #[serde(skip)]
    model: Model,
    #[base]
    base: BaseComponent,
}

impl Floor {
    const FLOOR_RESOLUTION: u32 = 12;
    const FLOOR_SHAPE: RoundCuboid = RoundCuboid {
        inner_shape: Cuboid {
            half_extents: Vector::new(20.0, 0.4),
        },
        border_radius: 0.1,
    };
    pub fn new(ctx: &Context) -> Self {
        let collider = ColliderBuilder::new(SharedShape::new(Self::FLOOR_SHAPE));
        Self {
            color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            model: ctx.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::FLOOR_RESOLUTION,
                0.0,
            )),
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(Vector::new(0.0, -1.0)),
                vec![collider],
            ),
        }
    }
}

impl ComponentController for Floor {
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(active, encoder, RenderConfig::WORLD, |r, floor, index| {
            r.render_color(index, &floor.model, &floor.color)
        })
    }
}

#[derive(Component, serde::Serialize, serde::Deserialize)]
struct PhysicsBox {
    collided: bool,
    hovered: bool,
    #[base]
    base: BaseComponent,
}

impl PhysicsBox {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector::new(PhysicsBox::HALF_BOX_SIZE, PhysicsBox::HALF_BOX_SIZE),
    };
    pub fn new(position: Vector<f32>) -> Self {
        Self {
            collided: false,
            hovered: false,
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(position),
                vec![ColliderBuilder::new(SharedShape::new(
                    PhysicsBox::BOX_SHAPE,
                ))],
            ),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::WORLD);
        let state = ctx.scene_state::<PhysicsState>();
        for (buffer, boxes) in ctx.path_render(&active) {
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

    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        let cursor_world: Point<f32> = (ctx.cursor_camera(&ctx.world_camera)).into();
        let remove = ctx.is_held(MouseButton::Left) || ctx.is_pressed(ScreenTouch);
        for physics_box in &mut ctx.path_mut(&active) {
            physics_box.hovered = false;
        }
        let mut component: Option<ComponentHandle> = None;
        ctx.intersections_with_point(&cursor_world, Default::default(), |component_handle, _| {
            component = Some(component_handle);
            false
        });
        if let Some(handle) = component {
            if let Some(physics_box) = ctx.component_mut::<Self>(handle) {
                physics_box.hovered = true;
                if remove {
                    ctx.remove_component(handle);
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

    fn visit_seq<V>(self, mut seq: V) -> Result<Floor, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let base: BaseComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Floor {
            base,
            color: self.ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            model: self.ctx.create_model(ModelBuilder::from_collider_shape(
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

    fn visit_seq<V>(self, mut seq: V) -> Result<Player, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let base: BaseComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Player {
            base,
            sprite: self.ctx.create_sprite(include_bytes!("./img/burger.png")),
            model: self.ctx.create_model(ModelBuilder::from_collider_shape(
                &Player::SHAPE,
                Player::RESOLUTION,
                0.0,
            )),
        })
    }
}

struct PhysicsStateVisitor<'a> {
    ctx: &'a Context<'a>,
}

impl<'de, 'a> serde::de::Visitor<'de> for PhysicsStateVisitor<'a> {
    type Value = PhysicsState;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A PhysicsState")
    }

    fn visit_seq<V>(self, _seq: V) -> Result<PhysicsState, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        Ok(PhysicsState {
            default_color: self.ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: self.ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: self.ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: self.ctx.create_model(ModelBuilder::from_collider_shape(
                &PhysicsBox::BOX_SHAPE,
                0,
                0.0,
            )),
        })
    }
}
