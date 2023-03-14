use shura::log::*;
use shura::physics::*;
use shura::*;
use std::{fmt, fs};

fn main() {
    if let Some(save_game) = fs::read("data.binc").ok() {
        Shura::init(SerializedScene {
            id: 1,
            scene: save_game,
            init: |ctx, s| {
                s.deserialize_components_with(ctx, |mut w, ctx| {
                    w.deserialize(FloorVisitor { ctx })
                });
                s.deserialize_components_with(ctx, |mut w, ctx| {
                    w.deserialize(PlayerVisitor { ctx })
                });
                s.deserialize_components_with(ctx, |mut w, ctx| {
                    w.deserialize(BoxManagerVisitor { ctx })
                });
                s.deserialize_components::<PhysicsBox>(ctx);
            },
        })
    } else {
        Shura::init(NewScene {
            id: 1,
            init: |ctx| {
                const PYRAMID_ELEMENTS: i32 = 8;
                const MINIMAL_SPACING: f32 = 0.1;
                ctx.set_camera_horizontal_fov(10.0);
                ctx.set_gravity(Vector::new(0.00, -9.81));

                for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                    for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                        ctx.create_component(PhysicsBox::new(Vector::new(
                            x as f32 * (BoxManager::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                            y as f32 * (BoxManager::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                        )));
                    }
                }

                let (_, player_handle) = ctx.create_component(Player::new(ctx));
                ctx.set_camera_target(Some(player_handle));
                ctx.create_component(Floor::new(ctx));
                ctx.create_component(BoxManager::new(ctx));
            },
        })
    };
}

#[derive(Component, serde::Serialize)]
struct BoxManager {
    #[serde(skip)]
    default_color: Uniform<Color>,
    #[serde(skip)]
    collision_color: Uniform<Color>,
    #[serde(skip)]
    hover_color: Uniform<Color>,
    #[serde(skip)]
    box_model: Model,
    #[component]
    component: BaseComponent,
}

impl BoxManager {
    const HALF_BOX_SIZE: f32 = 0.3;
    const BOX_SHAPE: Cuboid = Cuboid {
        half_extents: Vector::new(BoxManager::HALF_BOX_SIZE, BoxManager::HALF_BOX_SIZE),
    };
    pub fn new(ctx: &Context) -> Self {
        Self {
            default_color: ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: ctx.create_model(ModelBuilder::from_collider_shape(
                &Self::BOX_SHAPE,
                0,
                0.0,
            )),
            component: Default::default(),
        }
    }

    fn serialize_scene(ctx: &mut Context) {
        info!("Serializing scene!");
        let ser = ctx
            .serialize(|s| {
                s.serialize_components::<Floor>(GroupFilter::All);
                s.serialize_components::<Player>(GroupFilter::All);
                s.serialize_components::<BoxManager>(GroupFilter::All);
                s.serialize_components::<PhysicsBox>(GroupFilter::All);
            })
            .unwrap();
        fs::write("data.binc", ser).expect("Unable to write file");
    }
}

impl ComponentController for BoxManager {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        render: RenderOperation::Never,
        end: EndOperation::AllComponents,
        ..DEFAULT_CONFIG
    };
    fn update(_active: ComponentPath<Self>, ctx: &mut Context) {
        let scroll = ctx.wheel_delta();
        let fov = ctx.camera_fov();
        if scroll != 0.0 {
            ctx.set_camera_horizontal_fov(fov.x + scroll);
        }

        if ctx.is_held(MouseButton::Right) {
            let cursor = ctx.cursor_camera(&ctx.scene.world_camera);
            let cursor_pos = Isometry::new(cursor, 0.0);
            if ctx
                .intersection_with_shape(
                    &cursor_pos,
                    &Cuboid::new(Vector::new(
                        BoxManager::HALF_BOX_SIZE,
                        BoxManager::HALF_BOX_SIZE,
                    )),
                    Default::default(),
                )
                .is_none()
            {
                ctx.create_component(PhysicsBox::new(cursor));
            }
        }

        if ctx.is_pressed(Key::Z) {
            Self::serialize_scene(ctx);
        }
    }

    fn end(_all: ComponentPath<Self>, ctx: &mut Context) {
        Self::serialize_scene(ctx);
    }
}

#[derive(Component, serde::Serialize)]
struct Player {
    #[serde(skip)]
    sprite: Sprite,
    #[serde(skip)]
    model: Model,
    #[component]
    component: BaseComponent,
}

