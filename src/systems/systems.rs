use crate::ComponentResources;

use crate::{Context, Duration, EndReason, Instant, RenderEncoder};

type SetupSystem = fn(ctx: &mut Context);
type ResizeSystem = fn(ctx: &mut Context);
type UpdateSystem = fn(ctx: &mut Context);
type RenderSystem = fn(ctx: &ComponentResources, encoder: &mut RenderEncoder);
type EndSystem = fn(&mut Context, reason: EndReason);

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
    pub resize_systems: Vec<ResizeSystem>,
    pub update_systems: Vec<(UpdateOperation, UpdateSystem)>,
    pub end_systems: Vec<EndSystem>,
    pub render_systems: Vec<RenderSystem>,
}

impl SystemManager {
    pub fn empty() -> Self {
        return Self {
            resize_systems: Default::default(),
            update_systems: Default::default(),
            end_systems: Default::default(),
            render_systems: Default::default(),
        };
    }

    pub fn new(systems: &[System]) -> Self {
        let mut system_manager = Self::empty();
        system_manager.init(systems);
        return system_manager;
    }

    pub fn init(&mut self, systems: &[System]) {
        for system in systems {
            match *system {
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
                System::Setup(_) => {}
            }
        }
    }
}
