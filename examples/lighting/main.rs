use shura::{physics::*, *};
use std::f32::consts::PI;

const TWO_PI_INV: f32 = 1.0 / (2.0 * PI);

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_camera_vertical_fov(10.0);
            ctx.set_global_state(LightingState {
                shadow_color: ctx.create_uniform(Color::BLACK),
                inner_model: ctx.create_model(ModelBuilder::ball(0.5, 24)),
                light_shader: ctx.create_shader(ShaderConfig {
                    fragment_source: include_str!("./light.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: BlendState::ALPHA_BLENDING,
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
                shadow_shader: ctx.create_shader(ShaderConfig {
                    fragment_source: include_str!("./shadow.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: BlendState::ALPHA_BLENDING,
                    // blend: BlendState {
                    //     color: BlendComponent {
                    //         src_factor: BlendFactor::SrcAlpha,
                    //         dst_factor: BlendFactor::OneMinusSrcAlpha,
                    //         operation: BlendOperation::Subtract,
                    //     },
                    //     alpha: BlendComponent::OVER,
                    // },
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
            });
            ctx.add_component(Obstacle::new(
                ctx,
                Vector::new(3.0, 3.0),
                ColliderBuilder::cuboid(1.0, 1.0),
                Color::GREEN,
            ));

            ctx.add_component(Obstacle::new(
                ctx,
                Vector::new(-3.0, 2.5),
                ColliderBuilder::triangle(
                    Point::new(-1.5, 1.0),
                    Point::new(1.0, 1.5),
                    Point::new(1.5, 1.0),
                ),
                Color::BLUE,
            ));
            ctx.add_component(Obstacle::new(
                ctx,
                Vector::new(-3.0, -3.0),
                ColliderBuilder::cuboid(0.5, 1.5),
                Color::BLUE,
            ));
            ctx.add_component(Obstacle::new(
                ctx,
                Vector::new(3.0, -3.0),
                ColliderBuilder::round_cuboid(0.5, 1.5, 0.4),
                Color::BLUE,
            ));
            ctx.add_component(Light::new(
                ctx,
                Vector::new(0.0, 0.0),
                12.0,
                Color::RED,
                true,
            ));
            // ctx.add_component(Light::new(
            //     ctx,
            //     Vector::new(0.0, 1.0),
            //     10.0,
            //     Color::GREEN,
            //     false,
            // ));
        },
    });
}

impl GlobalState for LightingState {}
struct LightingState {
    light_shader: Shader,
    shadow_shader: Shader,
    shadow_color: Uniform<Color>,
    inner_model: Model,
}

#[derive(Component)]
struct Obstacle {
    #[base]
    base: BaseComponent,
    model: Model,
    color: Uniform<Color>,
}

impl Obstacle {
    pub fn new(
        ctx: &Context,
        position: Vector<f32>,
        collider: ColliderBuilder,
        color: Color,
    ) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                24,
                2.0,
            )),
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::fixed().translation(position),
                vec![collider],
            ),
            color: ctx.create_uniform(color),
        }
    }
}

impl ComponentController for Obstacle {
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
        let (_, mut renderer) = encoder.renderer(&config);
        for (i, b) in ctx.path_render(&active).iter() {
            renderer.render_color(i, &b.model, &b.color);
        }
    }
}

#[derive(Component)]
struct Light {
    #[base]
    base: BaseComponent,
    radius: f32,
    vertices: Vec<Vertex>,
    shadows: Vec<Model>,
    light_color: Uniform<Color>,
    light_model: Model,
    follow_mouse: bool,
}

impl Light {
    const RESOLUTION: u32 = 64;
    const IS_LIGHT_COLLIDER: u128 = 1000;
    pub fn new(
        ctx: &Context,
        position: Vector<f32>,
        radius: f32,
        color: Color,
        follow_cursor: bool,
    ) -> Self {
        let model_builder = ModelBuilder::ball(radius, Self::RESOLUTION);
        Self {
            radius,
            follow_mouse: follow_cursor,
            shadows: vec![],
            vertices: model_builder.vertices.clone(),
            base: BaseComponent::new_rigid_body(
                RigidBodyBuilder::dynamic().translation(position),
                vec![ColliderBuilder::ball(radius)
                    .sensor(true)
                    .user_data(Self::IS_LIGHT_COLLIDER)],
            ),
            light_model: ctx.create_model(model_builder),
            light_color: ctx.create_uniform(color),
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };

    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
            return v1.x * v2.y - v1.y * v2.x;
        }

        let cursor_pos = ctx.cursor_camera(&ctx.world_camera);
        for light in ctx.path_mut(&active).iter() {
            if light.follow_mouse {
                light.base.set_translation(cursor_pos);
            }
        }

