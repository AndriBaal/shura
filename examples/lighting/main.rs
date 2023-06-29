use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
        ctx.window
            .set_inner_size(winit::dpi::PhysicalSize::new(800, 800));
        ctx.window.set_resizable(false);
        ctx.window
            .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
        ctx.components.register::<Background>();
        ctx.components.add(ctx.world, Background::new(ctx));
    }))
}

#[derive(Component)]
struct Background {
    model: Model,
    lightmap: Sprite,
    present: Shader,
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
            lightmap: ctx.gpu.create_sprite(include_bytes!("./lightmap.png")),
            present: ctx.gpu.create_shader(ShaderConfig {
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
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components
            .render_each::<Self>(renderer, RenderCamera::World, |r, background, index| {
                r.render_sprite(index, &background.model, &background.level);
                r.use_shader(&background.present);
                r.use_sprite(&background.lightmap, 1);
                r.draw(index)
            });
    }
}

#[derive(Component)]
struct Light {
    #[base] pos: PositionComponent,
    #[buffer] color: Color
}
