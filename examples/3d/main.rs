use shura::*;

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Instance3D>("cube", BufferConfig::EveryFrame)
            .entity::<Cube>(EntityConfig {
                ..EntityConfig::DEFAULT
            })
            .entity::<Resources>(EntityConfig::RESOURCE)
            .system(System::Update(update))
            .system(System::Setup(setup))
            .system(System::Render(render))
    });
}

fn setup(ctx: &mut Context) {
    const NUM_INSTANCES_PER_ROW: u32 = 10;
    const SPACE_BETWEEN: f32 = 3.0;
    let cubes = (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let position = Vector3::new(x, 0.0, z);

                Cube::new(position)
            })
        })
        .collect::<Vec<_>>();
    ctx.entities.add_many(ctx.world, cubes);
    // ctx.entities.add(ctx.world, Resources::new(ctx));

    let gpu = ctx.gpu.clone();
    ctx.tasks
        .spawn_async(async move { Resources::new(&gpu).await }, |ctx, res| {
            ctx.entities.add(ctx.world, res);
        });
}

fn update(ctx: &mut Context) {
    const SPEED: f32 = 7.0;
    if ctx.entities.set::<Resources>().len() < 1 {
        return;
    }

    let speed = SPEED * ctx.frame.frame_time();
    let camera = ctx.world_camera3d.perspective_mut().unwrap();

    let forward = camera.target - camera.eye;
    let forward_norm = forward.normalize();
    let forward_mag = forward.magnitude();

    if ctx.input.is_held(Key::Up) && forward_mag > speed {
        camera.eye += forward_norm * speed;
    }
    if ctx.input.is_held(Key::Down) {
        camera.eye -= forward_norm * speed;
    }

    let right = forward_norm.cross(&camera.up);
    let forward = camera.target - camera.eye;
    let forward_mag = forward.magnitude();

    if ctx.input.is_held(Key::Right) {
        camera.eye = camera.target - (forward + right * speed).normalize() * forward_mag;
    }

    if ctx.input.is_held(Key::Left) {
        camera.eye = camera.target - (forward - right * speed).normalize() * forward_mag;
    }

    ctx.entities.set::<Cube>().for_each_mut(|cube| {
        let mut rot = cube.position.rotation();
        rot *= Rotation3::new(Vector3::new(
            1.0 * ctx.frame.frame_time(),
            1.0 * ctx.frame.frame_time(),
            1.0 * ctx.frame.frame_time(),
        ));
        cube.position.set_rotation(rot);
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    if let Some(resources) = ctx.try_single::<Resources>() {
        encoder.render3d(
            Some(RgbaColor::new(220, 220, 220, 255).into()),
            |renderer| {
                ctx.render_all(renderer, "cube", |renderer, buffer, instances| {
                    renderer.render_model(instances, buffer, &ctx.world_camera3d, &resources.model);
                });
            },
        );
    }
}

#[derive(Entity)]
struct Resources {
    model: Model,
}

impl Resources {
    pub async fn new(gpu: &Gpu) -> Self {
        Self {
            model: gpu.create_model(ModelBuilder::file("3d/cube/cube.obj").await),
        }
    }
}

#[derive(Entity)]
struct Cube {
    #[shura(component="cube")]
    position: PositionComponent3D,
}

impl Cube {
    pub fn new(position: Vector3<f32>) -> Cube {
        Cube {
            position: PositionComponent3D::new().with_translation(position), // .with_scaling(Vector3::new(0.001, 0.001, 0.001)),
        }
    }
}
