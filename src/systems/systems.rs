use crate::Component;
use rustc_hash::FxHashSet;
use std::cell::RefCell;

#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle, FxHashMap,
};
use crate::{
    BufferOperation, Color, ComponentConfig, ComponentTypeId, Context,
    EndReason, Instant, Duration, RenderTarget, RenderEncoder
};

type SetupSystem = fn(ctx: &mut Context);
type UpdateSystem = fn(ctx: &mut Context);
type RenderSystem = fn(ctx: &Context, components: &mut RenderEncoder);
type EndSystem = fn(&mut Context, reason: EndReason);

pub enum System {
    Setup(SetupSystem),
    Update(UpdateSystem),
    UpdateNFrame(u64, UpdateSystem),
    UpdateAfter(Duration, UpdateSystem),
    Render(RenderSystem),
    End(EndSystem)
}

pub(crate) enum UpdateOperation {
    EveryFrame,
    EveryNFrame(u64),
    UpdaterAfter(Instant, Duration)
}

pub(crate) struct SystemManager {
    pub update_systems: Vec<(
        UpdateOperation,
        UpdateSystem,
    )>,
    pub end_systems: Vec<EndSystem>,
    pub render_systems: Vec<RenderSystem>,
    // #[cfg(feature = "physics")]
    // collision_systems: FxHashMap<(ComponentTypeId, ComponentTypeId), CollisionSystem>
}

impl SystemManager {
    pub fn new(systems: &Vec<System>) -> Self {
        let mut update_systems = Vec::new();
        let mut render_systems = Vec::new();
        let mut end_systems = Vec::new();

        for system in systems {
            match *system {
                System::Update(update) => update_systems.push((UpdateOperation::EveryFrame, update)),
                System::UpdateNFrame(frame, update) => update_systems.push((UpdateOperation::EveryNFrame(frame), update)),
                System::UpdateAfter(duration, update) => update_systems.push((UpdateOperation::UpdaterAfter(Instant::now(), duration), update)),
                System::Render(render) => render_systems.push(render),
                System::End(end) => end_systems.push(end),
                _ => ()
            }
        }

        return Self {
            update_systems,
            render_systems,
            end_systems
        }
    }
}