        let mut all_shadows = vec![];
        for light in ctx.path(&active).iter() {
            let mut shadows = vec![];
            let light_collider_handle = light.base.collider_handles().unwrap()[0];
            let light_collider = light.base.collider(light_collider_handle).unwrap();
            let light_translation = light.base.translation();

            ctx.intersections_with_shape(
                light_collider.position(),
                light_collider.shape(),
                QueryFilter::new(),
                |_component, collider_handle| {
                    let obstacle_collider = ctx.collider(collider_handle).unwrap();
                    if obstacle_collider.user_data != Self::IS_LIGHT_COLLIDER {
                        let obstacle_shape = obstacle_collider.shape();
                        let mut model = ModelBuilder::from_collider_shape(obstacle_shape, 24, 2.0)
                            .vertex_position(*obstacle_collider.position());
                        model.apply_modifiers();
                        let vertices = model.vertices;

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

                        let mut leftback =
                            light_translation + leftmost_ray.normalize() * light.radius;

                        let mut rightback =
                            light_translation + rightmost_ray.normalize() * light.radius;

                        let mut ray_angle: f32 = rightmost_ray.y.atan2(rightmost_ray.x);
                        if ray_angle < 0.0 {
                            ray_angle += 2.0 * PI;
                        }
                        let right_index =
                            (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32) as usize;
                        let v0 = light.vertices[right_index].pos;
                        let v0_to_v1 =
                            light.vertices[(right_index + 1) % Self::RESOLUTION as usize].pos - v0;
                        let alpha = rightmost_ray.angle(&(-v0_to_v1));
                        let v0_to_rightback = rightback - (light_translation + v0);
                        let beta = v0_to_rightback.angle(&v0_to_v1);
                        let gamma = PI - alpha - beta;
                        let c = gamma.sin() / alpha.sin() * v0_to_rightback.norm();
                        rightback = light_translation + v0 + c * v0_to_v1.normalize();

                        let mut ray_angle: f32 = leftmost_ray.y.atan2(leftmost_ray.x);
                        if ray_angle < 0.0 {
                            ray_angle += 2.0 * PI;
                        }
                        let left_index =
                            (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32) as usize;
                        let v0 = light.vertices[left_index].pos;
                        let v0_to_v1 =
                            light.vertices[(left_index + 1) % Self::RESOLUTION as usize].pos - v0;
                        let alpha = leftmost_ray.angle(&(-v0_to_v1));
                        let v0_to_leftback = leftback - (light_translation + v0);
                        let beta = v0_to_leftback.angle(&v0_to_v1);
                        let gamma = PI - alpha - beta;
                        let c = gamma.sin() / alpha.sin() * v0_to_leftback.norm();
                        leftback = light_translation + v0 + c * v0_to_v1.normalize();

                        let mut vertices = vec![];
                        if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &rightmost.into())
                        {
                            vertices.push(rightmost);
                        }
                        vertices.push(rightback);
                        let end = (left_index + 1) % (Self::RESOLUTION as usize);
                        let mut i = (right_index + 1) % (Self::RESOLUTION as usize);
                        while i != end {
                            vertices.push(light_translation + light.vertices[i].pos);
                            i = (i + 1) % (Self::RESOLUTION as usize);
                        }

                        vertices.push(leftback);
                        if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &leftmost.into())
                        {
                            vertices.push(leftmost);
                        }

                        let mid = (leftmost + rightmost) / 2.0;
                        let delta = mid - light_translation;
                        let angle = delta.y.atan2(delta.x) - 90.0_f32.to_radians();
                        let rotation = Rotation::new(angle);
                        shadows.push(
                            ctx.create_model(
                                ModelBuilder::convex_polygon(vertices)
                                    .vertex_translation(-light_translation)
                                    .tex_coord_rotation(rotation),
                            ),
                        );
                    }
                    true
                },
            );
            all_shadows.push(shadows);
        }

        for (light, shadows) in ctx.path_mut(&active).iter().zip(all_shadows.into_iter()) {
            light.shadows = shadows;
        }
    }

    fn render<'a>(
        active: ComponentPath<Self>,
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        let (_, mut renderer) = encoder.renderer(&config);
        let state = ctx.global_state::<LightingState>().unwrap();
        renderer.use_shader(&state.light_shader);
        for (i, l) in ctx.path_render(&active).iter() {
            renderer.use_model(&l.light_model);
            renderer.use_uniform(&l.light_color, 1);
            renderer.draw(i);
        }

        for (i, l) in ctx.path_render(&active).iter() {
            renderer.use_shader(&state.shadow_shader);
            for shadow in &l.shadows {
                renderer.use_model(shadow);
                renderer.use_uniform(&state.shadow_color, 1);
                renderer.draw(i);
            }

            renderer.render_color(i, &state.inner_model, &l.light_color);
        }
    }
}
