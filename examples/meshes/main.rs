use shura::*;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .component::<MeshTest>(ComponentConfig {
                buffer: RenderGroupUpdate::Manual,
                ..ComponentConfig::DEFAULT
            })
            .system(System::Render(render))
            .system(System::Setup(setup))
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            res.render_each::<MeshTest>(renderer, |renderer, mesh, buffer, instances| {
                renderer.render_color(instances, buffer, res.world_camera2d, &mesh.mesh)
            });
        },
    );
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Min(10.0));
    let mut mesh_tests = ctx.components.set_mut::<MeshTest>();
    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-3.0, 3.0),
            ctx.gpu
                .create_mesh(&MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5))),
            Color::BLUE,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-1.0, 3.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
                MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5)),
                RoundingDirection::Outward,
                0.25,
                10,
            )),
            Color::CYAN,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(1.0, 3.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::triangle(
                Vector2::new(0.0, 0.5),
                Vector2::new(-0.5, -0.5),
                Vector2::new(0.5, -0.5),
            )),
            Color::BROWN,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(3.0, 3.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
                MeshBuilder2D::triangle(
                    Vector2::new(0.5, 0.5),
                    Vector2::new(-0.5, -0.5),
                    Vector2::new(0.5, -0.5),
                ),
                RoundingDirection::Outward,
                0.15,
                10,
            )),
            Color::LIME,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-3.0, 1.0),
            ctx.gpu
                .create_mesh(&MeshBuilder2D::regular_polygon(0.5, 32)),
            Color::NAVY,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-1.0, 1.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
                MeshBuilder2D::regular_polygon(0.5, 5),
                RoundingDirection::Outward,
                0.15,
                5,
            )),
            Color::SILVER,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(1.0, 1.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::segment(
                Vector2::new(0.5, 0.5),
                Vector2::new(-0.5, -0.5),
                0.2,
            )),
            Color::GRAY,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(3.0, 1.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
                MeshBuilder2D::segment(Vector2::new(-0.5, 0.5), Vector2::new(0.5, -0.5), 0.2),
                RoundingDirection::Outward,
                0.2,
                5,
            )),
            Color::PURPLE,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-3.0, -1.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::compound(vec![
                MeshBuilder2D::segment(Vector2::new(0.5, 0.5), Vector2::new(-0.5, -0.5), 0.2),
                MeshBuilder2D::rounded(
                    MeshBuilder2D::segment(Vector2::new(-0.5, 0.5), Vector2::new(0.5, -0.5), 0.2),
                    RoundingDirection::Outward,
                    0.2,
                    5,
                ),
            ])),
            Color::PINK,
        ),
    );

    mesh_tests.add(
        ctx.world,
        MeshTest::new(
            Vector2::new(-1.0, -1.0),
            ctx.gpu.create_mesh(&MeshBuilder2D::star(5, 0.2, 0.8)),
            Color::RED,
        ),
    );
}

#[derive(Component)]
struct MeshTest {
    mesh: Mesh2D,
    #[shura(instance)]
    instance: PositionInstance2D,
}

impl MeshTest {
    pub fn new(translation: Vector2<f32>, mesh: Mesh2D, color: Color) -> Self {
        Self {
            mesh,
            instance: PositionInstance2D::new()
                .with_translation(translation)
                .with_color(color),
        }
    }
}
