use shura::prelude::*;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .render_group3d("cube", RenderGroupConfig::EVERY_FRAME)
            .entities::<Cube>(Default::default())
            .single_entity::<Resources>(Default::default())
            .system(System::Update(update))
            .system(System::Setup(setup))
            .system(System::Render(render))
    });
}

fn setup(ctx: &mut Context) {
    const NUM_INSTANCES_PER_ROW: u32 = 10;
    const SPACE_BETWEEN: f32 = 3.0;
    ctx.entities.multiple().add_many(
        ctx.world,
        (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let position = Vector3::new(x, 0.0, z);

                Cube::new(position)
            })
        }),
    );

    let gpu = ctx.gpu.clone();
    ctx.tasks.spawn(
        move || Resources::new(&gpu),
        |ctx, res| {
            ctx.entities.single().set(ctx.world, res);
        },
    );
}

fn update(ctx: &mut Context) {
    const SPEED: f32 = 7.0;
    if ctx.entities.single::<Resources>().is_none() {
        return;
    }
    let speed = SPEED * ctx.time.delta();
    let camera = ctx.world_camera3d.perspective_mut().unwrap();

    let forward = camera.target - camera.eye;
    let forward_norm = forward.normalize();
    let forward_mag = forward.magnitude();

    if ctx.input.is_held(Key::KeyW) && forward_mag > speed {
        camera.eye += forward_norm * speed;
    }
    if ctx.input.is_held(Key::KeyS) {
        camera.eye -= forward_norm * speed;
    }

    let right = forward_norm.cross(&camera.up);
    let forward = camera.target - camera.eye;
    let forward_mag = forward.magnitude();

    if ctx.input.is_held(Key::KeyD) {
        camera.eye = camera.target - (forward + right * speed).normalize() * forward_mag;
    }

    if ctx.input.is_held(Key::KeyA) {
        camera.eye = camera.target - (forward - right * speed).normalize() * forward_mag;
    }

    for cube in ctx.entities.multiple::<Cube>().iter_mut() {
        let mut rot = cube.position.rotation();
        rot *= Rotation3::new(Vector3::new(
            1.0 * ctx.time.delta(),
            1.0 * ctx.time.delta(),
            1.0 * ctx.time.delta(),
        ));
        cube.position.set_rotation(rot);
    }
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    if let Some(resources) = ctx.entities.single::<Resources>().get() {
        encoder.render3d(
            Some(RgbaColor::new(220, 220, 220, 255).into()),
            |renderer| {
                ctx.render(renderer, "cube", |renderer, buffer, instances| {
                    renderer.render_model(instances, buffer, ctx.world_camera3d, &resources.model);
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
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            model: gpu.create_model(ModelBuilder::bytes(
                include_str_res!("3d/cube/cube.obj"),
                &[("cube.mtl", include_str_res!("3d/cube/cube.mtl"))],
                &[(
                    "cobble-diffuse.png",
                    include_bytes_res!("3d/cube/cobble-diffuse.png"),
                )],
            )),
        }
    }
}

#[derive(Entity)]
struct Cube {
    #[shura(component = "cube")]
    position: PositionComponent3D,
}

impl Cube {
    pub fn new(position: Vector3<f32>) -> Cube {
        Cube {
            position: PositionComponent3D::new().with_translation(position), // .with_scaling(Vector3::new(0.001, 0.001, 0.001)),
        }
    }
}
