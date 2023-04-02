use shura::*;

fn main() {
    Shura::init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Min(10.0));
        ctx.add_component(ModelTest::new(
            Vector::new(-3.0, 3.0),
            ctx.create_model(ModelBuilder::cuboid(Vector::new(0.5, 0.5))),
            ctx.create_uniform(Color::BLUE),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(-1.0, 3.0),
            ctx.create_model(ModelBuilder::rounded(
                ModelBuilder::cuboid(Vector::new(0.5, 0.5)),
                0.25,
                10,
            )),
            ctx.create_uniform(Color::CYAN),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(1.0, 3.0),
            ctx.create_model(ModelBuilder::triangle(
                Vector::new(0.0, 0.5),
                Vector::new(-0.5, -0.5),
                Vector::new(0.5, -0.5),
            )),
            ctx.create_uniform(Color::BROWN),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(3.0, 3.0),
            ctx.create_model(ModelBuilder::rounded(
                ModelBuilder::triangle(
                    Vector::new(0.5, 0.5),
                    Vector::new(-0.5, -0.5),
                    Vector::new(0.5, -0.5),
                ),
                0.15,
                10,
            )),
            ctx.create_uniform(Color::LIME),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(-3.0, 1.0),
            ctx.create_model(ModelBuilder::regular_polygon(0.5, 32)),
            ctx.create_uniform(Color::NAVY),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(-1.0, 1.0),
            ctx.create_model(ModelBuilder::rounded(
                ModelBuilder::regular_polygon(0.5, 5),
                0.15,
                5,
            )),
            ctx.create_uniform(Color::SILVER),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(1.0, 1.0),
            ctx.create_model(ModelBuilder::segment(
                Vector::new(0.5, 0.5),
                Vector::new(-0.5, -0.5),
                0.2,
            )),
            ctx.create_uniform(Color::GRAY),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(3.0, 1.0),
            ctx.create_model(ModelBuilder::rounded(
                ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                0.2,
                5,
            )),
            ctx.create_uniform(Color::PURPLE),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(-3.0, -1.0),
            ctx.create_model(ModelBuilder::compound(vec![
                ModelBuilder::segment(Vector::new(0.5, 0.5), Vector::new(-0.5, -0.5), 0.2),
                ModelBuilder::rounded(
                    ModelBuilder::segment(Vector::new(-0.5, 0.5), Vector::new(0.5, -0.5), 0.2),
                    0.2,
                    5,
                ),
            ])),
            ctx.create_uniform(Color::PINK),
        ));

        ctx.add_component(ModelTest::new(
            Vector::new(-1.0, -1.0),
            ctx.create_model(ModelBuilder::star(5, 0.2, 0.8)),
            ctx.create_uniform(Color::RED),
        ));
    }))
}

#[derive(Component)]
struct ModelTest {
    model: Model,
    color: Uniform<Color>,
    #[base]
    base: BaseComponent,
}

impl ModelTest {
    pub fn new(translation: Vector<f32>, model: Model, color: Uniform<Color>) -> Self {
        Self {
            model,
            color,
            base: BaseComponent::new(PositionBuilder::new().translation(translation)),
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
    fn render(active: ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let mut renderer = encoder.renderer(RenderConfig::WORLD);
        for (buffer, models) in ctx.path_render(&active) {
            for (index, model) in models {
                renderer.render_color(buffer, index, &model.model, &model.color);
            }
        }
    }
}
