#![windows_subsystem = "windows"]

use shura::physics::*;
use shura::*;
use std::{fmt, fs};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
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
                ctx.set_horizontal_fov(10.0);
                ctx.set_gravity(Vector::new(0.00, -9.81));

                for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
                    for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                        ctx.create_component(
                            None,
                            PhysicsBox::new(Vector::new(
                                x as f32 * (BoxManager::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING),
                                y as f32
                                    * (BoxManager::HALF_BOX_SIZE * 2.0 + MINIMAL_SPACING * 2.0),
                            )),
                        );
                    }
                }

                let (_, player_handle) = ctx.create_component(None, Player::new(ctx));
                ctx.set_camera_target(Some(player_handle));
                ctx.create_component(None, Floor::new(ctx));
                ctx.create_component(None, BoxManager::new(ctx));
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
    pub fn new(ctx: &Context) -> Self {
        Self {
            default_color: ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: ctx.create_model(ModelBuilder::cuboid(Dimension::new(
                Self::HALF_BOX_SIZE,
                Self::HALF_BOX_SIZE,
            ))),
            component: Default::default(),
        }
    }
}

impl ComponentController for BoxManager {
    fn update(components: ActiveComponents<Self>, ctx: &mut Context) {
        let scroll = ctx.wheel_delta();
        let fov = ctx.camera_fov();
        if scroll != 0.0 {
            ctx.set_horizontal_fov(fov.width + scroll);
        }

        if ctx.is_held(MouseButton::Right) {
            let cursor = *ctx.cursor_world();
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
                ctx.create_component(None, PhysicsBox::new(cursor));
            }
        }

        if ctx.is_pressed(Key::Z) {
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

    fn config() -> ComponentConfig {
        ComponentConfig {
            priority: 1,
            render: RenderOperation::None,
            ..ComponentConfig::default()
        }
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
    pub fn new(ctx: &Context) -> Self {
        Self {
            sprite: ctx.create_sprite(include_bytes!("../img/burger.png")),
            model: ctx.create_model(ModelBuilder::ball(Self::RADIUS, 24)),
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                vec![ColliderBuilder::ball(Self::RADIUS)
                    .active_events(ActiveEvents::COLLISION_EVENTS)],
            ),
        }
    }
}

impl ComponentController for Player {
    fn update(components: ActiveComponents<Self>, ctx: &mut Context) {
        let delta = ctx.frame_time();
        let world = &mut ctx.scene.world;
        let input = &mut ctx.shura.input;

        for player in &mut ctx
            .scene
            .component_manager
            .active_components_mut(&components)
        {
            let body = player.rigid_body_mut(world).unwrap();
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
        components: ActiveComponents<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        _instances: Instances,
    ) {
        let test = ctx.active_components_render(&components);
        for (instances, player) in &test {
            renderer.render_sprite(&player.model, &player.sprite);
            renderer.commit(instances);
        }
    }

    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
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
    const FLOOR_SIZE: Dimension<f32> = Dimension::new(20.0, 0.4);
    pub fn new(ctx: &Context) -> Self {
        Self {
            color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            model: ctx.create_model(ModelBuilder::cuboid(Self::FLOOR_SIZE)),
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(Vector::new(0.0, -1.0)),
                vec![ColliderBuilder::cuboid(
                    Self::FLOOR_SIZE.width,
                    Self::FLOOR_SIZE.height,
                )],
            ),
        }
    }
}

impl ComponentController for Floor {
    fn render<'a>(
        components: ActiveComponents<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
    ) {
        for (instance, floor) in &ctx.active_components_render(&components) {
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
                RigidBodyBuilder::fixed().translation(position),
                vec![ColliderBuilder::cuboid(
                    BoxManager::HALF_BOX_SIZE,
                    BoxManager::HALF_BOX_SIZE,
                )],
            ),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render<'a>(
        components: ActiveComponents<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        _instances: Instances,
    ) {
        let manager = ctx
            .components::<BoxManager>(GroupFilter::All)
            .iter()
            .next()
            .unwrap();

        for (instance, physics_box) in &ctx.active_components_render(&components) {
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

    fn update(components: ActiveComponents<Self>, ctx: &mut Context) {
        let cursor_world = *ctx.cursor_world();
        let world = &mut ctx.scene.world;
        let cm = &mut ctx.scene.component_manager;
        let input = &mut ctx.shura.input;
        let mut to_remove = vec![];
        let mut e = cm.active_components_mut(&components);
        for physics_box in &mut e {
            if world.intersects_point(
                physics_box
                    .component
                    .collider_handles(&world)
                    .unwrap()[0],
                cursor_world,
            ) {
                physics_box.hovered = true;
                if input.is_held(MouseButton::Left) || input.is_pressed(ScreenTouch) {
                    to_remove.push(*physics_box.component.handle());
                }
            } else {
                physics_box.hovered = false;
            }
        }
        
        for handle in to_remove {
            cm.remove_component(&handle, world);
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
            model: self
                .ctx
                .create_model(ModelBuilder::cuboid(Floor::FLOOR_SIZE)),
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
            sprite: self.ctx.create_sprite(include_bytes!("../img/burger.png")),
            model: self
                .ctx
                .create_model(ModelBuilder::ball(Player::RADIUS, 24)),
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
            box_model: self.ctx.create_model(ModelBuilder::cuboid(Dimension::new(
                BoxManager::HALF_BOX_SIZE,
                BoxManager::HALF_BOX_SIZE,
            ))),
        })
    }
}
