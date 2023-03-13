use nalgebra::distance;
use shura::{physics::*, *};

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_camera_vertical_fov(20.0);
            ctx.set_global_state(GameState {
                shadow_color: ctx.create_uniform(Color::BLACK),
                inner_model: ctx.create_model(ModelBuilder::ball(0.5, 24)),
                light_shader: ctx.create_shader(ShaderConfig {
                    fragment_source: include_str!("./shader.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: true,
                    smaa: true,
                    write_mask: ColorWrites::ALL
                }),
                test_shader: ctx.create_shader(ShaderConfig {
                    fragment_source: include_str!("./test.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: true,
                    smaa: true,
                    write_mask: ColorWrites::ALL
                }),
            });
            ctx.create_component(Obstacle::new(
                ctx,
                Vector::new(3.0, 3.0),
                ColliderBuilder::cuboid(1.0, 1.0),
                Color::GREEN,
            ));

            ctx.create_component(Obstacle::new(
                ctx,
                Vector::new(-3.0, 2.5),
                ColliderBuilder::triangle(
                    Point::new(-1.5, 1.0),
                    Point::new(1.0, 1.5),
                    Point::new(1.5, 1.0),
                ),
                Color::RED,
            ));
            ctx.create_component(Obstacle::new(
                ctx,
                Vector::new(-3.0, -3.0),
                ColliderBuilder::cuboid(0.5, 1.5),
                Color::BLUE,
            ));
            ctx.create_component(Obstacle::new(
                ctx,
                Vector::new(3.0, -3.0),
                ColliderBuilder::round_cuboid(0.5, 1.5, 0.4),
                Color::BLUE,
            ));
            ctx.create_component(Light::new(
                ctx,
                Vector::new(0.0, 0.0),
                15.0,
                Color::RED,
                true,
            ));
            ctx.create_component(Light::new(
                ctx,
                Vector::new(0.0, 1.0),
                10.0,
                Color::GREEN,
                false,
            ));

            // ctx.create_component(Light::new(
            //     ctx,
            //     Vector::new(1.5, 0.0),
            //     2.0,
            //     Color::WHITE,
            //     false,
            // ));
        },
    });
}

struct GameState {
    light_shader: Shader,
    test_shader: Shader,
    shadow_color: Uniform<Color>,
    inner_model: Model,
}

#[derive(Component)]
struct Obstacle {
    #[component]
    component: BaseComponent,
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
            component: BaseComponent::new_rigid_body(
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
            component: BaseComponent::new_rigid_body(
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
        fn rotate_point_around_origin(
            origin: Vector<f32>,
            delta: Vector<f32>,
            rot: Rotation<f32>,
        ) -> Vector<f32> {
            let sin = rot.sin_angle();
            let cos = rot.cos_angle();
            return Vector::new(
                origin.x + (delta.x) * cos - (delta.y) * sin,
                origin.y + (delta.x) * sin + (delta.y) * cos,
            );
        }

        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
            return v1.x * v2.y - v1.y * v2.x;
        }

        let cursor_pos = *ctx.cursor_world();
        for light in ctx.path_mut(&active).iter() {
            if light.follow_mouse {
                light.component.set_translation(cursor_pos);
            }
        }

        let mut all_shadows = vec![];
        for light in ctx.path(&active).iter() {
            let mut shadows = vec![];
            let light_collider_handle = light.component.collider_handles().unwrap()[0];
            let light_collider = light.component.collider(light_collider_handle).unwrap();
            let light_translation = light.component.translation();

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

                        let leftback = if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &leftmost.into())
                        {
                            light_translation + leftmost_ray.normalize() * light.radius
                        } else {
                            leftmost
                        };

                        let rightback = if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &rightmost.into())
                        {
                            light_translation + rightmost_ray.normalize() * light.radius
                        } else {
                            rightmost
                        };

                        struct EdgeData {
                            index: usize,
                            distance: f32,
                        }

                        // let mut closest_circle_left = (0, f32::MAX);
                        // let mut closest_circle_right = (0, f32::MAX);

                        // let delta = leftback - light_translation;
                        // let angle = delta.y.atan2(delta.x);
                        // println!("{}", angle.to_degrees());
                        // let rot = Rotation::new(-angle);
                        // let leftback_rotated =
                        //     rotate_point_around_origin(light_translation, delta, rot);
                        // for (i, v) in light.vertices.iter().enumerate() {
                        //     let rotated_vertex =
                        //         rotate_point_around_origin(light_translation, v.pos, rot);
                        //     if rotated_vertex.y < leftback_rotated.y {
                        //         let distance = distance(
                        //             &(v.pos + light_translation).into(),
                        //             &leftback.into(),
                        //         );
                        //         if distance < closest_circle_left.1 {
                        //             closest_circle_left = (i, distance);
                        //         }
                        //     }

                        //     // if t1 -
                        //     // let distance_left = distance(&(v.pos + light_translation).into(), &leftback.into());
                        //     // let distance_right = distance(&(v.pos + light_translation).into(), &rightback.into());
                        //     // if distance_left < closest_circle_left.1 {
                        //     //     closest_circle_left = (i, distance_left);
                        //     // }

                        //     // if distance_right < closest_circle_left.1 {
                        //     //     closest_circle_right = (i, distance_right);
                        //     // }
                        // }

                        let mut vertices = vec![leftmost];
                        if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &leftmost.into())
                        {
                            vertices.push(leftback);
                        }

                        // vertices.push(light.vertices[closest_circle_left.0].pos);
                        // vertices.push(light.vertices[closest_circle_right.0].pos);
                        // for i in closest_circle_right.0..closest_circle_left.0 {
                        //     vertices.push(light.vertices[i].pos + light_translation);
                        // }

                        if light_collider
                            .shape()
                            .contains_point(light_collider.position(), &rightmost.into())
                        {
                            vertices.push(rightback);
                        }
                        vertices.push(rightmost);

                        shadows.push(
                            ctx.create_model(
                                ModelBuilder::convex_polygon(vertices)
                                    .vertex_translation(-light_translation)
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
        let (_, mut renderer) = encoder.renderer(config);
        let state = ctx.global_state::<GameState>().unwrap();
        renderer.use_shader(&state.light_shader);
        for (i, l) in ctx.path_render(&active).iter() {
            renderer.use_model(&l.light_model);
            renderer.use_uniform(&l.light_color, 1);
            renderer.commit(i);
        }

        for (i, l) in ctx.path_render(&active).iter() {
            renderer.use_shader(&renderer.defaults.color);
            for shadow in &l.shadows {
                renderer.use_model(shadow);
                renderer.use_uniform(&state.shadow_color, 1);
                renderer.commit(i);
            }

            renderer.render_color(&state.inner_model, &l.light_color);
            renderer.commit(i);
        }
    }
}
