// use shura::{physics::*, *};
// use std::f32::consts::PI;

// const TWO_PI_INV: f32 = 1.0 / (2.0 * PI);

// fn main() {
//     Shura::init(NewScene {
//         id: 1,
//         init: |ctx| {
//             ctx.set_clear_color(Some(Color::BLACK));
//             ctx.set_camera_vertical_fov(10.0);
//             ctx.set_global_state(LightingState {
//                 shadow_color: ctx.create_uniform(Color::TRANSPARENT),
//                 inner_model: ctx.create_model(ModelBuilder::ball(0.5, 24)),
//                 light_shader: ctx.create_shader(ShaderConfig {
//                     fragment_source: include_str!("./light.glsl"),
//                     shader_lang: ShaderLang::GLSL,
//                     shader_fields: &[ShaderField::Uniform],
//                     blend: BlendState::ALPHA_BLENDING,
//                     msaa: true,
//                     write_mask: ColorWrites::ALL,
//                 }),
//                 shadow_shader: ctx.create_shader(ShaderConfig {
//                     fragment_source: include_str!("./shadow.glsl"),
//                     shader_lang: ShaderLang::GLSL,
//                     shader_fields: &[ShaderField::Uniform],
//                     blend: BlendState::REPLACE,
//                     // blend: BlendState {
//                     //     color: BlendComponent {
//                     //         src_factor: BlendFactor::SrcAlpha,
//                     //         dst_factor: BlendFactor::OneMinusSrcAlpha,
//                     //         operation: BlendOperation::Subtract,
//                     //     },
//                     //     alpha: BlendComponent::OVER,
//                     // },
//                     msaa: true,
//                     write_mask: ColorWrites::ALL,
//                 }),
//             });
//             ctx.add_component(Obstacle::new(
//                 ctx,
//                 Vector::new(3.0, 3.0),
//                 ColliderBuilder::cuboid(1.0, 1.0),
//                 Color::GREEN,
//             ));

//             ctx.add_component(Obstacle::new(
//                 ctx,
//                 Vector::new(-3.0, 2.5),
//                 ColliderBuilder::triangle(
//                     Point::new(-1.5, 1.0),
//                     Point::new(1.0, 1.5),
//                     Point::new(1.5, 1.0),
//                 ),
//                 Color::BLUE,
//             ));

//             for i in 0..4 {
//                 ctx.add_component(Obstacle::new(
//                     ctx,
//                     Vector::new(-6.0, i as f32 * 1.0),
//                     ColliderBuilder::cuboid(0.04, 0.4),
//                     Color::BLUE,
//                 ));
//             }

//             ctx.add_component(Obstacle::new(
//                 ctx,
//                 Vector::new(6.0, 0.0),
//                 ColliderBuilder::ball(1.0),
//                 Color::BLUE,
//             ));

//             ctx.add_component(Obstacle::new(
//                 ctx,
//                 Vector::new(-3.0, -3.0),
//                 ColliderBuilder::cuboid(0.5, 1.5),
//                 Color::BLUE,
//             ));
//             ctx.add_component(Obstacle::new(
//                 ctx,
//                 Vector::new(3.0, -3.0),
//                 ColliderBuilder::round_cuboid(0.5, 1.5, 0.4),
//                 Color::BLUE,
//             ));
//             ctx.add_component(Light::new(
//                 ctx,
//                 Vector::new(0.0, 0.0),
//                 12.0,
//                 Color::RED,
//                 true,
//             ));
//             ctx.add_component(Light::new(
//                 ctx,
//                 Vector::new(0.0, 1.0),
//                 10.0,
//                 Color::GREEN,
//                 false,
//             ));
//         },
//     });
// }

// impl GlobalState for LightingState {}
// struct LightingState {
//     light_shader: Shader,
//     shadow_shader: Shader,
//     shadow_color: Uniform<Color>,
//     inner_model: Model,
// }

// #[derive(Component)]
// struct Obstacle {
//     #[base]
//     base: BaseComponent,
//     model: Model,
//     color: Uniform<Color>,
// }

