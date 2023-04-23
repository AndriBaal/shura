use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 0,
        init: |ctx| {
            ctx.set_scene_state(GuiState::default())
        },
    });
}

#[derive(State, Default)]
struct GuiState {
    demo: egui_demo_lib::DemoWindows,
}
impl SceneState for GuiState {
    fn update(ctx: &mut Context) {
        let state = ctx.scene_state.downcast_mut::<Self>().unwrap();
        state.demo.ui(ctx.gui);
    }
}
