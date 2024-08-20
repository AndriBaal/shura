use shura::prelude::*;

#[shura::app]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity::<MeshTest>()
            .system(System::render(render))
            .system(System::setup(setup))
    });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            renderer.draw_color_mesh(
                &ctx.default_assets.world_camera2d,
                &ctx.assets.write_mesh_once("meshes", || {
                    ctx.entities.meshes::<MeshTest, _, _>(|mesh| &mesh.mesh)
                }),
            );
        },
    );
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Min(10.0));
    let mut meshes = ctx.entities.get_mut::<MeshTest>();
    meshes.add_many(
        ctx.world,
        [
            MeshTest {
                mesh: MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5))
                    .apply_vertex_translation(Vector2::new(-3.0, 3.0))
                    .apply_data(Color::BLUE),
            },
            MeshTest {
                mesh: MeshBuilder2D::rounded(
                    MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5)),
                    RoundingDirection::Outward,
                    0.25,
                    10,
                )
                .apply_vertex_translation(Vector2::new(-1.0, 3.0))
                .apply_data(Color::CYAN),
            },
            MeshTest {
                mesh: MeshBuilder2D::compound(&[
                    MeshBuilder2D::segment(Vector2::new(0.5, 0.5), Vector2::new(-0.5, -0.5), 0.2),
                    MeshBuilder2D::rounded(
                        MeshBuilder2D::segment(
                            Vector2::new(-0.5, 0.5),
                            Vector2::new(0.5, -0.5),
                            0.2,
                        ),
                        RoundingDirection::Outward,
                        0.2,
                        5,
                    ),
                ])
                .apply_vertex_translation(Vector2::new(-3.0, -1.0))
                .apply_data(Color::PINK),
            },
        ],
    );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(-1.0, 3.0),
    // ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
    //     MeshBuilder2D::cuboid(Vector2::new(0.5, 0.5)),
    //     RoundingDirection::Outward,
    //     0.25,
    //     10,
    // )),
    //         Color::CYAN,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(1.0, 3.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::triangle(
    //             Vector2::new(0.0, 0.5),
    //             Vector2::new(-0.5, -0.5),
    //             Vector2::new(0.5, -0.5),
    //         )),
    //         Color::BROWN,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(3.0, 3.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
    //             MeshBuilder2D::triangle(
    //                 Vector2::new(0.5, 0.5),
    //                 Vector2::new(-0.5, -0.5),
    //                 Vector2::new(0.5, -0.5),
    //             ),
    //             RoundingDirection::Outward,
    //             0.15,
    //             10,
    //         )),
    //         Color::LIME,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(-3.0, 1.0),
    //         ctx.gpu
    //             .create_mesh(&MeshBuilder2D::regular_polygon(0.5, 32)),
    //         Color::NAVY,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(-1.0, 1.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
    //             MeshBuilder2D::regular_polygon(0.5, 5),
    //             RoundingDirection::Outward,
    //             0.15,
    //             5,
    //         )),
    //         Color::SILVER,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(1.0, 1.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::segment(
    //             Vector2::new(0.5, 0.5),
    //             Vector2::new(-0.5, -0.5),
    //             0.2,
    //         )),
    //         Color::GRAY,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(3.0, 1.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::rounded(
    //             MeshBuilder2D::segment(Vector2::new(-0.5, 0.5), Vector2::new(0.5, -0.5), 0.2),
    //             RoundingDirection::Outward,
    //             0.2,
    //             5,
    //         )),
    //         Color::PURPLE,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(-3.0, -1.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::compound(vec![
    //             MeshBuilder2D::segment(Vector2::new(0.5, 0.5), Vector2::new(-0.5, -0.5), 0.2),
    //             MeshBuilder2D::rounded(
    //                 MeshBuilder2D::segment(Vector2::new(-0.5, 0.5), Vector2::new(0.5, -0.5), 0.2),
    //                 RoundingDirection::Outward,
    //                 0.2,
    //                 5,
    //             ),
    //         ])),
    //         Color::PINK,
    //     ),
    // );

    // meshes.add(
    //     ctx.world,
    //     MeshTest::new(
    //         Vector2::new(-1.0, -1.0),
    //         ctx.gpu.create_mesh(&MeshBuilder2D::star(5, 0.2, 0.8)),
    //         Color::RED,
    //     ),
    // );
}

#[derive(Entity)]
struct MeshTest {
    mesh: ColorMeshBuilder2D,
}