// impl Obstacle {
//     pub fn new(
//         ctx: &Context,
//         position: Vector<f32>,
//         collider: ColliderBuilder,
//         color: Color,
//     ) -> Self {
//         Self {
//             model: ctx.create_model(ModelBuilder::from_collider_shape(
//                 collider.shape.as_ref(),
//                 24,
//                 2.0,
//             )),
//             base: BaseComponent::new_rigid_body(
//                 RigidBodyBuilder::fixed().translation(position),
//                 vec![collider],
//             ),
//             color: ctx.create_uniform(color),
//         }
//     }
// }

// impl ComponentController for Obstacle {
//     const CONFIG: ComponentConfig = ComponentConfig {
//         priority: 2,
//         update: UpdateOperation::Never,
//         ..DEFAULT_CONFIG
//     };

//     fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
//         let mut renderer = encoder.renderer(RenderConfig::WORLD);
//         for (buffer, obstacles) in ctx.path_render(&active) {
//             for (i, b) in obstacles {
//                 renderer.render_color(buffer, i, &b.model, &b.color)
//             }
//         }
//     }
// }

// #[derive(Component)]
// struct Light {
//     #[base]
//     base: BaseComponent,
//     radius: f32,
//     vertices: Vec<Vertex>,
//     shadows: Vec<Model>,
//     light_color: Uniform<Color>,
//     light_model: Model,
//     follow_mouse: bool,
// }

// impl Light {
//     const RESOLUTION: u32 = 64;
//     const IS_LIGHT_COLLIDER: u128 = 1000;
//     pub fn new(
//         ctx: &Context,
//         position: Vector<f32>,
//         radius: f32,
//         color: Color,
//         follow_cursor: bool,
//     ) -> Self {
//         let model_builder = ModelBuilder::ball(radius, Self::RESOLUTION);
//         Self {
//             radius,
//             follow_mouse: follow_cursor,
//             shadows: vec![],
//             vertices: model_builder.vertices.clone(),
//             base: BaseComponent::new_rigid_body(
//                 RigidBodyBuilder::dynamic().translation(position),
//                 vec![ColliderBuilder::ball(radius)
//                     .sensor(true)
//                     .user_data(Self::IS_LIGHT_COLLIDER)],
//             ),
//             light_model: ctx.create_model(model_builder),
//             light_color: ctx.create_uniform(color),
//         }
//     }
// }

// impl ComponentController for Light {
//     const CONFIG: ComponentConfig = ComponentConfig {
//         priority: 1,
//         ..DEFAULT_CONFIG
//     };

//     fn update(active: ComponentPath<Self>, ctx: &mut Context) {
//         fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
//             return v1.x * v2.y - v1.y * v2.x;
//         }

//         let cursor_pos = ctx.cursor_camera(&ctx.world_camera);
//         for light in ctx.path_mut(&active) {
//             if light.follow_mouse {
//                 light.base.set_translation(cursor_pos);
//             }
//         }

//         let mut all_shadows = vec![];
//         for light in ctx.path(&active) {
//             let mut shadows = vec![];
//             let light_collider_handle = light.base.collider_handles().unwrap()[0];
//             let light_collider = light.base.collider(light_collider_handle).unwrap();
//             let light_translation = light.base.translation();

//             ctx.intersections_with_shape(
//                 light_collider.position(),
//                 light_collider.shape(),
//                 QueryFilter::new(),
//                 |_component, collider_handle| {
//                     let obstacle_collider = ctx.collider(collider_handle).unwrap();
//                     if obstacle_collider.user_data != Self::IS_LIGHT_COLLIDER {
//                         let obstacle_shape = obstacle_collider.shape();
//                         let collider_vertices =
//                             ModelBuilder::from_collider_shape(obstacle_shape, 24, 2.0)
//                                 .vertex_position(*obstacle_collider.position())
//                                 .apply_modifiers()
//                                 .vertices;

//                         let mut leftmost = collider_vertices[0].pos;
//                         let mut rightmost = collider_vertices[0].pos;
//                         let mut leftmost_ray = leftmost - light_translation;
//                         let mut rightmost_ray = rightmost - light_translation;

