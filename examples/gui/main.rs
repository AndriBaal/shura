use shura::prelude::*;

#[shura::app]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity_single::<Demo>()
            .system(System::update(update))
            .system(System::setup(setup))
    });
}

#[derive(Entity, Default)]
struct Demo {
    demo: egui_demo_lib::DemoWindows,
}

fn setup(ctx: &mut Context) {
    ctx.entities
        .single_mut::<Demo>()
        .set(ctx.world, Demo::default());
}

fn update(ctx: &mut Context) {
    let mut gui = ctx.entities.single_mut::<Demo>().unwrap();
    gui.demo.ui(ctx.gui);
}
