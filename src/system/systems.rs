use crate::{
    context::{Context, RenderContext},
    graphics::RenderEncoder,
    time::{Duration, Instant},
};

pub type SetupSystem = fn(ctx: &mut Context);
pub type ResizeSystem = fn(ctx: &mut Context);
pub type UpdateSystem = fn(ctx: &mut Context);
pub type RenderSystem = fn(ctx: &RenderContext, encoder: &mut RenderEncoder);
pub type EndSystem = fn(&mut Context, reason: EndReason);

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndReason {
    End,
    Removed,
    Replaced,
}

pub enum System {
    Setup(SetupSystem),
    Update(UpdateSystem),
    Resize(ResizeSystem),
    UpdateNFrame(u64, UpdateSystem),
    UpdateAfter(Duration, UpdateSystem),
    Render(RenderSystem),
    End(EndSystem),
}

pub enum UpdateOperation {
    EveryFrame,
    EveryNFrame(u64),
    UpdaterAfter(Instant, Duration),
}

pub struct SystemManager {
    pub setup_systems: Vec<SetupSystem>,
    pub resize_systems: Vec<ResizeSystem>,
    pub update_systems: Vec<(UpdateOperation, UpdateSystem)>,
    pub end_systems: Vec<EndSystem>,
    pub render_systems: Vec<RenderSystem>,
}

impl SystemManager {
    pub fn new() -> Self {
        Self {
            resize_systems: Default::default(),
            update_systems: Default::default(),
            end_systems: Default::default(),
            render_systems: Default::default(),
            setup_systems: Default::default(),
        }
    }

    pub fn register_system(&mut self, system: System) {
        match system {
            System::Update(update) => self
                .update_systems
                .push((UpdateOperation::EveryFrame, update)),
            System::UpdateNFrame(frame, update) => self
                .update_systems
                .push((UpdateOperation::EveryNFrame(frame), update)),
            System::UpdateAfter(duration, update) => self.update_systems.push((
                UpdateOperation::UpdaterAfter(Instant::now(), duration),
                update,
            )),
            System::Render(render) => self.render_systems.push(render),
            System::End(end) => self.end_systems.push(end),
            System::Resize(resize) => self.resize_systems.push(resize),
            System::Setup(setup) => self.setup_systems.push(setup)
        }
    }
}