//                         let mut ray;
//                         for v in &collider_vertices[1..] {
//                             ray = v.pos - light_translation;
//                             if det(ray, leftmost_ray) < 0.0 {
//                                 leftmost = v.pos;
//                                 leftmost_ray = ray;
//                             } else if det(ray, rightmost_ray) > 0.0 {
//                                 rightmost = v.pos;
//                                 rightmost_ray = ray;
//                             }
//                         }

//                         let mut leftback =
//                             light_translation + leftmost_ray.normalize() * light.radius;

//                         let mut rightback =
//                             light_translation + rightmost_ray.normalize() * light.radius;

//                         let mut ray_angle: f32 = rightmost_ray.y.atan2(rightmost_ray.x);
//                         if ray_angle < 0.0 {
//                             ray_angle += 2.0 * PI;
//                         }
//                         let right_index = (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32)
//                             as usize
//                             % Self::RESOLUTION as usize;
//                         let v0 = light.vertices[right_index].pos;
//                         let v0_to_v1 =
//                             light.vertices[(right_index + 1) % Self::RESOLUTION as usize].pos - v0;
//                         let alpha = rightmost_ray.angle(&(-v0_to_v1));
//                         let v0_to_rightback = rightback - (light_translation + v0);
//                         let beta = v0_to_rightback.angle(&v0_to_v1);
//                         let gamma = PI - alpha - beta;
//                         let c = gamma.sin() / alpha.sin() * v0_to_rightback.norm();
//                         rightback = light_translation + v0 + c * v0_to_v1.normalize();

//                         let mut ray_angle: f32 = leftmost_ray.y.atan2(leftmost_ray.x);
//                         if ray_angle < 0.0 {
//                             ray_angle += 2.0 * PI;
//                         }
//                         let left_index = (ray_angle * TWO_PI_INV * Self::RESOLUTION as f32) as usize
//                             as usize
//                             % Self::RESOLUTION as usize;
//                         let v0 = light.vertices[left_index].pos;
//                         let v0_to_v1 =
//                             light.vertices[(left_index + 1) % Self::RESOLUTION as usize].pos - v0;
//                         let alpha = leftmost_ray.angle(&(-v0_to_v1));
//                         let v0_to_leftback = leftback - (light_translation + v0);
//                         let beta = v0_to_leftback.angle(&v0_to_v1);
//                         let gamma = PI - alpha - beta;
//                         let c = gamma.sin() / alpha.sin() * v0_to_leftback.norm();
//                         leftback = light_translation + v0 + c * v0_to_v1.normalize();

//                         let mut vertices = vec![];
//                         if light_collider
//                             .shape()
//                             .contains_point(light_collider.position(), &rightmost.into())
//                         {
//                             vertices.push(rightmost);
//                         }
//                         vertices.push(rightback);
//                         let end = (left_index + 1) % (Self::RESOLUTION as usize);
//                         let mut i = (right_index + 1) % (Self::RESOLUTION as usize);
//                         while i != end {
//                             vertices.push(light_translation + light.vertices[i].pos);
//                             i = (i + 1) % (Self::RESOLUTION as usize);
//                         }

//                         vertices.push(leftback);
//                         if light_collider
//                             .shape()
//                             .contains_point(light_collider.position(), &leftmost.into())
//                         {
//                             vertices.push(leftmost);
//                         }

//                         let mut builder = ModelBuilder::convex_polygon(vertices)
//                             .vertex_translation(-light_translation);
//                         let diameter = 2.0 * light.radius;
//                         for vertex in &mut builder.vertices {
//                             let rel = vertex.pos - light_translation;
//                             vertex.tex_coords =
//                                 Vector::new(rel.x / diameter + 0.5, rel.y / -diameter + 0.5);
//                         }
//                         shadows.push(ctx.create_model(builder));
//                     }
//                     true
//                 },
//             );
//             let size = RenderTarget::compute_target_size(
//                 Vector::new(light.radius, light.radius),
//                 &ctx.world_camera,
//                 ctx.window_size(),
//             );
//             let state = ctx.global_state::<LightingState>().unwrap();
//             let test = ctx.create_computed_target(size, &ctx.defaults.unit_camera, |mut config, encoder| {
//                 // config.clear_color = Some(Color::BLACK);
//                 let mut renderer = encoder.renderer(config);
//                 renderer.use_instances(&ctx.defaults.single_centered_instance);
//                 renderer.use_shader(&state.light_shader);
//                 renderer.use_model(&ctx.defaults.unit_camera.model);
//                 renderer.use_uniform(&light.light_color, 1);
//                 renderer.draw(0);

