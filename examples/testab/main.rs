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
        ctx.components.register::<Test>();
        ctx.components.add(ctx.world, Test::new(ctx));
    }))
}

#[derive(Component)]
struct Test {
    model: Model,
    lightmap: Sprite,
    shader: Shader,
    level: Sprite,
    #[base]
    base: PositionComponent,
}

impl Test {
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx
                .gpu
                .create_model(ModelBuilder::cuboid(vector(10.0, 10.0))),
            level: ctx.gpu.create_sprite(include_bytes!("./level.png")),
            base: Default::default(),
            lightmap: ctx.gpu.create_sprite(include_bytes!("./lightmap.png")),
            shader: ctx.gpu.create_shader(ShaderConfig {
                fragment_source: Shader::SPRITE,
                shader_fields: &[ShaderField::Sprite],
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

impl ComponentController for Test {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components
            .render_each::<Self>(renderer, RenderCamera::World, |r, model, index| {
                r.render_sprite(index, &model.model, &model.level);
                r.use_shader(&model.shader);
                r.use_sprite(&model.lightmap, 1);
                r.draw(index)
            });
    }
}
