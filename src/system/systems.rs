use crate::{
    context::{Context, RenderContext},
    graphics::RenderEncoder,
    time::{Duration, Instant},
};

pub type SetupSystem = Box<dyn Fn(&mut Context)>;
pub type ResizeSystem = Box<dyn Fn(&mut Context)>;
pub type UpdateSystem = Box<dyn Fn(&mut Context)>;
pub type RenderSystem = Box<dyn Fn(&RenderContext, &mut RenderEncoder)>;
pub type EndSystem = Box<dyn Fn(&mut Context, EndReason)>;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndReason {
    Close,
    Removed,
}

enum SystemType {
    Setup(SetupSystem),
    Update(UpdateSystem),
    Resize(ResizeSystem),
    UpdateNFrame(u64, UpdateSystem),
    UpdateAfter(Duration, UpdateSystem),
    Render(RenderSystem),
    End(EndSystem),
}

pub struct System(SystemType);

impl System {
    pub fn setup(system: impl Fn(&mut Context) + 'static) -> Self {
        Self(SystemType::Setup(Box::new(system)))
    }
    pub fn update(system: impl Fn(&mut Context) + 'static) -> Self {
        Self(SystemType::Update(Box::new(system)))
    }
    pub fn resize(system: impl Fn(&mut Context) + 'static) -> Self {
        Self(SystemType::Resize(Box::new(system)))
    }
    pub fn update_nframe(frame: u64, system: impl Fn(&mut Context) + 'static) -> Self {
        Self(SystemType::UpdateNFrame(frame, Box::new(system)))
    }
    pub fn update_after(duration: Duration, system: impl Fn(&mut Context) + 'static) -> Self {
        Self(SystemType::UpdateAfter(duration, Box::new(system)))
    }
    pub fn render(system: impl Fn(&RenderContext, &mut RenderEncoder) + 'static) -> Self {
        Self(SystemType::Render(Box::new(system)))
    }
    pub fn end(system: impl Fn(&mut Context, EndReason) + 'static) -> Self {
        Self(SystemType::End(Box::new(system)))
    }
}

pub enum UpdateOperation {
    EveryFrame,
    EveryNFrame(u64),
    UpdaterAfter(Instant, Duration),
}

#[derive(Default)]
pub struct SystemManager {
    pub setup_systems: Vec<SetupSystem>,
    pub resize_systems: Vec<ResizeSystem>,
    pub update_systems: Vec<(UpdateOperation, UpdateSystem)>,
    pub end_systems: Vec<EndSystem>,
    pub render_systems: Vec<RenderSystem>,
}

impl SystemManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_system(&mut self, system: System) {
        match system.0 {
            SystemType::Update(update) => self
                .update_systems
                .push((UpdateOperation::EveryFrame, update)),
            SystemType::UpdateNFrame(frame, update) => self
                .update_systems
                .push((UpdateOperation::EveryNFrame(frame), update)),
            SystemType::UpdateAfter(duration, update) => self.update_systems.push((
                UpdateOperation::UpdaterAfter(Instant::now(), duration),
                update,
            )),
            SystemType::Render(render) => self.render_systems.push(render),
            SystemType::End(end) => self.end_systems.push(end),
            SystemType::Resize(resize) => self.resize_systems.push(resize),
            SystemType::Setup(setup) => self.setup_systems.push(setup),
        }
    }
}
