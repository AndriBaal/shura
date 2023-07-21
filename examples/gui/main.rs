use shura::*;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene {
        id: 0,
        init: |ctx| {
            ctx.components.register::<GuiComponent>(ctx.groups);
            ctx.components.add(ctx.world, GuiComponent::default());
        },
    });
}

#[derive(Component, Default)]
struct GuiComponent {
    #[base]
    _empty: EmptyComponent,
    demo: egui_demo_lib::DemoWindows,
}

unsafe impl Send for GuiComponent {}
unsafe impl Sync for GuiComponent {}

impl ComponentController for GuiComponent {
    const CONFIG: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::Never,
        ..ComponentConfig::DEFAULT
    };

    fn update(ctx: &mut Context) {
        for gui in ctx.components.iter_mut::<Self>() {
            gui.demo.ui(ctx.gui);
        }
    }
}