impl Player {
    const RADIUS: f32 = 0.75;
    const RESOLUTION: u32 = 24;
    const SHAPE: Ball = Ball {
        radius: Self::RADIUS,
    };
    pub fn new(ctx: &Context) -> Self {
        let collider =
            ColliderBuilder::ball(Self::RADIUS).active_events(ActiveEvents::COLLISION_EVENTS);
        Self {
            sprite: ctx.create_sprite(include_bytes!("./img/burger.png")),
            model: ctx.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                Self::RESOLUTION,
                0.0,
            )),
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                vec![collider],
            ),
        }
    }
}

impl ComponentController for Player {
    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let delta = ctx.frame_time();
        let input = &mut ctx.shura.input;

        for player in &mut ctx.scene.component_manager.path_mut(&active) {
            let mut body = player.rigid_body_mut().unwrap();
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

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (_, mut renderer) = encoder.renderer(config);
        for (instances, player) in &ctx.path_render(&active) {
            renderer.render_sprite(&player.model, &player.sprite);
            renderer.commit(instances);
        }
    }

    fn collision(
        ctx: &mut Context,
        _self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if let Some(b) = ctx.component_mut::<PhysicsBox>(&other_handle) {
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
    #[component]
    component: BaseComponent,
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
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(Vector::new(0.0, -1.0)),
                vec![collider],
            ),
        }
    }
}

impl ComponentController for Floor {
    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let floors = ctx.path_render(&active);
        let (_, mut renderer) = encoder.renderer(config);
        for (instance, floor) in &floors {
            renderer.render_color(&floor.model, &floor.color);
            renderer.commit(instance);
        }
    }
}

#[derive(Component, serde::Serialize, serde::Deserialize)]
struct PhysicsBox {
    collided: bool,
    hovered: bool,
    #[component]
    component: BaseComponent,
}

impl PhysicsBox {
    pub fn new(position: Vector<f32>) -> Self {
        Self {
            collided: false,
            hovered: false,
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(position),
                vec![ColliderBuilder::new(SharedShape::new(
                    BoxManager::BOX_SHAPE,
                ))],
            ),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (_, mut renderer) = encoder.renderer(config);
        let manager = ctx
            .components::<BoxManager>(GroupFilter::All)
            .iter()
            .next()
            .unwrap();

        for (instance, physics_box) in &ctx.path_render(&active) {
            let color: &Uniform<Color>;
            if physics_box.collided {
                color = &manager.collision_color;
            } else if physics_box.hovered {
                color = &manager.hover_color;
            } else {
                color = &manager.default_color;
            }
            renderer.render_color(&manager.box_model, color);
            renderer.commit(instance);
        }
    }

    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let cursor_world: Point<f32> = (ctx.cursor_camera(&ctx.scene.world_camera)).into();
        let mut to_remove = vec![];
        let remove = ctx.is_held(MouseButton::Left) || ctx.is_pressed(ScreenTouch);
        for physics_box in &mut ctx.path_mut(&active) {
            let collider_handle = physics_box.collider_handles().unwrap()[0];
            let collider = physics_box.collider(collider_handle).unwrap();
            if collider
                .shape()
                .contains_point(collider.position(), &cursor_world)
            {
                drop(collider);
                physics_box.hovered = true;
                if remove {
                    to_remove.push(*physics_box.component.handle().unwrap());
                }
            } else {
                drop(collider);
                physics_box.hovered = false;
            }
        }

        for handle in to_remove {
            ctx.remove_component(&handle);
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
        let component: BaseComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Floor {
            component,
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
        let component: BaseComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(Player {
            component,
            sprite: self.ctx.create_sprite(include_bytes!("./img/burger.png")),
            model: self.ctx.create_model(ModelBuilder::from_collider_shape(
                &Player::SHAPE,
                Player::RESOLUTION,
                0.0,
            )),
        })
    }
}

struct BoxManagerVisitor<'a> {
    ctx: &'a Context<'a>,
}

impl<'de, 'a> serde::de::Visitor<'de> for BoxManagerVisitor<'a> {
    type Value = BoxManager;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A BoxManager")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<BoxManager, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let component: BaseComponent = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        Ok(BoxManager {
            component,
            default_color: self.ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: self.ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: self.ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: self.ctx.create_model(ModelBuilder::from_collider_shape(
                &BoxManager::BOX_SHAPE,
                0,
                0.0,
            )),
        })
    }
}
