use crate::{
    context::{Context, RenderContext},
    graphics::RenderEncoder,
    time::{Duration, Instant},
};

pub type SetupSystem = Box<dyn FnOnce(&mut Context)>;
pub type ResizeSystem = Box<dyn Fn(&mut Context)>;
pub type UpdateSystem = Box<dyn Fn(&mut Context)>;
pub type SwitchSystem = Box<dyn Fn(&mut Context, u32)>;
pub type RenderSystem = Box<dyn Fn(&RenderContext, &mut RenderEncoder)>;
pub type EndSystem = Box<dyn Fn(&mut Context, EndReason)>;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndReason {
    Close,
    Removed,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SystemPriority(pub u8);
impl SystemPriority {
    pub const FIRST: Self = Self(0);
    pub const BEFORE: Self = Self(64);
    pub const DURING: Self = Self(128);
    pub const AFTER: Self = Self(192);
    pub const LAST: Self = Self(255);
}

impl Default for SystemPriority {
    #[inline]
    fn default() -> Self {
        SystemPriority::DURING
    }
}

enum SystemType {
    Setup(SetupSystem),
    Update(UpdateSystem),
    UpdateNFrame(u64, UpdateSystem),
    UpdateAfter(Duration, UpdateSystem),
    Resize(ResizeSystem),
    Switch(SwitchSystem),
    Render(RenderSystem),
    End(EndSystem),
    // TODO: Custom callable event
}

pub struct System {
    pub priority: SystemPriority,
    system_type: SystemType,
}

impl System {
    pub fn setup(system: impl FnOnce(&mut Context) + 'static) -> Self {
        Self {
            system_type: SystemType::Setup(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn update(system: impl Fn(&mut Context) + 'static) -> Self {
        Self {
            system_type: SystemType::Update(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn switch(system: impl Fn(&mut Context, u32) + 'static) -> Self {
        Self {
            system_type: SystemType::Switch(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn resize(system: impl Fn(&mut Context) + 'static) -> Self {
        Self {
            system_type: SystemType::Resize(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn update_nframe(frame: u64, system: impl Fn(&mut Context) + 'static) -> Self {
        Self {
            system_type: SystemType::UpdateNFrame(frame, Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn update_after(duration: Duration, system: impl Fn(&mut Context) + 'static) -> Self {
        Self {
            system_type: SystemType::UpdateAfter(duration, Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn render(system: impl Fn(&RenderContext, &mut RenderEncoder) + 'static) -> Self {
        Self {
            system_type: SystemType::Render(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }
    pub fn end(system: impl Fn(&mut Context, EndReason) + 'static) -> Self {
        Self {
            system_type: SystemType::End(Box::new(system)),
            priority: SystemPriority::default(),
        }
    }

    pub fn priority(mut self, priority: SystemPriority) -> Self {
        self.priority = priority;
        self
    }
}

pub enum UpdateOperation {
    EveryFrame,
    EveryNFrame(u64),
    UpdaterAfter(Instant, Duration),
}

#[derive(Default)]
pub struct SystemManager {
    pub setup_systems: Vec<(SystemPriority, SetupSystem)>,
    pub switch_systems: Vec<(SystemPriority, SwitchSystem)>,
    pub resize_systems: Vec<(SystemPriority, ResizeSystem)>,
    pub update_systems: Vec<(SystemPriority, (UpdateOperation, UpdateSystem))>,
    pub end_systems: Vec<(SystemPriority, EndSystem)>,
    pub render_systems: Vec<(SystemPriority, RenderSystem)>,
}

impl SystemManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self) {
        self.setup_systems.sort_by_key(|e| e.0);
        self.switch_systems.sort_by_key(|e| e.0);
        self.resize_systems.sort_by_key(|e| e.0);
        self.update_systems.sort_by_key(|e| e.0);
        self.end_systems.sort_by_key(|e| e.0);
        self.render_systems.sort_by_key(|e| e.0);
    }

    pub fn register_system(&mut self, system: System) {
        match system.system_type {
            SystemType::Update(update) => self
                .update_systems
                .push((system.priority, (UpdateOperation::EveryFrame, update))),
            SystemType::UpdateNFrame(frame, update) => self.update_systems.push((
                system.priority,
                (UpdateOperation::EveryNFrame(frame), update),
            )),
            SystemType::UpdateAfter(duration, update) => self.update_systems.push((
                system.priority,
                (
                    UpdateOperation::UpdaterAfter(Instant::now(), duration),
                    update,
                ),
            )),
            SystemType::Render(render) => self.render_systems.push((system.priority, render)),
            SystemType::End(end) => self.end_systems.push((system.priority, end)),
            SystemType::Resize(resize) => self.resize_systems.push((system.priority, resize)),
            SystemType::Setup(setup) => self.setup_systems.push((system.priority, setup)),
            SystemType::Switch(switch) => self.switch_systems.push((system.priority, switch)),
        }
    }
}
