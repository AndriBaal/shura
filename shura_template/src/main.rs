use shura::prelude::*;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .render_group3d("cube", RenderGroupUpdate::EVERY_FRAME)
            .entity::<Cube>()
            .entity_single::<Assets>()
            .system(System::update(update))
            .system(System::setup(setup))
            .system(System::render(render))
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
    ctx.entities.get_mut().add_many(ctx.world, cubes);

    let gpu = ctx.gpu.clone();
    let assets = ctx.assets.clone();
    ctx.tasks
        .spawn(move || { Assets::new(&*assets, &gpu) }, |ctx, res| {
            ctx.entities.single_mut().set(ctx.world, res);
        });
}

fn update(ctx: &mut Context) {
    const SPEED: f32 = 7.0;
    if ctx.entities.single::<Assets>().is_none() {
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

    for cube in ctx.entities.get_mut::<Cube>().iter_mut() {
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
    if let Some(assets) = ctx.entities.single::<Assets>().get() {
        encoder.render3d(
            Some(RgbaColor::new(220, 220, 220, 255).into()),
            |renderer| {
                ctx.group("cube", |buffer| {
                    renderer.draw_model(buffer, ctx.world_camera3d, &assets.model);
                });
            },
        );
    }
}

#[derive(Entity)]
struct Assets {
    model: Model,
}

impl Assets {
    pub fn new(assets: &dyn AssetManager, gpu: &Gpu) -> Self {
        Self {
            model: gpu.create_model(ModelBuilder::asset(assets, "3d/cube/cube.obj")),
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