//                 for shadow in &shadows {
//                     // renderer.render_color(&buffer, i, shadow, &state.shadow_color);

//                     renderer.use_shader(&state.shadow_shader);
//                     renderer.use_model(shadow);
//                     renderer.use_uniform(&state.shadow_color, 1);
//                     renderer.draw(0);
//                 }
//             });
//             test.save(ctx.gpu, "test.png");
//             all_shadows.push(shadows);
//         }

//         for (light, shadows) in ctx.path_mut(&active).zip(all_shadows.into_iter()) {
//             light.shadows = shadows;
//         }
//     }

//     fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
//         let mut renderer = encoder.renderer(RenderConfig::WORLD);
//         let state = ctx.global_state::<LightingState>().unwrap();
//         for (buffer, lights) in ctx.path_render(&active) {
//             renderer.use_instances(&buffer);

//             for (i, light) in lights {
//                 renderer.use_shader(&state.light_shader);
//                 renderer.use_model(&light.light_model);
//                 renderer.use_uniform(&light.light_color, 1);
//                 renderer.draw(i);

//                 for shadow in &light.shadows {
//                     // renderer.render_color(&buffer, i, shadow, &state.shadow_color);

//                     renderer.use_shader(&state.shadow_shader);
//                     renderer.use_model(shadow);
//                     renderer.use_uniform(&light.light_color, 1);
//                     renderer.draw(i);
//                 }

//                 renderer.use_shader(&ctx.defaults.color);
//                 renderer.use_model(&state.inner_model);
//                 renderer.use_uniform(&light.light_color, 1);
//                 renderer.draw(i);
//             }

//         }
//     }
// }

use shura::physics::parry::query::PointQuery;
use shura::{physics::*, *};
use std::f32::consts::PI;

const TWO_PI_INV: f32 = 1.0 / (2.0 * PI);

