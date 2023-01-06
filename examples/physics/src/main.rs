#![windows_subsystem = "windows"]

use shura::physics::*;
use shura::*;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    shura::init("Physics", |ctx| {
        const PYRAMID_ELEMENTS: i32 = 8;
        const MINIMAL_SPACING: f32 = 0.1;
        ctx.set_horizontal_fov(10.0);
        ctx.set_gravity(Vector::new(0.00, -9.81));

        for x in -PYRAMID_ELEMENTS..PYRAMID_ELEMENTS {
            for y in 0..(PYRAMID_ELEMENTS - x.abs()) {
                ctx.create_component(
                    None,
                    PhysicsBox::new(Vector::new(
                        x as f32 * (BOX_SIZE*2.0 + MINIMAL_SPACING),
                        y as f32 * (BOX_SIZE*2.0 + MINIMAL_SPACING*2.0),
                    )),
                );
            }
        }

        let (_, player_handle) = ctx.create_component(None, Player::new(ctx));
        ctx.set_camera_target(Some(player_handle));
        ctx.create_component(None, Floor::new(ctx));

        GameScene {
            default_color: ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            collision_color: ctx.create_uniform(Color::new_rgba(255, 0, 0, 255)),
            hover_color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            box_model: ctx.create_model(ModelBuilder::cuboid(Dimension::new(BOX_SIZE, BOX_SIZE))),
        }
    });
}

const BOX_SIZE: f32 = 0.3;

struct GameScene {
    default_color: Uniform<Color>,
    collision_color: Uniform<Color>,
    hover_color: Uniform<Color>,
    box_model: Model,
}

impl SceneController for GameScene {
    fn update(&mut self, ctx: &mut Context) {
        let scroll = ctx.wheel_delta();
        let fov = ctx.camera_fov();
        if scroll != 0.0 {
            ctx.set_horizontal_fov(fov.width + scroll);
        }

        if ctx.is_pressed(MouseButton::Right) {
            ctx.create_component(None, PhysicsBox::new(*ctx.cursor_world()));
        }
    }
}

#[derive(Component)]
struct Player {
    sprite: Sprite,
    model: Model,
    #[component]
    component: PhysicsComponent,
}

impl Player {
    pub fn new(ctx: &Context) -> Self {
        let radius = 0.75;
        Self {
            sprite: ctx.create_sprite(include_bytes!("../img/burger.png")),
            model: ctx.create_model(ModelBuilder::ball(radius, 24)),
            component: PhysicsComponent::new(
                RigidBodyBuilder::dynamic().translation(Vector::new(5.0, 4.0)),
                vec![ColliderBuilder::ball(radius).active_events(ActiveEvents::COLLISION_EVENTS)],
            ),
        }
    }
}

impl ComponentController for Player {
    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        let delta = ctx.delta_time();
        let body = self.component.body_mut(ctx.world);
        let mut linvel = *body.linvel();

        if ctx.input.is_held(Key::D) {
            linvel.x += 15.0 * delta;
        }

        if ctx.input.is_held(Key::A) {
            linvel.x += -15.0 * delta;
        }

        if ctx.input.is_pressed(Key::W) {
            linvel.y = 7.0;
        }

        if ctx.input.is_pressed(Key::S) {
            linvel.y = -17.0;
        }

        body.set_linvel(linvel, true);
    }

    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        renderer.render_sprite(&self.model, &self.sprite);
        renderer.commit(&instance);
    }

    fn collision(
        &mut self,
        _scene: &mut DynamicScene,
        ctx: &mut Context,
        other: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
        if let Some(b) = ctx.component_mut::<PhysicsBox>(&other) {
            b.collided = collide_type == CollideType::Started;
        }
    }
}

#[derive(Component)]
struct Floor {
    color: Uniform<Color>,
    model: Model,
    #[component]
    component: PhysicsComponent,
}

impl Floor {
    pub fn new(ctx: &Context) -> Self {
        let size = Dimension::new(20.0, 0.4);
        Self {
            color: ctx.create_uniform(Color::new_rgba(0, 0, 255, 255)),
            model: ctx.create_model(ModelBuilder::cuboid(size)),
            component: PhysicsComponent::new(
                RigidBodyBuilder::fixed().translation(Vector::new(0.0, -1.0)),
                vec![ColliderBuilder::cuboid(size.width, size.height)],
            ),
        }
    }
}

impl ComponentController for Floor {
    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        renderer.render_color(&self.model, &self.color);
        renderer.commit(&instance);
    }
}

#[derive(Component)]
struct PhysicsBox {
    collided: bool,
    hovered: bool,
    #[component]
    component: PhysicsComponent,
}

impl PhysicsBox {
    pub fn new(position: Vector<f32>) -> Self {
        Self {
            collided: false,
            hovered: false,
            component: PhysicsComponent::new(
                RigidBodyBuilder::dynamic().translation(position),
                vec![ColliderBuilder::cuboid(BOX_SIZE, BOX_SIZE)],
            ),
        }
    }
}

impl ComponentController for PhysicsBox {
    fn render<'a>(
        &'a self,
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        let scene = scene.downcast_ref::<GameScene>().unwrap();
        let color: &Uniform<Color>;
        if self.collided {
            color = &scene.collision_color;
        } else if self.hovered {
            color = &scene.hover_color;
        } else {
            color = &scene.default_color;
        }
        renderer.render_color(&scene.box_model, color);
        renderer.commit(&instance);
    }

    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        if ctx.intersects_point(self.component.collider_handles(ctx.world)[0], *ctx.cursor_world()) {
            self.hovered = true;
            if ctx.is_pressed(MouseButton::Left) || ctx.is_pressed(ScreenTouch) {
                ctx.remove_component(self.component.handle());
            }
        } else {
            self.hovered = false;
        }
    }
}
