use shura::*;

const SIZE: Vector<u32> = vector(800, 800);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(|| {
        NewScene::new(1, |ctx| {
            register!(ctx, [Background, Light, LightResources]);
            ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
            ctx.window
                .set_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));
            ctx.window.set_resizable(false);
            ctx.window
                .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
            ctx.components.add(ctx.world, LightResources::new(ctx));
            ctx.components.add(ctx.world, Background::new(ctx));

            ctx.components.add(
                ctx.world,
                Light::new(vector(0.0, -2.0), Color::ORANGE, 6.0, false),
            );

            ctx.components.add(
                ctx.world,
                Light::new(vector(0.0, 0.0), Color::WHITE, 5.0, true),
            );
        })
    })
}

#[derive(Component)]
struct Background {
    model: Model,
    level: Sprite,
    #[position]
    position: PositionComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx
                .gpu
                .create_model(ModelBuilder::cuboid(vector(10.0, 10.0))),
            level: ctx.gpu.create_sprite(sprite_file!("./level.png")),
            position: Default::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        storage: ComponentStorage::Single,
        render_priority: 1,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(renderer: &mut ComponentRenderer<'a>) {
        renderer.render_single::<Self>(RenderCamera::World, |r, background, index| {
            r.render_sprite(index, &background.model, &background.level);
        });
    }
}

#[derive(Component)]
pub struct LightResources {
    light_model: Model,
    light_map: RenderTarget,
    light_shader: Shader,
    present_shader: Shader,
}

impl ComponentController for LightResources {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        storage: ComponentStorage::Single,
        render_priority: 3,
        ..ComponentConfig::DEFAULT
    };

    fn render<'a>(renderer: &mut ComponentRenderer<'a>) {
        renderer.render_single::<Self>(RenderCamera::World, |r, res, index| {
            r.use_model(r.defaults.unit_model());
            r.use_camera(RenderCamera::Unit);
            r.use_shader(&res.present_shader);
            r.use_sprite(res.light_map.sprite(), 1);
            r.draw(index);
        });
    }
}

impl LightResources {
    pub fn new(ctx: &Context) -> Self {
        Self {
            present_shader: ctx.gpu.create_shader(ShaderConfig {
                fragment_source: Shader::SPRITE,
                uniforms: &[UniformField::Sprite],
                blend: BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::Zero,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                },
                ..Default::default()
            }),
            light_shader: ctx.gpu.create_shader(ShaderConfig {
                fragment_source: include_str!("./light.wgsl"),
                uniforms: &[],
                instance_fields: &[InstanceField {
                    format: VertexFormat::Float32x4,
                    field_name: "color",
                    data_type: "vec4<f32>",
                }],
                ..Default::default()
            }),
            light_map: ctx.gpu.create_render_target(ctx.window_size),
            light_model: ctx.gpu.create_model(ModelBuilder::cuboid(vector(1.0, 1.0))),
        }
    }
}

#[derive(Component)]
struct Light {
    #[position]
    pos: PositionComponent,
    #[buffer]
    color: Color,
    follow_player: bool,
}

impl Light {
    pub fn new(translation: Vector<f32>, color: Color, radius: f32, follow_player: bool) -> Self {
        Self {
            pos: PositionComponent::new()
                .with_translation(translation)
                .with_scale(vector(radius, radius)),
            color,
            follow_player,
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        render_priority: 2,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
        if ctx.screen_config.resized() {
            let mut res = ctx.components.single_mut::<LightResources>();
            res.light_map.resize(&ctx.gpu, ctx.window_size);
        }

        ctx.components.for_each_mut::<Self>(|light| {
            if light.follow_player {
                light.pos.set_translation(ctx.cursor);
            }
        });
    }

    fn render_target<'a>(
        renderer: &mut ComponentRenderer<'a>,
    ) -> (Option<Color>, &'a RenderTarget) {
        let res = renderer.single::<LightResources>();
        return (Some(Color::new(0.06, 0.08, 0.13, 1.0)), &res.light_map);
    }

    fn render<'a>(renderer: &mut ComponentRenderer<'a>) {
        let res = renderer.single::<LightResources>();
        renderer.render_all::<Self>(RenderCamera::World, |r, i| {
            r.use_shader(&res.light_shader);
            r.use_model(&res.light_model);
            r.draw(i);
        });
    }
}