fn main() {
    Shura::init(NewScene {
        id: 1,
        init: |ctx| {
            ctx.set_clear_color(Some(Color::BLACK));
            ctx.set_camera_scale(WorldCameraScale::Max(10.0));

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

            ctx.add_component(LightManager::new(ctx));
            ctx.add_component(Background::new(ctx));
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
        },
    });
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

struct Light {
    position: Isometry<f32>,
    model: Model,
    color: Uniform<Color>,
    shape: Ball,
    vertices: Vec<Vertex>,
}

impl Light {
    pub const RESOLUTION: u32 = 64;
    pub const IS_LIGHT_COLLIDER: u128 = 1000;
    pub fn new(ctx: &Context, translation: Vector<f32>, radius: f32, color: Color) -> Self {
        let builder = ModelBuilder::ball(radius, Self::RESOLUTION);
        let model = builder.clone().build(ctx.gpu);
        Self {
            position: translation.into(),
            model: model,
            vertices: builder.vertices,
            color: ctx.create_uniform(color),
            shape: Ball::new(radius),
        }
    }
}

#[derive(Component)]
pub struct LightManager {
    #[base]
    base: BaseComponent,
    shadow: Color,
    shadow_uniform: Uniform<Color>,
    map: RenderTarget,
    light_layer: RenderTarget,
    lights: Vec<Light>,

    light_shader: Shader,
    shadow_shader: Shader,
    insert_light_shader: Shader,
    shadow_color: Uniform<Color>,
    inner_model: Model,
}

impl LightManager {
    pub fn new(ctx: &Context) -> Self {
        let shadow = Color::new(0.0, 0.0, 0.0, 0.9);
        Self {
            base: Default::default(),
            shadow,
            map: ctx.create_render_target(ctx.window_size()),
            light_layer: ctx.create_render_target(ctx.window_size()),
            shadow_color: ctx.create_uniform(shadow),
            inner_model: ctx.create_model(ModelBuilder::ball(0.5, 24)),
            lights: vec![Light::new(
                ctx,
                Vector::new(0.0, 0.0),
                5.0,
                Color {
                    a: 0.1,
                    ..Color::RED
                },
            )],
            light_shader: ctx.create_shader(ShaderConfig {
                fragment_source: include_str!("./light.glsl"),
                shader_lang: ShaderLang::GLSL,
                shader_fields: &[ShaderField::Uniform, ShaderField::Uniform],
                blend: BlendState::ALPHA_BLENDING,
                msaa: false,
                write_mask: ColorWrites::ALL,
            }),
            shadow_shader: ctx.create_shader(ShaderConfig {
                fragment_source: Shader::COLOR_WGSL,
                shader_lang: ShaderLang::WGSL,
                shader_fields: &[ShaderField::Uniform],
                blend: BlendState::REPLACE,
                msaa: false,
                write_mask: ColorWrites::ALL,
            }),
            insert_light_shader: ctx.create_shader(ShaderConfig {
                fragment_source: Shader::SPIRTE_WGSL,
                shader_lang: ShaderLang::WGSL,
                shader_fields: &[ShaderField::Sprite],
                blend: BlendState::REPLACE,
                msaa: false,
                write_mask: ColorWrites::ALL,
            }),
            shadow_uniform: ctx.create_uniform(shadow),
        }
    }
}

impl ComponentController for LightManager {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };

    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::RELATIVE_WORLD);
        for (instance, maps) in ctx.path_render(&active) {
            for (index, map) in maps {
                renderer.render_sprite(
                    instance,
                    index,
                    ctx.defaults.relative_camera.0.model(),
                    &map.map,
                );
            }
        }
    }

    fn update(active: ComponentPath<Self>, ctx: &mut Context) {
        let window_size = ctx.window_size();
        fn det(v1: Vector<f32>, v2: Vector<f32>) -> f32 {
            return v1.x * v2.y - v1.y * v2.x;
        }
        for man in ctx.component_manager.path_mut(&active) {
            if window_size != *man.map.sprite().size() {
                man.map = ctx.gpu.create_render_target(window_size);
                man.light_layer = ctx.gpu.create_render_target(window_size);
            }
        }

        for man in ctx.component_manager.path(&active) {
            for light in &man.lights {
                let mut shadows = vec![];
                let light_translation = light.position.translation.vector;

                ctx.intersections_with_shape(
                    &light.position,
                    &light.shape,
                    QueryFilter::new(),
                    |_component, collider_handle| {
                        let obstacle_collider = ctx.collider(collider_handle).unwrap();
                        if obstacle_collider.user_data != Light::IS_LIGHT_COLLIDER {
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
                                light_translation + leftmost_ray.normalize() * light.shape.radius;

                            let mut rightback =
                                light_translation + rightmost_ray.normalize() * light.shape.radius;

                            let mut ray_angle: f32 = rightmost_ray.y.atan2(rightmost_ray.x);
                            if ray_angle < 0.0 {
                                ray_angle += 2.0 * PI;
                            }
                            let right_index = (ray_angle * TWO_PI_INV * Light::RESOLUTION as f32)
                                as usize
                                % Light::RESOLUTION as usize;
                            let v0 = light.vertices[right_index].pos;
                            let v0_to_v1 =
                                light.vertices[(right_index + 1) % Light::RESOLUTION as usize].pos
                                    - v0;
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
                            let left_index = (ray_angle * TWO_PI_INV * Light::RESOLUTION as f32)
                                as usize as usize
                                % Light::RESOLUTION as usize;
                            let v0 = light.vertices[left_index].pos;
                            let v0_to_v1 =
                                light.vertices[(left_index + 1) % Light::RESOLUTION as usize].pos
                                    - v0;
                            let alpha = leftmost_ray.angle(&(-v0_to_v1));
                            let v0_to_leftback = leftback - (light_translation + v0);
                            let beta = v0_to_leftback.angle(&v0_to_v1);
                            let gamma = PI - alpha - beta;
                            let c = gamma.sin() / alpha.sin() * v0_to_leftback.norm();
                            leftback = light_translation + v0 + c * v0_to_v1.normalize();

                            let mut vertices = vec![];
                            if light
                                .shape
                                .contains_point(&light.position, &rightmost.into())
                            {
                                vertices.push(rightmost);
                            }
                            vertices.push(rightback);
                            let end = (left_index + 1) % (Light::RESOLUTION as usize);
                            let mut i = (right_index + 1) % (Light::RESOLUTION as usize);
                            while i != end {
                                vertices.push(light_translation + light.vertices[i].pos);
                                i = (i + 1) % (Light::RESOLUTION as usize);
                            }

                            vertices.push(leftback);
                            if light
                                .shape
                                .contains_point(&light.position, &leftmost.into())
                            {
                                vertices.push(leftmost);
                            }

                            let mut builder = ModelBuilder::convex_polygon(vertices)
                                .vertex_translation(-light_translation);
                            let diameter = 2.0 * light.shape.radius;
                            for vertex in &mut builder.vertices {
                                let rel = vertex.pos - light_translation;
                                vertex.tex_coords =
                                    Vector::new(rel.x / diameter + 0.5, rel.y / -diameter + 0.5);
                            }
                            shadows.push(ctx.create_model(builder));
                        }
                        true
                    },
                );

                // let state = ctx.global_state::<LightingState>().unwrap();
                // let test = ctx.create_computed_target(
                //     size,
                //     &ctx.defaults.unit_camera,
                //     |mut config, encoder| {
                //         // config.clear_color = Some(Color::BLACK);
                //         let mut renderer = encoder.renderer(config);
                //         renderer.use_instances(&ctx.defaults.single_centered_instance);
                //         renderer.use_shader(&state.light_shader);
                //         renderer.use_model(&ctx.defaults.unit_camera.model);
                //         renderer.use_uniform(&light.light_color, 1);
                //         renderer.draw(0);

                //         for shadow in &shadows {
                //             // renderer.render_color(&buffer, i, shadow, &state.shadow_color);

                //             renderer.use_shader(&state.shadow_shader);
                //             renderer.use_model(shadow);
                //             renderer.use_uniform(&state.shadow_color, 1);
                //             renderer.draw(0);
                //         }
                //     },
                // );

                let test_camera = ctx.gpu.create_camera_buffer(&Camera::new(
                    Vector::default().into(),
                    ctx.world_camera.fov(),
                ));
                man.light_layer.draw(
                    &ctx.gpu,
                    &ctx.defaults,
                    &test_camera,
                    |mut config, encoder| {
                        config.msaa = false;
                        config.clear_color = Some(Color::TRANSPARENT);
                        let mut renderer = encoder.renderer(config);
                        // TODO: Change to lights world pos
                        let instances = &ctx.defaults.single_centered_instance;
                        renderer.use_instances(instances);
                        renderer.use_shader(&man.light_shader);
                        renderer.use_model(&light.model);
                        renderer.use_uniform(&light.color, 1);
                        renderer.use_uniform(&man.shadow_uniform, 2);
                        renderer.draw(instances.all_instances());

                        for shadow in &shadows {
                            renderer.use_shader(&man.shadow_shader);
                            renderer.use_model(shadow);
                            renderer.use_uniform(&man.shadow_color, 1);
                            renderer.draw(0);
                        }
                    },
                );

                man.light_layer.save(ctx.gpu, "test.png");

                // man.map.draw(
                //     &ctx.gpu,
                //     &ctx.defaults,
                //     &test_camera,
                //     |mut config, encoder| {
                //         config.msaa = false;
                //         config.clear_color = Some(man.shadow);
                //         let mut renderer = encoder.renderer(config);
                //         renderer.use_instances(&ctx.defaults.single_centered_instance);
                //         renderer.use_shader(&man.insert_light_shader);
                //         renderer.use_model();
                //         renderer.use_sprite(&man.light_layer, 1);
                //         renderer.draw(ctx.defaults.single_centered_instance.all_instances());

                //         // renderer.use_shader(&man.shadow_shader);
                //         // renderer.use_uniform(&test_color1, 1);
                //         // renderer.draw(ctx.defaults.single_centered_instance.all_instances());
                //     },
                // );
            }
        }
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
        priority: 1,
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
