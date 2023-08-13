use std::cell::RefCell;

use rustc_hash::FxHashSet;

#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle, FxHashMap,
};
use crate::{
    BufferHelper, BufferOperation, Color, ComponentBuffer, ComponentConfig, ComponentController,
    ComponentRenderer, ComponentTypeId, Context, EndOperation, EndReason, Gpu, Instant,
    RenderOperation, RenderTarget, UpdateOperation,
};

pub(crate) type BufferCallback = fn(gpu: &Gpu, helper: BufferHelper);
type UpdateCallback = fn(ctx: &mut Context);
type RenderCallback = for<'a> fn(ctx: &'a Context, renderer: &mut ComponentRenderer<'a>);
type TargetCallback = for<'a> fn(ctx: &'a Context) -> (Option<Color>, &'a RenderTarget);
#[cfg(feature = "physics")]
type CollisionCallback = fn(
    ctx: &mut Context,
    self_handle: ComponentHandle,
    other_handle: ComponentHandle,
    self_collider: ColliderHandle,
    other_collider: ColliderHandle,
    collision_type: CollideType,
);
type EndCallback = fn(&mut Context, reason: EndReason);

struct NewEntry {
    config: ComponentConfig,
    type_id: ComponentTypeId,
    update: UpdateCallback,
    render: RenderCallback,
    #[cfg(feature = "physics")]
    collision: CollisionCallback,
    end: EndCallback,
    buffer: BufferCallback,
    render_target: TargetCallback,
}

#[derive(Default)]
pub(crate) struct ControllerManager {
    update_callbacks: Vec<(
        i16,
        i16,
        UpdateCallback,
        Option<RefCell<Instant>>,
        UpdateOperation,
    )>,
    end_callbacks: Vec<(i16, EndCallback)>,
    render_callbacks: Vec<(i16, RenderCallback, TargetCallback)>,
    buffer_callbacks: Vec<(BufferCallback, ComponentTypeId)>,
    #[cfg(feature = "physics")]
    collision_callbacks: FxHashMap<ComponentTypeId, CollisionCallback>,
    new_entries: RefCell<(FxHashSet<ComponentTypeId>, Vec<NewEntry>)>,
}

impl ControllerManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn updates(
        &self,
    ) -> &Vec<(
        i16,
        i16,
        UpdateCallback,
        Option<RefCell<Instant>>,
        UpdateOperation,
    )> {
        &self.update_callbacks
    }

    pub fn buffers(&self) -> &Vec<(BufferCallback, ComponentTypeId)> {
        &self.buffer_callbacks
    }

    pub fn ends(&self) -> &Vec<(i16, EndCallback)> {
        &self.end_callbacks
    }

    pub fn renders(&self) -> &Vec<(i16, RenderCallback, TargetCallback)> {
        &self.render_callbacks
    }

    #[cfg(feature = "physics")]
    pub fn collisions(&self) -> &FxHashMap<ComponentTypeId, CollisionCallback> {
        &self.collision_callbacks
    }

    pub fn register<C: ComponentController + ComponentBuffer>(&self, config: ComponentConfig) {
        let mut new = self.new_entries.borrow_mut();
        if !new.0.contains(&C::IDENTIFIER) {
            new.0.insert(C::IDENTIFIER);
            new.1.push(NewEntry {
                type_id: C::IDENTIFIER,
                config,
                update: C::update,
                #[cfg(feature = "physics")]
                collision: C::collision,
                render: C::render,
                end: C::end,
                buffer: C::buffer,
                render_target: C::render_target,
            });
            #[cfg(feature = "log")]
            log::info!(
                "Register component '{}' with ID '{}'",
                C::TYPE_NAME,
                C::IDENTIFIER
            );
        }
    }

    pub fn apply(&mut self) {
        let mut new = self.new_entries.borrow_mut();
        if !new.1.is_empty() {
            for entry in new.1.drain(..) {
                let config = entry.config;
                #[cfg(feature = "physics")]
                self.collision_callbacks
                    .insert(entry.type_id, entry.collision);
                match config.update {
                    UpdateOperation::EveryFrame => self.update_callbacks.push((
                        config.update_priority,
                        config.force_update_level,
                        entry.update,
                        None,
                        config.update,
                    )),
                    UpdateOperation::EveryNFrame(_) => self.update_callbacks.push((
                        config.update_priority,
                        config.force_update_level,
                        entry.update,
                        None,
                        config.update,
                    )),
                    UpdateOperation::AfterDuration(d) => self.update_callbacks.push((
                        config.update_priority,
                        config.force_update_level,
                        entry.update,
                        Some(RefCell::new(Instant::now() - d)),
                        config.update,
                    )),
                    UpdateOperation::Never => (),
                }

                match config.buffer {
                    BufferOperation::Manual => {
                        self.buffer_callbacks.push((entry.buffer, entry.type_id))
                    }
                    BufferOperation::EveryFrame => {
                        self.buffer_callbacks.push((entry.buffer, entry.type_id))
                    }
                    BufferOperation::Never => (),
                }

                match config.render {
                    RenderOperation::EveryFrame => self.render_callbacks.push((
                        config.render_priority,
                        entry.render,
                        entry.render_target,
                    )),
                    RenderOperation::Never => (),
                }

                match config.end {
                    EndOperation::Always => {
                        self.end_callbacks.push((config.end_priority, entry.end))
                    }
                    EndOperation::Never => (),
                }
            }
            self.update_callbacks.sort_by_key(|e| e.0);
            self.render_callbacks.sort_by_key(|e| e.0);
            self.end_callbacks.sort_by_key(|e| e.0);
        }
    }
}
