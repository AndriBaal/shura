use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
        ctx.components.register::<ModelTest>();
        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(-3.0, 3.0),
                ctx.gpu
                    .create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.5))),
                ctx.gpu.create_uniform(Color::BLUE),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(-1.0, 3.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::cuboid(Vector::new(0.5, 0.5)),
                    0.25,
                    10,
                )),
                ctx.gpu.create_uniform(Color::CYAN),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(1.0, 3.0),
                ctx.gpu.create_model(ModelBuilder::triangle(
                    Vector::new(0.0, 0.5),
                    Vector::new(-0.5, -0.5),
                    Vector::new(0.5, -0.5),
                )),
                ctx.gpu.create_uniform(Color::BROWN),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
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
                ctx.gpu.create_uniform(Color::LIME),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(-3.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::regular_polygon(0.5, 32)),
                ctx.gpu.create_uniform(Color::NAVY),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(-1.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::regular_polygon(0.5, 5),
                    0.15,
                    5,
                )),
                ctx.gpu.create_uniform(Color::SILVER),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(1.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::segment(
                    Vector::new(0.5, 0.5),
                    Vector::new(-0.5, -0.5),
                    0.2,
                )),
                ctx.gpu.create_uniform(Color::GRAY),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(3.0, 1.0),
                ctx.gpu.create_model(ModelBuilder::rounded(
                    ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                    0.2,
                    5,
                )),
                ctx.gpu.create_uniform(Color::PURPLE),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
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
                ctx.gpu.create_uniform(Color::PINK),
            ),
        );

        ctx.components.add(
            ctx.world,
            GroupHandle::DEFAULT_GROUP,
            ModelTest::new(
                Vector::new(-1.0, -1.0),
                ctx.gpu.create_model(ModelBuilder::star(5, 0.2, 0.8)),
                ctx.gpu.create_uniform(Color::RED),
            ),
        );
    }))
}

#[derive(Component)]
struct ModelTest {
    model: Model,
    color: Uniform<Color>,
    #[base]
    base: PositionComponent,
}

impl ModelTest {
    pub fn new(translation: Vector<f32>, model: Model, color: Uniform<Color>) -> Self {
        Self {
            model,
            color,
            base: PositionComponent::new(PositionBuilder::new().translation(translation)),
        }
    }
}

impl ComponentController for ModelTest {
    const CONFIG: ComponentConfig = ComponentConfig {
        update: UpdateOperation::Never,
        render: RenderOperation::EveryFrame,
        buffer: BufferOperation::Manual,
        ..DEFAULT_CONFIG
    };
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::WORLD, |r, model, index| {
            r.render_color(index, &model.model, &model.color)
        })
    }
}
