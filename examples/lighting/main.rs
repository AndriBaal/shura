use shura::*;

const SIZE: Vector<u32> = vector(800, 800);

#[shura::main]
fn app(config: ShuraConfig) {
    config.init(|| {
        NewScene::new(1, |ctx| {
            register!(ctx, [Background, Light, LightAssets]);
            ctx.world_camera.set_scaling(WorldCameraScaling::Min(10.0));
            ctx.window
                .set_inner_size(winit::dpi::PhysicalSize::new(SIZE.x, SIZE.y));
            ctx.window.set_resizable(false);
            ctx.window
                .set_enabled_buttons(winit::window::WindowButtons::CLOSE);
            ctx.components.add(ctx.world, LightAssets::new(ctx));
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
    mesh: Mesh,
    level: Sprite,
    #[position]
    position: PositionComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        Self {
            mesh: ctx
                .gpu
                .create_mesh(&MeshBuilder::cuboid(vector(10.0, 10.0))),
            level: ctx.gpu.create_sprite(sprite_file!("./level.png")),
            position: Default::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: RenderGroupConfig::Manual,
        storage: ComponentStorage::Single,
        render_priority: 1,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        components.render_single::<Self>(|renderer, background, buffer, instances| {
            renderer.render_sprite(
                instances,
                buffer,
                renderer.world_camera,
                &background.mesh,
                &background.level,
            );
        });
    }
}

#[derive(Component)]
pub struct LightAssets {
    light_mesh: Mesh,
    light_map: SpriteRenderTarget,
    light_shader: Shader,
    present_shader: Shader,
}

impl ComponentController for LightAssets {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: RenderGroupConfig::Never,
        storage: ComponentStorage::Single,
        render_priority: 3,
        ..ComponentConfig::DEFAULT
    };

    fn render_target<'a>(
        components: &mut ComponentRenderer<'a>,
    ) -> Option<(Option<Color>, &'a dyn RenderTarget)> {
        return Some((None, components.ctx.default_resources.default_target()));
    }

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        let res = components.single::<LightAssets>();
        let renderer = &mut components.renderer;
        renderer.use_mesh(renderer.unit_mesh);
        renderer.use_camera(&renderer.unit_camera);
        renderer.use_instances(renderer.single_centered_instance);
        renderer.use_shader(&res.present_shader);
        renderer.use_sprite(res.light_map.sprite(), 1);
        renderer.draw(0..1);
    }
}

impl LightAssets {
    pub fn new(ctx: &Context) -> Self {
        Self {
            present_shader: ctx.gpu.create_shader(ShaderConfig {
                fragment_shader: Shader::SPRITE,
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
                fragment_shader: include_str!("./light.wgsl"),
                uniforms: &[],
                vertex_shader: VertexShader::AutoInstance(&[InstanceField {
                    format: VertexFormat::Float32x4,
                    field_name: "color",
                    data_type: "vec4<f32>",
                }]),
                ..Default::default()
            }),
            light_map: ctx.gpu.create_render_target(ctx.window_size),
            light_mesh: ctx.gpu.create_mesh(&MeshBuilder::cuboid(vector(1.0, 1.0))),
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
        if ctx.resized {
            let mut res = ctx.components.single_mut::<LightAssets>();
            res.light_map.resize(&ctx.gpu, ctx.window_size);
        }

        ctx.components.for_each_mut::<Self>(|light| {
            if light.follow_player {
                light.pos.set_translation(ctx.cursor);
            }
        });
    }

    fn render_target<'a>(
        components: &mut ComponentRenderer<'a>,
    ) -> Option<(Option<Color>, &'a dyn RenderTarget)> {
        let res = components.single::<LightAssets>();
        return Some((Some(Color::new(0.06, 0.08, 0.13, 1.0)), &res.light_map));
    }

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        let res = components.single::<LightAssets>();
        components.render::<Self>(|renderer, buffer, instances| {
            renderer.use_instances(buffer);
            renderer.use_camera(renderer.world_camera);
            renderer.use_shader(&res.light_shader);
            renderer.use_mesh(&res.light_mesh);
            renderer.draw(instances);
        });
    }
}
