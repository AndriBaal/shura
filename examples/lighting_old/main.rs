use shura::physics::parry::query::PointQuery;
use shura::{physics::*, *};
use std::f32::consts::PI;

const TWO_PI_INV: f32 = 1.0 / (2.0 * PI);

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_camera_scale(WorldCameraScale::Max(10.0));
            // ctx.add_component(Background::new(ctx));
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
                    fragment_source: Shader::COLOR_WGSL,
                    shader_lang: ShaderLang::WGSL,
                    shader_fields: &[ShaderField::Uniform],
                    blend: BlendState::REPLACE,
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
                blend_additive: ctx.create_shader(ShaderConfig {
                    fragment_source: Shader::SPIRTE_WGSL,
                    shader_lang: ShaderLang::WGSL,
                    shader_fields: &[ShaderField::Sprite],
                    blend: BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    },
                    msaa: true,
                    write_mask: ColorWrites::ALL,
                }),
                present_map: ctx.create_shader(ShaderConfig {
                    fragment_source: include_str!("./present_map.glsl"),
                    shader_lang: ShaderLang::GLSL,
                    shader_fields: &[ShaderField::Sprite, ShaderField::Uniform],
                    blend: BlendState::ALPHA_BLENDING,
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

            for i in 0..4 {
                ctx.add_component(Obstacle::new(
                    ctx,
                    Vector::new(-6.0, i as f32 * 1.0),
                    ColliderBuilder::cuboid(0.04, 0.4),
                    Color::BLUE,
                ));
            }

            ctx.add_component(Obstacle::new(
                ctx,
                Vector::new(6.0, 0.0),
                ColliderBuilder::ball(1.0),
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
                6.0,
                Color {
                    a: 0.3,
                    ..Color::RED
                },
                true,
            ));
            ctx.add_component(Light::new(
                ctx,
                Vector::new(0.0, 1.0),
                10.0,
                Color {
                    a: 0.3,
                    ..Color::GREEN
                },
                false,
            ));
        },
    });
}

impl GlobalState for LightingState {}
struct LightingState {
    light_shader: Shader,
    shadow_shader: Shader,
    blend_additive: Shader,
    present_map: Shader,
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

    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::WORLD);
        for (buffer, obstacles) in ctx.path_render(&active) {
            for (i, b) in obstacles {
                renderer.render_color(buffer, i, &b.model, &b.color)
            }
        }
    }
}

#[derive(Component)]
struct Light {
    #[base]
    base: BaseComponent,
    radius: f32,
    vertices: Vec<Vertex>,
    light_color: Uniform<Color>,
    light_model: Model,
    follow_mouse: bool,
    shape: Ball,
    layer: RenderTarget,
    camera: CameraBuffer,
}

