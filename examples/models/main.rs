use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(|| {
        NewScene::new(1, |ctx| {
            ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
            ctx.components.register::<ModelTest>(ctx.groups);
            let mut model_tests = ctx.components.set_mut::<ModelTest>();
            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-3.0, 3.0),
                    ctx.gpu
                        .create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.5))),
                    Color::BLUE,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-1.0, 3.0),
                    ctx.gpu.create_model(ModelBuilder::rounded(
                        ModelBuilder::cuboid(Vector::new(0.5, 0.5)),
                        RoundingDirection::Outward,
                        0.25,
                        10,
                    )),
                    Color::CYAN,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(1.0, 3.0),
                    ctx.gpu.create_model(ModelBuilder::triangle(
                        Vector::new(0.0, 0.5),
                        Vector::new(-0.5, -0.5),
                        Vector::new(0.5, -0.5),
                    )),
                    Color::BROWN,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(3.0, 3.0),
                    ctx.gpu.create_model(ModelBuilder::rounded(
                        ModelBuilder::triangle(
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

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-3.0, 1.0),
                    ctx.gpu.create_model(ModelBuilder::regular_polygon(0.5, 32)),
                    Color::NAVY,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-1.0, 1.0),
                    ctx.gpu.create_model(ModelBuilder::rounded(
                        ModelBuilder::regular_polygon(0.5, 5),
                        RoundingDirection::Outward,
                        0.15,
                        5,
                    )),
                    Color::SILVER,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(1.0, 1.0),
                    ctx.gpu.create_model(ModelBuilder::segment(
                        Vector::new(0.5, 0.5),
                        Vector::new(-0.5, -0.5),
                        0.2,
                    )),
                    Color::GRAY,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(3.0, 1.0),
                    ctx.gpu.create_model(ModelBuilder::rounded(
                        ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                        RoundingDirection::Outward,
                        0.2,
                        5,
                    )),
                    Color::PURPLE,
                ),
            );

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-3.0, -1.0),
                    ctx.gpu.create_model(ModelBuilder::compound(vec![
                        ModelBuilder::segment(Vector::new(0.5, 0.5), Vector::new(-0.5, -0.5), 0.2),
                        ModelBuilder::rounded(
                            ModelBuilder::segment(
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

            model_tests.add(
                ctx.world,
                ModelTest::new(
                    Vector::new(-1.0, -1.0),
                    ctx.gpu.create_model(ModelBuilder::star(5, 0.2, 0.8)),
                    Color::RED,
                ),
            );
        })
    })
}

#[derive(Component)]
struct ModelTest {
    model: Model,
    #[position]
    base: PositionComponent,
    #[buffer]
    color: Color,
}

impl ModelTest {
    pub fn new(translation: Vector<f32>, model: Model, color: Color) -> Self {
        Self {
            model,
            color,
            base: PositionComponent::new().with_translation(translation),
        }
    }
}

impl ComponentController for ModelTest {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };

    fn render<'a>(components: &mut ComponentRenderer<'a>) {
        components.render_each::<Self>(|renderer, model, buffer, instances| {
            renderer.render_color(instances, buffer, renderer.world_camera, &model.model)
        });
    }
}
