use shura::prelude::*;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .entity::<Demo>(EntityStorage::Single, Default::default())
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
        .single::<Demo>()
        .set(ctx.world, Demo::default());
}

fn update(ctx: &mut Context) {
    let mut gui = ctx.entities.single::<Demo>().get_mut().unwrap();
    gui.demo.ui(ctx.gui);
}
