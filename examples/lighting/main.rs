use shura::*;

const SIZE: Vector<u32> = vector(800, 800);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        register!(ctx.components, [Background, Light]);
        ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
        ctx.window
            .set_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));
        ctx.window.set_resizable(false);
        ctx.window
            .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
        ctx.scene_states.insert(LightResources::new(ctx));
        ctx.components.add(ctx.world, Background::new(ctx));

        ctx.components.add(
            ctx.world,
            Light::new(vector(0.0, -2.0), Color::ORANGE, 6.0, false),
        );

        ctx.components.add(
            ctx.world,
            Light::new(vector(0.0, 0.0), Color::WHITE, 5.0, true),
        );
    }))
}

#[derive(State)]
pub struct LightResources {
    light_model: Model,
    light_map: RenderTarget,
    light_shader: Shader,
    present_shader: Shader,
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
            light_map: ctx.gpu.create_render_target(vector(SIZE.y, SIZE.y)),
            light_model: ctx.gpu.create_model(ModelBuilder::cuboid(vector(1.0, 1.0))),
        }
    }
}

#[derive(Component)]
struct Background {
    model: Model,
    level: Sprite,
    #[base]
    base: PositionComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx
                .gpu
                .create_model(ModelBuilder::cuboid(vector(10.0, 10.0))),
            level: ctx.gpu.create_sprite(include_bytes!("./level.png")),
            base: Default::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        storage: ComponentStorage::Single,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        let res = ctx.scene_states.get::<LightResources>();
        ctx.components.render_single::<Self>(
            renderer,
            RenderCamera::World,
            |r, background, index| {
                r.render_sprite(index, &background.model, &background.level);
                r.use_model(ctx.defaults.unit_camera.0.model());
                r.use_camera(RenderCamera::Unit);
                r.use_shader(&res.present_shader);
                r.use_sprite(res.light_map.sprite(), 1);
                r.draw(index)
            },
        );
    }
}

#[derive(Component)]
struct Light {
    #[base]
    pos: PositionComponent,
    #[buffer]
    color: Color,
    follow_cursor: bool,
}

impl Light {
    pub fn new(translation: Vector<f32>, color: Color, radius: f32, follow_cursor: bool) -> Self {
        Self {
            pos: PositionComponent::new()
                .with_translation(translation)
                .with_scale(vector(radius, radius)),
            color,
            follow_cursor,
        }
    }
}

impl ComponentController for Light {
    const CONFIG: ComponentConfig = ComponentConfig {
        render_priority: 15,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
        for light in ctx.components.iter_mut::<Self>() {
            if light.follow_cursor {
                light
                    .pos
                    .set_translation(ctx.input.cursor(ctx.world_camera));
            }
        }
    }

    fn render_target<'a>(ctx: &'a Context) -> (Option<Color>, &'a RenderTarget) {
        let res = ctx.scene_states.get::<LightResources>();
        return (
            Some(Color::new(
                0.12941176470588237,
                0.1803921568627451,
                0.27450980392156865,
                1.0,
            )),
            &res.light_map,
        );
    }

    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        let res = ctx.scene_states.get::<LightResources>();
        ctx.components
            .render_all::<Self>(renderer, RenderCamera::World, |r, i| {
                r.use_shader(&res.light_shader);
                r.use_model(&res.light_model);
                r.draw(i)
            });
    }
}
