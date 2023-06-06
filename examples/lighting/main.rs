use shura::physics::parry::query::PointQuery;
use shura::{physics::*, *};
use std::f32::consts::PI;

const TWO_PI_INV: f32 = 1.0 / (2.0 * PI);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.components.register::<Obstacle>();
            ctx.components.register::<Light>();

            let blend = BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::ReverseSubtract,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::ReverseSubtract,
                },
            };
            ctx.world_camera.set_scaling(WorldCameraScale::Max(10.0));
            // ctx.components.add(ctx.world, GroupHandle::DEFAULT_GROUP, Background::new(ctx));
            ctx.scene_states.insert(LightingState {
                light_layer: ctx.gpu.create_render_target(Vector::new(1, 1)),
                shadow_color: ctx.gpu.create_uniform(Color::BLACK),
                light_shader: ctx.gpu.create_shader(ShaderConfig {
                    fragment_source: include_str!("./light.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: BlendState::ALPHA_BLENDING,
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
                shadow_shader: ctx.gpu.create_shader(ShaderConfig {
                    fragment_source: include_str!("./light.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend,
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
                present_shader: ctx.gpu.create_shader(ShaderConfig {
                    fragment_source: include_str!("./present.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Sprite, ShaderField::Uniform],
                    blend: BlendState::ALPHA_BLENDING,
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
            });
            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Obstacle::new(
                    ctx.world,
                    ctx.gpu,
                    ColliderBuilder::cuboid(1.0, 1.0).translation(Vector::new(3.0, 3.0)),
                    Color::GREEN,
                ),
            );

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Obstacle::new(
                    ctx.world,
                    ctx.gpu,
                    ColliderBuilder::triangle(
                        Point::new(-1.5, 1.0),
                        Point::new(1.0, 1.5),
                        Point::new(1.5, 1.0),
                    )
                    .translation(Vector::new(-3.0, 2.5)),
                    Color::BLUE,
                ),
            );

            for i in 0..4 {
                ctx.components.add(
                    GroupHandle::DEFAULT_GROUP,
                    Obstacle::new(
                        ctx.world,
                        ctx.gpu,
                        ColliderBuilder::cuboid(0.04, 0.4)
                            .translation(Vector::new(-6.0, i as f32 * 1.0)),
                        Color::BLUE,
                    ),
                );
            }

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Obstacle::new(
                    ctx.world,
                    ctx.gpu,
                    ColliderBuilder::ball(1.0).translation(Vector::new(6.0, 0.0)),
                    Color::BLUE,
                ),
            );

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Obstacle::new(
                    ctx.world,
                    ctx.gpu,
                    ColliderBuilder::cuboid(0.5, 1.5).translation(Vector::new(-3.0, -3.0)),
                    Color::BLUE,
                ),
            );

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Obstacle::new(
                    ctx.world,
                    ctx.gpu,
                    ColliderBuilder::round_cuboid(0.5, 1.5, 0.4)
                        .translation(Vector::new(3.0, -3.0)),
                    Color::BLUE,
                ),
            );

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Light::new(
                    ctx.gpu,
                    Vector::new(0.0, 0.0),
                    7.0,
                    Color {
                        a: 1.0,
                        ..Color::RED
                    },
                    true,
                ),
            );

            ctx.components.add(
                GroupHandle::DEFAULT_GROUP,
                Light::new(
                    ctx.gpu,
                    Vector::new(0.0, 1.0),
                    5.0,
                    Color {
                        a: 1.0,
                        ..Color::GREEN
                    },
                    false,
                ),
            );
        },
    });
}

#[derive(State)]
struct LightingState {
    light_layer: RenderTarget,
    light_shader: Shader,
    shadow_shader: Shader,
    present_shader: Shader,
    shadow_color: Uniform<Color>,
}

impl SceneStateController for LightingState {
    fn update(ctx: &mut Context) {
        if *ctx.scene_resized {
            let state = ctx.scene_states.get_mut::<Self>();
            state.light_layer = ctx.gpu.create_render_target(ctx.window_size);
        }
    }
}

#[derive(Component)]
struct Obstacle {
    #[base]
    collider: ColliderComponent,
    model: Model,
    color: Uniform<Color>,
}

impl Obstacle {
    pub fn new(world: &mut World, gpu: &Gpu, collider: ColliderBuilder, color: Color) -> Self {
        Self {
            color: gpu.create_uniform(color),
            model: gpu.create_model(ModelBuilder::from_collider_shape(
                collider.shape.as_ref(),
                24,
                2.0,
            )),
            collider: ColliderComponent::new(world, collider),
        }
    }
}

impl ComponentController for Obstacle {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        update: UpdateOperation::Never,
        ..DEFAULT_CONFIG
    };

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::WORLD, |renderer, o, i| {
            renderer.render_color(i, &o.model, &o.color)
        });
    }
}

