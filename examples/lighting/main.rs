use std::f32::consts::PI;

use egui::Widget;

use shura::prelude::*;
use shura::prelude::*;

const SIZE: Vector2<u32> = vector!(800, 800);

#[shura::main]
fn app(mut config: AppConfig) {
    config.gpu.max_samples = 1;
    App::run(config, || {
        Scene::new()
            .system(System::setup(setup))
            .system(System::update(update))
            .system(System::resize(resize))
            .system(System::render(render))
            .render_group2d("background", RenderGroupUpdate::MANUAL)
            // .render_group::<LightInstance>("shadow", RenderGroupUpdate::EVERY_FRAME)
            .render_group::<LightInstance>("light", RenderGroupUpdate::EVERY_FRAME)
            .entity_single::<LightAssets>()
            .entity::<Light>()
    })
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Min(10.0));
    let _ = ctx
        .window
        .request_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));
    // ctx.window.set_resizable(false);
    // ctx.window
    //     .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
    ctx.entities
        .single_mut()
        .set(ctx.world, LightAssets::new(ctx));

    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(
            LightInstance {
                outer_radius: 10.0,
                inner_radius: 5.0,
                color: Color::BLUE,
                ..Default::default()
            },
            false,
        ),
    );
    ctx.entities.get_mut().add(
        ctx.world,
        Light::new(
            LightInstance {
                translation: vector!(0.0, -2.0),
                outer_radius: 12.0,
                inner_radius: 5.0,
                color: Color::RED,
                ..Default::default()
            },
            true,
        ),
    );
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let assets = ctx.entities.single::<LightAssets>().get_ref().unwrap();
    encoder.render2d(Some(Color::BLACK), |renderer| {
        ctx.render(renderer, "background", |renderer, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                ctx.world_camera2d,
                ctx.unit_mesh,
                &assets.background_sprite,
            );
        });
    });

    encoder.render2d_to(Some(Color::BLACK), &assets.light_map, |renderer| {
        ctx.render::<LightInstance>(renderer, "light", |renderer, buffer, instances| {
            renderer.use_instances(buffer);
            renderer.use_camera(ctx.world_camera2d);
            renderer.use_shader(&assets.light_shader);
            renderer.use_mesh(ctx.unit_mesh);
            renderer.draw(instances);
        });

        // ctx.render::<LightInstance>(renderer, "shadow", |renderer, buffer, instances| {
        //     renderer.use_instances(buffer);
        //     renderer.use_camera(ctx.world_camera2d);
        //     renderer.use_shader(&assets.shadow_shader);
        //     renderer.use_mesh(ctx.unit_mesh);
        //     renderer.draw(instances);
        // });
    });

    encoder.render2d(None, |renderer| {
        renderer.use_mesh(ctx.unit_mesh);
        renderer.use_camera(ctx.unit_camera);
        renderer.use_instances(ctx.centered_instance);
        renderer.use_shader(&assets.present_shader);
        renderer.use_sprite(assets.light_map.sprite(), 1);
        renderer.draw(0..1);
    });
}

fn resize(ctx: &mut Context) {
    let mut res = ctx.entities.single_mut::<LightAssets>().get_ref().unwrap();
    res.light_map.resize(&ctx.gpu, ctx.window_size);
}

fn update(ctx: &mut Context) {
    for light in ctx.entities.get_mut::<Light>().iter_mut() {
        if light.follow_player {
            light.inner.translation = ctx.cursor.coords;

            gui::Window::new("Light")
                .resizable(false)
                .show(ctx.gui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Outer Radius:");
                        gui::widgets::Slider::new(&mut light.inner.outer_radius, 0.0..=50.0).ui(ui);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Inner Radius:");
                        gui::widgets::Slider::new(
                            &mut light.inner.inner_radius,
                            0.0..=light.inner.outer_radius,
                        )
                        .ui(ui);
                    });

                    let mut egui_color = light.inner.color.into();
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        gui::widgets::color_picker::color_edit_button_rgba(
                            ui,
                            &mut egui_color,
                            egui::widgets::color_picker::Alpha::OnlyBlend,
                        )
                    });
                    light.inner.color = egui_color.into();

                    //
                    // ui.horizontal(|ui| {
                    //     ui.label("Test:");
                    //     gui::widgets::Slider::new(&mut light.inner.outer_radius, 0.0..=100.0).ui(ui);
                    // });
                    //
                    // ui.horizontal(|ui| {
                    //     ui.label("Test:");
                    //     gui::widgets::Slider::new(&mut light.inner.outer_radius, 0.0..=100.0).ui(ui);
                    // });
                    //
                    // ui.horizontal(|ui| {
                    //     ui.label("Test:");
                    //     gui::widgets::Slider::new(&mut light.inner.outer_radius, 0.0..=100.0).ui(ui);
                    // });
                    //
                    // ui.horizontal(|ui| {
                    //     ui.label("Test:");
                    //     gui::widgets::Slider::new(&mut light.inner.outer_radius, 0.0..=100.0).ui(ui);
                    // });
                });
        }
    }
}

