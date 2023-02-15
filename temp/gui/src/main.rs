#![windows_subsystem = "windows"]
use shura::*;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    init("postprocessing", |_ctx| GameScene {
        demo: egui_demo_lib::DemoWindows::default(),
    });
}

struct GameScene {
    demo: egui_demo_lib::DemoWindows,
}
impl SceneController for GameScene {
    fn update(&mut self, ctx: &mut Context) {
        self.demo.ui(&ctx.gui());
    }
}