#[derive(Component)]
struct Light {
    #[base]
    base: PositionComponent,
    radius: f32,
    vertices: Vec<Vertex>,
    light_color: Uniform<Color>,
    light_model: Model,
    follow_mouse: bool,
    shape: Ball,
    shadows: Vec<Model>,
}

impl Light {
    const RESOLUTION: u32 = 64;
    pub fn new(
        gpu: &Gpu,
        position: Vector<f32>,
        radius: f32,
        color: Color,
        follow_cursor: bool,
    ) -> Self {
        let model_builder = ModelBuilder::ball(radius, Self::RESOLUTION);
        Self {
            radius,
            follow_mouse: follow_cursor,
            vertices: model_builder.vertices.clone(),
            light_model: gpu.create_model(model_builder),
            light_color: gpu.create_uniform(color),
            shape: Ball::new(radius),
            base: PositionComponent::new(PositionBuilder::new().translation(position)),
            shadows: vec![],
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };

    fn update(ctx: &mut Context) {
        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
            return v1.x * v2.y - v1.y * v2.x;
        }
        let cursor_pos = ctx.input.cursor(&ctx.world_camera);
        for light in ctx.components.iter_mut::<Self>(ComponentFilter::Active) {
            if light.follow_mouse {
                light.base.set_translation(cursor_pos);
            }

            let light_position = light.base.position();
            let light_translation = light.base.translation();
            light.shadows.clear();
            ctx.world.intersections_with_shape(
                &light_position,
                &light.shape,
                QueryFilter::new(),
                |_component, collider_handle| {
                    let obstacle_collider = ctx.world.collider(collider_handle).unwrap();
                    let obstacle_shape = obstacle_collider.shape();
                    let collider_vertices =
                        ModelBuilder::from_collider_shape(obstacle_shape, 24, 2.0)
                            .vertex_position(*obstacle_collider.position())
                            .apply_modifiers()
                            .vertices;

                    let mut leftmost = collider_vertices[0].pos;
                    let mut rightmost = collider_vertices[0].pos;
                    let mut leftmost_ray = leftmost - light_translation;
                    let mut rightmost_ray = rightmost - light_translation;

                    let mut ray;
                    for v in &collider_vertices[1..] {
                        ray = v.pos - light_translation;
                        if det(ray, leftmost_ray) < 0.0 {
                            leftmost = v.pos;
                            leftmost_ray = ray;
                        } else if det(ray, rightmost_ray) > 0.0 {
                            rightmost = v.pos;
                            rightmost_ray = ray;
                        }
                    }

                    let mut leftback = light_translation + leftmost_ray.normalize() * light.radius;

                    let mut rightback =
                        light_translation + rightmost_ray.normalize() * light.radius;

                    let mut ray_angle: f32 = rightmost_ray.y.atan2(rightmost_ray.x);
                    if ray_angle < 0.0 {
                        ray_angle += 2.0 * PI;
                    }
                    let right_index = (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32) as usize
                        % Self::RESOLUTION as usize;
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
                    let left_index = (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32) as usize
                        as usize
                        % Self::RESOLUTION as usize;
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
                    if light
                        .shape
                        .contains_point(&light_position, &rightmost.into())
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
                    if light
                        .shape
                        .contains_point(&light_position, &leftmost.into())
                    {
                        vertices.push(leftmost);
                    }

                    if vertices.len() >= 3 {
                        let mut builder = ModelBuilder::convex_polygon(vertices)
                            .vertex_translation(-light_translation);

                        let diameter = 2.0 * light.radius;
                        for vertex in &mut builder.vertices {
                            let rel = vertex.pos - light_translation;
                            vertex.tex_coords =
                                Vector::new(rel.x / diameter + 0.5, rel.y / -diameter + 0.5);
                        }
                        light.shadows.push(ctx.gpu.create_model(builder));
                    }
                    true
                },
            );
        }
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let state = ctx.scene_states.get::<LightingState>();

        {
            let mut renderer = encoder.renderer(RenderConfig {
                target: RenderConfigTarget::Custom(&state.light_layer),
                clear_color: Some(Color::TRANSPARENT),
                ..RenderConfig::WORLD
            });
            for (buffer, lights) in ctx.components.iter_render::<Self>(ComponentFilter::Active) {
                renderer.use_instances(buffer);
                for (i, light) in lights {
                    renderer.use_shader(&state.light_shader);
                    renderer.use_model(&light.light_model);
                    renderer.use_uniform(&light.light_color, 1);
                    renderer.draw(i);

                    for shadow in &light.shadows {
                        renderer.use_shader(&state.shadow_shader);
                        renderer.use_model(shadow);
                        renderer.draw(i);
                    }
                }
            }
        }

        let mut renderer = encoder.renderer(RenderConfig::RELATIVE_WORLD);
        renderer.use_instances(&ctx.defaults.single_centered_instance);
        renderer.use_shader(&state.present_shader);
        renderer.use_model(&ctx.defaults.relative_camera.0.model());
        renderer.use_sprite(&state.light_layer, 1);
        renderer.use_uniform(&state.shadow_color, 2);
        renderer.draw(0);
    }
}
