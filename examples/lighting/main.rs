use shura::{physics::*, *};

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_camera_vertical_fov(20.0);
            ctx.create_component(Box::new(
                ctx,
                Vector::new(2.0, 2.0),
                Vector::new(0.5, 0.5),
                Color::GREEN,
            ));
            // ctx.create_component(Box::new(
            //     ctx,
            //     Vector::new(-2.0, -2.0),
            //     Vector::new(0.5, 0.5),
            //     Color::BLUE,
            // ));
            ctx.create_component(Light::new(ctx, Vector::new(2.0, 2.0), 4.0, Color::WHITE));
        },
    });
}


#[derive(Component)]
struct Box {
    #[component]
    component: BaseComponent,
    model: Model,
    color: Uniform<Color>,
}

impl Box {
    pub fn new(ctx: &Context, position: Vector<f32>, size: Vector<f32>, color: Color) -> Self {
        Self {
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(position),
                vec![ColliderBuilder::cuboid(size.x, size.y)],
            ),
            model: ctx.create_model(ModelBuilder::cuboid(size)),
            color: ctx.create_uniform(color),
        }
    }
}

impl ComponentController for Box {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        update: UpdateOperation::Never,
        ..DEFAULT_CONFIG
    };

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (_, mut renderer) = encoder.renderer(config);
        for (i, b) in ctx.path_render(&active).iter() {
            renderer.render_color(&b.model, &b.color);
            renderer.commit(i);
        }
    }
}

#[derive(Component)]
struct Light {
    #[component]
    component: BaseComponent,
    radius: f32,
    model: Model,
    color: Uniform<Color>,
}

impl Light {
    const RESOLUTION: u32 = 64;
    pub fn new(ctx: &Context, position: Vector<f32>, radius: f32, color: Color) -> Self {
        Self {
            radius,
            component: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(position),
                vec![ColliderBuilder::ball(radius).sensor(true)],
            ),
            model: ctx.create_model(ModelBuilder::ball(radius, Self::RESOLUTION)),
            color: ctx.create_uniform(color)
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };

    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let cursor_pos = *ctx.cursor_world();
        for light in ctx.path_mut(&active).iter() {
            light.component.set_translation(cursor_pos);
        }

        for light in ctx.path(&active).iter() {
            let light_collider_handle = light.component.collider_handles().unwrap()[0];
            let light_collider = light.component.collider(light_collider_handle).unwrap();
            let light_translation = *light_collider.translation();

            ctx.intersections_with_shape(
                light_collider.position(),
                light_collider.shape(),
                QueryFilter::new(),
                |_component, collider_handle| {
                    if collider_handle != light_collider_handle {
                        let box_collider = ctx.collider(collider_handle).unwrap();
                        let box_shape = box_collider.shape().downcast_ref::<Cuboid>().unwrap();

                        let mut model = ModelBuilder::cuboid(box_shape.half_extents)
                            .vertex_position(*box_collider.position());
                        model.apply_modifiers();
                        let vertices = model.vertices;

                        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
                            return v1.x * v2.y - v1.y * v2.x;
                        }

                        let mut leftmost = vertices[0].pos;
                        let mut rightmost = vertices[0].pos;
                        let mut leftmost_ray = leftmost - light_translation;
                        let mut rightmost_ray = rightmost - light_translation;

                        let mut ray;
                        for v in &vertices[1..] {
                            ray = v.pos - light_translation;
                            if det(ray, leftmost_ray) < 0.0 {
                                leftmost = v.pos;
                                leftmost_ray = ray;
                            } else if det(ray, rightmost_ray) > 0.0 {
                                rightmost = v.pos;
                                rightmost_ray = ray;
                            }
                        }

                        let leftback = light_translation + leftmost_ray.normalize() * light.radius;
                        let rightback = light_translation + rightmost_ray.normalize() * light.radius;

                    }
                    true
                },
            )
        }
    }

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (_, mut renderer) = encoder.renderer(config);
        for (i, l) in ctx.path_render(&active).iter() {
            renderer.render_color(&l.model, &l.color);
            renderer.commit(i);
        }
    }
}

// #[derive(Component)]
// struct LightMap {
//     #[component]
//     component: BaseComponent,
//     map: RenderTarget,
// }

// impl ComponentController for LightMap {}