impl Light {
    const RESOLUTION: u32 = 64;
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
            layer: ctx.create_render_target(Vector::new(1, 1)),
            follow_mouse: follow_cursor,
            vertices: model_builder.vertices.clone(),
            light_model: ctx.create_model(model_builder),
            light_color: ctx.create_uniform(color),
            shape: Ball::new(radius),
            base: BaseComponent::new(PositionBuilder::new().translation(position)),
            camera: ctx
                .create_camera_buffer(&Camera::new(position.into(), Vector::new(radius, radius))),
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };

    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let cursor_pos = ctx.cursor_camera(&ctx.world_camera);
        let window_size = ctx.window_size();
        for light in ctx.component_manager.path_mut(&active) {
            if *ctx.scene_resized {
                let light_size = Vector::new(light.radius, light.radius);
                let layer_size =
                    RenderTarget::compute_target_size(light_size, &ctx.world_camera, window_size);
                light.layer = ctx.gpu.create_render_target(layer_size);
            }
            if light.follow_mouse {
                light.camera = ctx.gpu.create_camera_buffer(&Camera::new(
                    cursor_pos.into(),
                    Vector::new(light.radius, light.radius),
                ));
                light.base.set_translation(cursor_pos);
            }
        }
    }

    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
            return v1.x * v2.y - v1.y * v2.x;
        }

        let map = ctx.create_render_target(ctx.window_size());
        let state = ctx.global_state::<LightingState>().unwrap();
        for (buffer, lights) in ctx.path_render(&active) {
            for (i, light) in lights {
                let mut shadows = vec![];
                let light_position = light.base.position();
                let light_translation = light.base.translation();

                ctx.intersections_with_shape(
                    &light_position,
                    &light.shape,
                    QueryFilter::new(),
                    |_component, collider_handle| {
                        let obstacle_collider = ctx.collider(collider_handle).unwrap();
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

                        let mut leftback =
                            light_translation + leftmost_ray.normalize() * light.radius;

                        let mut rightback =
                            light_translation + rightmost_ray.normalize() * light.radius;

                        let mut ray_angle: f32 = rightmost_ray.y.atan2(rightmost_ray.x);
                        if ray_angle < 0.0 {
                            ray_angle += 2.0 * PI;
                        }
                        let right_index = (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32)
                            as usize
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

                        let builder = ModelBuilder::convex_polygon(vertices)
                            .vertex_translation(-light_translation);
                        // let diameter = 2.0 * light.radius;
                        // for vertex in &mut builder.vertices {
                        //     let rel = vertex.pos - light_translation;
                        //     vertex.tex_coords =
                        //         Vector::new(rel.x / diameter + 0.5, rel.y / -diameter + 0.5);
                        // }
                        shadows.push(ctx.create_model(builder));
                        true
                    },
                );

                {
                    let mut renderer = encoder.renderer(RenderConfig {
                        target: RenderConfigTarget::Custom(&light.layer),
                        camera: RenderConfigCamera::Custom(&light.camera),
                        intances: Some(RenderConfigInstances::Custom(buffer)),
                        msaa: true,
                        clear_color: Some(Color::TRANSPARENT),
                    });
                    renderer.use_shader(&state.light_shader);
                    renderer.use_model(&light.light_model);
                    renderer.use_uniform(&light.light_color, 1);
                    renderer.draw(i);

                    for shadow in &shadows {
                        renderer.use_shader(&state.shadow_shader);
                        renderer.use_model(shadow);
                        renderer.use_uniform(&state.shadow_color, 1);
                        renderer.draw(i);
                    }
                }

                {
                    let mut renderer = encoder.renderer(RenderConfig {
                        // target: RenderConfigTarget::Custom(&map),
                        ..RenderConfig::WORLD
                    });
                    renderer.use_instances(buffer);
                    renderer.use_shader(&state.blend_additive);
                    renderer.use_model(&light.light_model);
                    renderer.use_sprite(&light.layer, 1);
                    renderer.draw(i);
                }
            }
        }
        // {
        //     let mut renderer = encoder.renderer(RenderConfig::RELATIVE_WORLD);
        //     renderer.use_instances(&ctx.defaults.single_centered_instance);
        //     renderer.use_shader(&state.present_map);
        //     renderer.use_model(&ctx.defaults.relative_camera.model);
        //     renderer.use_sprite(&map, 1);
        //     renderer.use_uniform(&state.shadow_color, 2);
        //     renderer.draw(0);
        // }
    }
}

#[derive(Component)]
struct Background {
    #[base]
    base: BaseComponent,
    model: Model,
    sprite: Sprite,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::square(10.0)),
            base: BaseComponent::new(Default::default()),
            sprite: ctx.create_sprite(include_bytes!("./img/background.png")),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 0,
        update: UpdateOperation::Never,
        ..DEFAULT_CONFIG
    };

    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::WORLD);
        for (buffer, obstacles) in ctx.path_render(&active) {
            for (i, b) in obstacles {
                renderer.render_sprite(buffer, i, &b.model, &b.sprite)
            }
        }
    }
}
