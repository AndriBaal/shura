use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
        ctx.components.register::<ModelTest>(ctx.groups);
        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-3.0, 3.0),
                ctx.gpu
                    .create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.5))),
                Color::BLUE,
            ),
        );

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-1.0, 3.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::cuboid(Vector::new(0.5, 0.5)),
                    0.25,
                    10,
                )),
                Color::CYAN,
            ),
        );

        ctx.components.add(
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

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(3.0, 3.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::triangle(
                        Vector::new(0.5, 0.5),
                        Vector::new(-0.5, -0.5),
                        Vector::new(0.5, -0.5),
                    ),
                    0.15,
                    10,
                )),
                Color::LIME,
            ),
        );

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-3.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::regular_polygon(0.5, 32)),
                Color::NAVY,
            ),
        );

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-1.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::regular_polygon(0.5, 5),
                    0.15,
                    5,
                )),
                Color::SILVER,
            ),
        );

        ctx.components.add(
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

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(3.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                    0.2,
                    5,
                )),
                Color::PURPLE,
            ),
        );

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-3.0, -1.0),
                ctx.gpu.create_model(ModelBuilder::compound(vec![
                    ModelBuilder::segment(Vector::new(0.5, 0.5), Vector::new(-0.5, -0.5), 0.2),
                    ModelBuilder::rounded(
                        ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                        0.2,
                        5,
                    ),
                ])),
                Color::PINK,
            ),
        );

        ctx.components.add(
            ctx.world,
            ModelTest::new(
                Vector::new(-1.0, -1.0),
                ctx.gpu.create_model(ModelBuilder::star(5, 0.2, 0.8)),
                Color::RED,
            ),
        );
    }))
}

#[derive(Component)]
struct ModelTest {
    model: Model,
    #[base]
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

    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components
            .render_each::<Self>(renderer, RenderCamera::World, |r, model, index| {
                r.render_color(index, &model.model)
            });
    }
}
