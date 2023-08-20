use shura::{log, rand, *, text::*};

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 1,
        init: |ctx| {
            register!(ctx, [Bunny, Resources]);
            ctx.components
                
                .add::<Resources>(ctx.world, Resources::new(ctx));
            ctx.screen_config
                .set_clear_color(Some(RgbaColor::new(220, 220, 220, 255).into()));
            ctx.world_camera.set_scaling(WorldCameraScale::Min(3.0));
            ctx.components
                
                .add_with::<Bunny>(ctx.world, |handle| {
                    Bunny::new(Vector::new(0.0, 0.0), handle)
                });
        },
    });
}
#[derive(Component)]
struct Resources {
    screenshot: Option<RenderTarget>,
    bunny_model: Model,
    bunny_sprite: Sprite,
    font: FontBrush,
}
impl ComponentController for Resources {
    const CONFIG: ComponentConfig = ComponentConfig::RESOURCE;
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        let bunny_model = ctx
            .gpu
            .create_model(ModelBuilder::cuboid(Vector::new(0.06, 0.09)));
        let bunny_sprite = ctx.gpu.create_sprite(sprite_file!("./img/wabbit.png"));
        Resources {
            screenshot: None,
            bunny_model,
            bunny_sprite,
            font: ctx
                .gpu
                .create_font(include_bytes!("./font/novem.ttf"), 1000),
        }
    }
}

#[derive(Component)]
struct Bunny {
    #[position]
    position: PositionComponent,
    linvel: Vector<f32>,
    handle: ComponentHandle,
}
impl Bunny {
    pub fn new(translation: Vector<f32>, handle: ComponentHandle) -> Bunny {
        let scale = rand::gen_range(0.75..2.0);
        let position = PositionComponent::new()
            .with_translation(translation)
            .with_rotation(Rotation::new(rand::gen_range(-1.0..1.0)))
            .with_scale(Vector::new(scale, scale));
        let linvel = Vector::new(rand::gen_range(-2.5..2.5), rand::gen_range(-7.5..7.5));
        Bunny {
            position,
            linvel,
            handle,
        }
    }
}

impl ComponentController for Bunny {
    const CONFIG: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
        const MODIFY_STEP: usize = 1500;
        const GRAVITY: f32 = -2.5;
        gui::Window::new("bunnymark")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(format!("FPS: {}", ctx.frame.fps()));
                ui.label(format!("Bunnies: {}", ctx.components.len::<Bunny>()));
                if ui.button("Clear Bunnies").clicked() {
                    ctx.screen_config.set_render_scale(0.5);
                    ctx.components.remove_all::<Bunny>(ctx.world);
                }
            });

        if ctx.input.is_held(MouseButton::Left) || ctx.input.is_held(ScreenTouch) {
            let cursor = ctx.input.cursor(ctx.world_camera);
            for _ in 0..MODIFY_STEP {
                ctx.components
                    
                    .add_with::<Bunny>(ctx.world, |handle| Bunny::new(cursor, handle));
            }
        }
        if ctx.input.is_held(MouseButton::Right) {
            let mut dead: Vec<ComponentHandle> = vec![];
            let mut bunnies = ctx.components.set::<Bunny>();
            if bunnies.len() != 1 {
                for bunny in bunnies.iter().rev() {
                    if dead.len() == MODIFY_STEP {
                        break;
                    }
                    dead.push(bunny.handle);
                }
                for handle in dead {
                    bunnies.remove(ctx.world, handle);
                }
            }
        }

        {
            let mut resources = ctx.components.single_mut::<Resources>();
            if let Some(screenshot) = resources.screenshot.take() {
                log::info!("Saving Screenshot!");
                screenshot.sprite().save(&ctx.gpu, "screenshot.png").ok();
            } else if ctx.input.is_pressed(Key::S) {
                resources.screenshot = Some(ctx.gpu.create_render_target(ctx.window_size));
            }
        }

        let frame = ctx.frame.frame_time();
        let fov = ctx.world_camera.fov();
        ctx.components
            
            .buffer_for_each_mut::<Self>(ctx.world, &ctx.gpu, |bunny| {
                let mut linvel = bunny.linvel;
                let mut translation = bunny.position.translation();

                linvel.y += GRAVITY * frame;
                translation += linvel * frame;
                if translation.x >= fov.x {
                    linvel.x = -linvel.x;
                    translation.x = fov.x;
                } else if translation.x <= -fov.x {
                    linvel.x = -linvel.x;
                    translation.x = -fov.x;
                }

                if translation.y < -fov.y {
                    linvel.y = rand::gen_range(0.0..15.0);
                    translation.y = -fov.y;
                } else if translation.y > fov.y {
                    linvel.y = -1.0;
                    translation.y = fov.y;
                }
                bunny.linvel = linvel;
                bunny.position.set_translation(translation);
            });
    }

    fn render<'a>(renderer: &mut ComponentRenderer<'a>) {
        let resources = renderer.resource::<Resources>().single();
        renderer.render_all::<Bunny>(
            RenderCamera::World,
            |r, instances| {
                r.render_sprite(instances, &resources.bunny_model, &resources.bunny_sprite);

                r.queue_text(
                    RenderCamera::RelativeTopRight,
                    &resources.font,
                    vec![TextSection {
                        position: Vector::new(0.0, 0.0),
                        text: vec![Text::new(&format!(
                            "FPS: {}\nBunnies: {}",
                            renderer.ctx.frame.fps(),
                            renderer.ctx.components.len::<Bunny>()
                        ))
                        .with_scale(0.05)
                        .with_color(Color::RED)],
                        alignment: TextAlignment::TopRight,
                        ..Default::default()
                    }],
                );
                r.render_font(&resources.font)
            },
        );
        if let Some(screenshot) = &resources.screenshot {
            renderer.screenshot = Some(screenshot);
        }
    }
}
