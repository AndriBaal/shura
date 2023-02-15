#![windows_subsystem = "windows"]
use shura::*;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    init("template", |ctx| {
        ctx.create_component(
            None,
            MyComponent {
                component: PositionComponent::new(),
                model: ctx.create_model(ModelBuilder::cuboid(Dimension::new(1.0, 1.0))),
            },
        );
        GameScene {}
    });
}

struct GameScene {}
impl SceneController for GameScene {}

#[derive(Component)]
struct MyComponent {
    #[component]
    component: PositionComponent,
    model: Model,
}

impl ComponentController for MyComponent {
    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        renderer.render_rainbow(&self.model);
        renderer.commit(&instance);
    }
}
