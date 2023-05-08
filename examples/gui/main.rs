use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 0,
        init: |ctx| ctx.insert_scene_state(GuiState::default()),
    });
}

#[derive(State, Default)]
struct GuiState {
    demo: egui_demo_lib::DemoWindows,
}

impl SceneStateController for GuiState {
    fn update(ctx: &mut Context) {
        let state = ctx.scene_states.get_mut::<Self>();
        state.demo.ui(ctx.gui);
    }
}
