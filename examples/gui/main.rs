use shura::*;

#[shura::main]
fn shura_main(config: AppConfig) {
    App::run(config, || {
        NewScene::new(1)
            .component::<Demo>(ComponentConfig::RESOURCE)
            .system(System::Update(update))
            .system(System::Setup(setup))
    });
}

#[derive(Component, Default)]
struct Demo {
    demo: egui_demo_lib::DemoWindows,
}

fn setup(ctx: &mut Context) {
    ctx.components.add(ctx.world, Demo::default());
}

fn update(ctx: &mut Context) {
    let mut gui = ctx.components.single::<Demo>();
    gui.demo.ui(ctx.gui);
}