#[derive(Entity)]
pub struct LightAssets {
    light_map: SpriteRenderTarget,
    light_shader: Shader,
    shadow_shader: Shader,
    present_shader: Shader,
    background_sprite: Sprite,
    #[shura(component = "background")]
    background: PositionComponent2D,
}

impl LightAssets {
    pub fn new(ctx: &Context) -> Self {
        Self {
            present_shader: ctx.gpu.create_shader(ShaderConfig {
                source: ShaderModuleSource::Separate {
                    vertex: &ctx.gpu.shared_assets().vertex_shader_module,
                    fragment: &ctx
                        .gpu
                        .create_shader_module(include_wgsl!("../../static/shader/2d/sprite.wgsl")),
                },
                uniforms: &[UniformField::Camera, UniformField::Sprite],
                blend: BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::Zero,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent::REPLACE,
                },
                ..Default::default()
            }),
            shadow_shader: ctx.gpu.create_shader(ShaderConfig {
                source: ShaderModuleSource::Single(
                    &ctx.gpu
                        .create_shader_module(include_asset_wgsl!("lighting/light.wgsl")),
                ),
                uniforms: &[UniformField::Camera],
                buffers: &[Vertex2D::LAYOUT, LightInstance::LAYOUT],
                blend: BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One, // Use additive blending for the first render
                        operation: BlendOperation::ReverseSubtract,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::OneMinusSrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::ReverseSubtract,
                    },
                },
                ..Default::default()
            }),
            light_shader: ctx.gpu.create_shader(ShaderConfig {
                source: ShaderModuleSource::Single(
                    &ctx.gpu
                        .create_shader_module(include_asset_wgsl!("lighting/light.wgsl")),
                ),
                uniforms: &[UniformField::Camera],
                buffers: &[Vertex2D::LAYOUT, LightInstance::LAYOUT],
                ..Default::default()
            }),
            light_map: ctx.gpu.create_render_target(ctx.window_size),
            background_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "lighting/level.png"
                ))),
            background: PositionComponent2D::new().with_scaling(Vector2::new(10.0, 10.0) * 2.0),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightInstance {
    translation: Vector2<f32>,
    outer_radius: f32,
    rotation: f32,
    color: Color,
    sector: Vector2<f32>,
    inner_radius: f32,
    inner_magnification: f32,
    outer_magnification: f32,
}

impl Default for LightInstance {
    fn default() -> Self {
        Self {
            translation: vector!(0.0, 0.0),
            rotation: 0.0,
            outer_radius: 10.0,
            color: Color::WHITE,
            sector: vector![-PI, PI],
            inner_radius: 2.0,
            inner_magnification: 1.0,
            outer_magnification: 1.0,
        }
    }
}

impl Instance for LightInstance {
    const ATTRIBUTES: &'static [VertexAttribute] = &vertex_attr_array![
        2 => Float32x2,
        3 => Float32,
        4 => Float32,
        5 => Float32x4,
        6 => Float32x2,
        7 => Float32,
        8 => Float32,
        9 => Float32,
    ];
}

#[derive(Entity)]
struct Light {
    #[shura(component = "light")]
    inner: LightInstance,

    follow_player: bool,
}

impl Light {
    pub fn new(instance: LightInstance, follow_player: bool) -> Self {
        if instance.inner_radius > instance.outer_radius {
            panic!("Inner radius must be smaller than outer radius!");
        }

        Self {
            inner: instance,
            follow_player,
        }
    }
}
