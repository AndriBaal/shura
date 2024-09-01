mod plugin;

use plugin::*;
use shura::gui::Widget;
use shura::prelude::*;
use std::f32::consts::{PI, TAU};

const SIZE: Vector2<u32> = vector!(800, 800);

#[shura::app]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .plugin(LightPlugin {})
            .system(System::setup(load_assets))
            .system(System::setup(setup))
            .system(System::render(render))
            .system(System::update(update))
            .entity::<MyLight>()
    })
}

fn load_assets(ctx: &mut Context) {
    ctx.assets.load_sprite(
        "background_sprite",
        SpriteBuilder::bytes(include_resource_bytes!("lighting/level.png")),
    );
    ctx.assets.load_mesh(
        "background_mesh",
        &MeshBuilder2D::<SpriteVertex2D>::cuboid(vector![10.0, 10.0]),
    );
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Min(10.0));
    let _ = ctx
        .window
        .request_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));

    ctx.entities.get_mut().add(
        ctx.world,
        MyLight {
            light: LightComponent {
                inner_radius: 0.2,
                outer_radius: 10.0,
                color: Color::BLUE,
                ..Default::default()
            },
            display: true,
            follow_player: true
        }
    );
    // ctx.entities.get_mut().add(
    //     ctx.world,
    //     MyLight {
    //         light: LightComponent {
    //             inner_radius: 0.2,
    //             outer_radius: 8.0,
    //             color: Color::RED,
    //             ..Default::default()
    //         },
    //         display: false,
    //         follow_player: false
    //     }
    // );
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(Some(Color::BLACK), |renderer| {
        renderer.draw_sprite_mesh(
            &ctx.assets.mesh("background_mesh"),
            &ctx.default_assets.world_camera2d,
            &ctx.assets.sprite("background_sprite"),
        );
    });
}

fn update(ctx: &mut Context) {
    let mut lights = ctx.entities.get_mut::<MyLight>();
    for my_light in lights.iter_mut() {
        if my_light.display {
            gui::Window::new("Light")
                .resizable(false)
                .show(ctx.gui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Position: {} / {}",
                            my_light.light.position.translation.x,
                            my_light.light.position.translation.y
                        ));
                    });

                    let mut rotation = my_light.light.position.rotation.angle();
                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                        gui::widgets::Slider::new(&mut rotation, -TAU..=TAU).ui(ui);
                    });
                    my_light.light.position.rotation = Rotation2::new(rotation);

                    ui.horizontal(|ui| {
                        ui.label("Outer Radius:");
                        gui::widgets::Slider::new(&mut my_light.light.outer_radius, 0.0..=50.0)
                            .ui(ui);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Inner Radius:");
                        gui::widgets::Slider::new(&mut my_light.light.inner_radius, 0.0..=1.0)
                            .ui(ui);
                    });

                    let mut egui_color = my_light.light.color.into();
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        gui::widgets::color_picker::color_edit_button_rgba(
                            ui,
                            &mut egui_color,
                            egui::widgets::color_picker::Alpha::OnlyBlend,
                        )
                    });
                    my_light.light.color = egui_color.into();

                    ui.horizontal(|ui| {
                        ui.label("Inner Magnification:");
                        gui::widgets::Slider::new(
                            &mut my_light.light.inner_magnification,
                            0.01..=10.0,
                        )
                        .ui(ui);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Outer Magnification:");
                        gui::widgets::Slider::new(
                            &mut my_light.light.outer_magnification,
                            0.01..=10.0,
                        )
                        .ui(ui);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Side Falloff Magnification:");
                        gui::widgets::Slider::new(
                            &mut my_light.light.side_falloff_magnification,
                            0.0..=10.0,
                        )
                        .ui(ui);
                    });

                    let end = my_light.light.sector.y;
                    ui.horizontal(|ui| {
                        ui.label("Start:");
                        gui::widgets::Slider::new(&mut my_light.light.sector.x, -PI..=end).ui(ui);
                    });

                    let start = my_light.light.sector.x;
                    ui.horizontal(|ui| {
                        ui.label("End:");
                        gui::widgets::Slider::new(&mut my_light.light.sector.y, start..=PI).ui(ui);
                    });
                });

            if ctx.input.is_pressed(MouseButton::Left) && !ctx.gui.is_pointer_over_area() {
                my_light.follow_player = !my_light.follow_player;
            }
            if my_light.follow_player {
                my_light.light.position.translation.vector = ctx.cursor.coords;
            }
        }
    }
}

#[derive(Entity)]
pub struct MyLight {
    #[shura(component)]
    light: LightComponent,
    display: bool,
    follow_player: bool,
}
