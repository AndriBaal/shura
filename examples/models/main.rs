use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.world_camera.set_scaling(WorldCameraScale::Min(10.0));
        ctx.components.register::<ModelTest>();
        ctx.components.add(ModelTest::new(
            Vector::new(-3.0, 3.0),
            ctx.gpu
                .create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.5))),
            ctx.gpu.create_color(Color::BLUE),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(-1.0, 3.0),
            ctx.gpu.create_model(ModelBuilder::rounded(
                ModelBuilder::cuboid(Vector::new(0.5, 0.5)),
                0.25,
                10,
            )),
            ctx.gpu.create_color(Color::CYAN),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(1.0, 3.0),
            ctx.gpu.create_model(ModelBuilder::triangle(
                Vector::new(0.0, 0.5),
                Vector::new(-0.5, -0.5),
                Vector::new(0.5, -0.5),
            )),
            ctx.gpu.create_color(Color::BROWN),
        ));

        ctx.components.add(ModelTest::new(
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
            ctx.gpu.create_color(Color::LIME),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(-3.0, 1.0),
            ctx.gpu.create_model(ModelBuilder::regular_polygon(0.5, 32)),
            ctx.gpu.create_color(Color::NAVY),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(-1.0, 1.0),
            ctx.gpu.create_model(ModelBuilder::rounded(
                ModelBuilder::regular_polygon(0.5, 5),
                0.15,
                5,
            )),
            ctx.gpu.create_color(Color::SILVER),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(1.0, 1.0),
            ctx.gpu.create_model(ModelBuilder::segment(
                Vector::new(0.5, 0.5),
                Vector::new(-0.5, -0.5),
                0.2,
            )),
            ctx.gpu.create_color(Color::GRAY),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(3.0, 1.0),
            ctx.gpu.create_model(ModelBuilder::rounded(
                ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                0.2,
                5,
            )),
            ctx.gpu.create_color(Color::PURPLE),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(-3.0, -1.0),
            ctx.gpu.create_model(ModelBuilder::compound(vec![
                ModelBuilder::segment(Vector::new(0.5, 0.5), Vector::new(-0.5, -0.5), 0.2),
                ModelBuilder::rounded(
                    ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                    0.2,
                    5,
                ),
            ])),
            ctx.gpu.create_color(Color::PINK),
        ));

        ctx.components.add(ModelTest::new(
            Vector::new(-1.0, -1.0),
            ctx.gpu.create_model(ModelBuilder::star(5, 0.2, 0.8)),
            ctx.gpu.create_color(Color::RED),
        ));
    }))
}

#[derive(Component)]
struct ModelTest {
    model: Model,
    color: Sprite,
    #[base]
    base: PositionComponent,
}

impl ModelTest {
    pub fn new(translation: Vector<f32>, model: Model, color: Sprite) -> Self {
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
        buffer: BufferOperation::Manual,
        ..ComponentConfig::DEFAULT
    };
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {
        ctx.components
            .render_each::<Self>(renderer, RenderCamera::World, |r, model, index| {
                r.render_sprite(index, &model.model, &model.color)
            });
    }
}
