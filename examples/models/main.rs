use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(|| {
        NewScene::new(1, |ctx| {
            ctx.world_camera.set_scaling(WorldCameraScaling::Min(10.0));
            ctx.components.register::<MeshTest>(ctx.groups);
            let mut mesh_tests = ctx.components.set_mut::<MeshTest>();
            mesh_tests.add(
                ctx.world,
                MeshTest::new(
                    Vector::new(-3.0, 3.0),
                    ctx.gpu
                        .create_mesh(MeshBuilder::cuboid(Vector::new(0.5, 0.5))),
                    Color::BLUE,
                ),
            );

            mesh_tests.add(
                ctx.world,
                MeshTest::new(
                    Vector::new(-1.0, 3.0),
                    ctx.gpu.create_mesh(MeshBuilder::rounded(
                        MeshBuilder::cuboid(Vector::new(0.5, 0.5)),
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
                    Vector::new(1.0, 3.0),
                    ctx.gpu.create_mesh(MeshBuilder::triangle(
                        Vector::new(0.0, 0.5),
                        Vector::new(-0.5, -0.5),
                        Vector::new(0.5, -0.5),
                    )),
                    Color::BROWN,
                ),
            );

            mesh_tests.add(
                ctx.world,
                MeshTest::new(
                    Vector::new(3.0, 3.0),
                    ctx.gpu.create_mesh(MeshBuilder::rounded(
                        MeshBuilder::triangle(
                            Vector::new(0.5, 0.5),
                            Vector::new(-0.5, -0.5),
                            Vector::new(0.5, -0.5),
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
                    Vector::new(-3.0, 1.0),
                    ctx.gpu.create_mesh(MeshBuilder::regular_polygon(0.5, 32)),
                    Color::NAVY,
                ),
            );

            mesh_tests.add(
                ctx.world,
                MeshTest::new(
                    Vector::new(-1.0, 1.0),
                    ctx.gpu.create_mesh(MeshBuilder::rounded(
                        MeshBuilder::regular_polygon(0.5, 5),
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
                    Vector::new(1.0, 1.0),
                    ctx.gpu.create_mesh(MeshBuilder::segment(
                        Vector::new(0.5, 0.5),
                        Vector::new(-0.5, -0.5),
                        0.2,
                    )),
                    Color::GRAY,
                ),
            );

            mesh_tests.add(
                ctx.world,
                MeshTest::new(
                    Vector::new(3.0, 1.0),
                    ctx.gpu.create_mesh(MeshBuilder::rounded(
                        MeshBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
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
                    Vector::new(-3.0, -1.0),
                    ctx.gpu.create_mesh(MeshBuilder::compound(vec![
                        MeshBuilder::segment(Vector::new(0.5, 0.5), Vector::new(-0.5, -0.5), 0.2),
                        MeshBuilder::rounded(
                            MeshBuilder::segment(
                                Vector::new(-0.5, 0.5),
                                Vector::new(0.5, -0.5),
                                0.2,
                            ),
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
                    Vector::new(-1.0, -1.0),
                    ctx.gpu.create_mesh(MeshBuilder::star(5, 0.2, 0.8)),
                    Color::RED,
                ),
            );
        })
    })
}

#[derive(Component)]
struct MeshTest {
    mesh: Mesh,
    #[position]
    base: PositionComponent,
    #[buffer]
    color: Color,
}

impl MeshTest {
    pub fn new(translation: Vector<f32>, mesh: Mesh, color: Color) -> Self {
        Self {
            mesh,
            color,
            base: PositionComponent::new().with_translation(translation),
        }
    }
}

impl ComponentController for MeshTest {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        components.render_each::<Self>(|renderer, mesh, buffer, instances| {
            renderer.render_color(instances, buffer, renderer.world_camera, &mesh.mesh)
        });
    }
}
