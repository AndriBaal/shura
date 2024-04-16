use std::collections::hash_map::{Iter, IterMut};

use crate::{
    entity::EntityGroupManager,
    graphics::{Gpu, Instance, InstanceBuffer},
};
use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderGroupUpdate {
    pub call: BufferCall,
    pub buffer_on_group_change: bool,
}

impl RenderGroupUpdate {
    pub const MANUAL: RenderGroupUpdate = RenderGroupUpdate {
        call: BufferCall::Manual,
        buffer_on_group_change: false,
    };

    pub const GROUP_CHANGED: RenderGroupUpdate = RenderGroupUpdate {
        call: BufferCall::Manual,
        buffer_on_group_change: true,
    };

    pub const EVERY_FRAME: RenderGroupUpdate = RenderGroupUpdate {
        call: BufferCall::EveryFrame,
        buffer_on_group_change: true,
    };
}

impl Default for RenderGroupUpdate {
    fn default() -> Self {
        Self::EVERY_FRAME
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferCall {
    Manual,
    EveryFrame,
}

pub trait RenderGroupCommon: Downcast {
    fn prepare(&mut self, groups: &EntityGroupManager);
    fn apply(&mut self, gpu: &Gpu);
    fn deinit(&mut self);
    fn needs_update(&self) -> bool;
    fn manual_buffer(&self) -> bool;
    fn set_manual_buffer(&mut self, manual_buffer: bool);
}
impl_downcast!(RenderGroupCommon);

pub struct RenderGroup<I: Instance> {
    buffer: Option<InstanceBuffer<I>>,
    config: RenderGroupUpdate,
    data: Vec<I>,
    manual_buffer: bool,
    needs_update: bool,
}

impl<I: Instance> RenderGroup<I> {
    const ALLOC: u64 = 16;
    pub(crate) fn new(config: RenderGroupUpdate) -> Self {
        Self {
            buffer: None,
            manual_buffer: true,
            data: Vec::with_capacity(Self::ALLOC as usize),
            config,
            needs_update: false,
        }
    }

    pub fn push(&mut self, instance: I) {
        self.data.push(instance);
    }

    pub fn extend(&mut self, instances: impl Iterator<Item = I>) {
        self.data.extend(instances);
    }

    pub fn buffer(&self) -> &InstanceBuffer<I> {
        self.buffer.as_ref().expect("Buffer uninitialized!")
    }
}

#[cfg(feature = "rayon")]
impl<I: Instance + Send + Sync> RenderGroup<I> {
    pub fn par_extend(&mut self, instances: impl ParallelIterator<Item = I>) {
        self.data.par_extend(instances);
    }
}

impl<I: Instance> RenderGroupCommon for RenderGroup<I> {
    fn apply(&mut self, gpu: &Gpu) {
        if self.needs_update {
            self.buffer
                .get_or_insert_with(|| InstanceBuffer::<I>::empty(gpu, self.data.len() as u64))
                .write(gpu, &self.data);
            self.data.clear();
            self.needs_update = false;
            self.manual_buffer = match self.config.call {
                BufferCall::Manual => false,
                BufferCall::EveryFrame => true,
            }
        }
    }

    fn manual_buffer(&self) -> bool {
        self.manual_buffer
    }

    fn set_manual_buffer(&mut self, manual_buffer: bool) {
        self.manual_buffer = manual_buffer;
    }

    fn deinit(&mut self) {
        self.buffer = None;
    }

    fn prepare(&mut self, groups: &EntityGroupManager) {
        self.needs_update = self.config.call == BufferCall::EveryFrame
            || self.manual_buffer
            || (groups.render_groups_changed() && self.config.buffer_on_group_change)
    }

    fn needs_update(&self) -> bool {
        self.needs_update
    }
}

pub struct RenderGroupManager {
    buffers: FxHashMap<&'static str, Box<dyn RenderGroupCommon>>,
}

impl RenderGroupManager {
    pub(crate) fn new() -> Self {
        Self {
            buffers: Default::default(),
        }
    }

    pub(crate) fn register<I: Instance>(&mut self, name: &'static str, config: RenderGroupUpdate) {
        if self.buffers.contains_key(name) {
            panic!("Group \"{}\" already defined!", name);
        }

        self.buffers
            .insert(name, Box::new(RenderGroup::<I>::new(config)));
    }

    pub(crate) fn apply_buffers(&mut self, gpu: &Gpu) {
        for buffer in self.buffers.values_mut() {
            buffer.apply(gpu);
        }
    }

    pub(crate) fn prepare_buffers(&mut self, groups: &EntityGroupManager) {
        for buffer in self.buffers.values_mut() {
            buffer.prepare(groups)
        }
    }

    pub fn get<I: Instance>(&self, name: &'static str) -> Option<&RenderGroup<I>> {
        self.buffers
            .get(name)
            .and_then(|b| b.downcast_ref::<RenderGroup<I>>())
    }

    pub fn get_mut<I: Instance>(&mut self, name: &'static str) -> Option<&mut RenderGroup<I>> {
        self.buffers
            .get_mut(name)
            .and_then(|b| b.downcast_mut::<RenderGroup<I>>())
    }

    pub fn iter(&self) -> Iter<'_, &str, Box<dyn RenderGroupCommon>> {
        return self.buffers.iter();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, &str, Box<dyn RenderGroupCommon>> {
        return self.buffers.iter_mut();
    }
}
