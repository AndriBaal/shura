use shura::*;

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Cube>(ComponentConfig {
                buffer: BufferOperation::Manual,
                ..ComponentConfig::DEFAULT
            })
            .component::<Resources>(ComponentConfig::RESOURCE)
            .system(System::Update(update))
            .system(System::Setup(setup))
            .system(System::Render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.components.add(ctx.world, Cube::new());
    ctx.components.add(ctx.world, Resources::new(ctx));
}

fn update(ctx: &mut Context) {}

fn render(res: &ComponentResources, encoder: &mut RenderEncoder) {
    let resources = res.single::<Resources>();
    encoder.render(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            res.render_all::<Cube>(renderer, |renderer, buffer, instances| {
                renderer.renderr_test3d(instances, buffer, &resources.camera, &resources.model);
            });
        },
    );
}

#[derive(Component)]
struct Resources {
    model: Model3D,
    camera: CameraBuffer<PerspectiveCamera3D>,
}

impl Resources {
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx
                .gpu
                .create_model(ModelBuilder3D::cube(Vector3::new(0.2, 0.2, 0.2))),
            camera: ctx
                .gpu
                .create_camera_buffer(&PerspectiveCamera3D::new(ctx.window_size)),
        }
    }
}

#[derive(Component)]
struct Cube {
    #[shura(instance)]
    position: PositionInstance3D,
}

impl Cube {
    pub fn new() -> Cube {
        Cube {
            position: Default::default(),
        }
    }
}
