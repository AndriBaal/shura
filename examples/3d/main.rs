use shura::prelude::*;

#[shura::app]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity::<Cube>()
            .system(System::update(update))
            .system(System::setup(setup))
            .system(System::render(render))
    });
}

fn setup(ctx: &mut Context) {
    const NUM_INSTANCES_PER_ROW: u32 = 10;
    const SPACE_BETWEEN: f32 = 3.0;
    ctx.entities.get_mut().add_many(
        ctx.physics,
        (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let position = Vector3::new(x, 0.0, z);

                Cube {
                    position: position.into(),
                }
            })
        }),
    );

    ctx.assets.load_model(
        "cube",
        ModelBuilder::bytes(
            include_resource_str!("3d/cube/cube.obj"),
            &[("cube.mtl", include_resource_str!("3d/cube/cube.mtl"))],
            &[(
                "cobble-diffuse.png",
                include_resource_bytes!("3d/cube/cobble-diffuse.png"),
            )],
        ),
    );
}

fn update(ctx: &mut Context) {
    const SPEED: f32 = 7.0;
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
        cube.position.rotation *= Rotation3::new(Vector3::new(
            1.0 * ctx.time.delta(),
            1.0 * ctx.time.delta(),
            1.0 * ctx.time.delta(),
        ));
    }
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render3d(Some(Color::new_rgba(220, 220, 220, 255)), |renderer| {
        renderer.draw_model(
            &ctx.write_instance_entities::<Cube, _>("cubes", |cube, data| {
                data.push(Instance3D::new(cube.position, Vector3::new(1.0, 1.0, 1.0)))
            }),
            &ctx.assets.model("cube"),
            &ctx.default_assets.world_camera3d,
        );
    });
}

#[derive(Entity)]
struct Cube {
    position: Isometry3<f32>,
}
