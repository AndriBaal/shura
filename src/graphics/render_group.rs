use std::collections::hash_map::{Iter, IterMut};

use crate::{
    entity::EntityGroupManager,
    graphics::{Gpu, Instance, InstanceBuffer, GLOBAL_GPU},
};
use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderGroupConfig {
    pub call: BufferCall,
    pub init: bool,
    // pub mapped: bool,
    pub buffer_on_group_change: bool,
}

impl RenderGroupConfig {
    pub const MANUAL: RenderGroupConfig = RenderGroupConfig {
        call: BufferCall::Manual,
        init: true,
        buffer_on_group_change: false,
    };

    pub const EVERY_FRAME: RenderGroupConfig = RenderGroupConfig {
        call: BufferCall::EveryFrame,
        init: true,
        buffer_on_group_change: true,
    };
}

impl Default for RenderGroupConfig {
    fn default() -> Self {
        Self {
            call: BufferCall::EveryFrame,
            // mapped: false,
            buffer_on_group_change: false,
            init: true,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferCall {
    Manual,
    EveryFrame,
}

pub trait RenderGroupImpl: Downcast {
    fn apply(&mut self, groups: &EntityGroupManager, gpu: &Gpu);
    fn deinit(&mut self);
    fn update_buffer(&self) -> bool;
    fn set_update_buffer(&mut self, update_buffer: bool);
}
impl_downcast!(RenderGroupImpl);

pub struct RenderGroup<I: Instance> {
    buffer: Option<InstanceBuffer<I>>,
    config: RenderGroupConfig,
    data: Vec<I>,
    update_buffer: bool,
}

impl<I: Instance> RenderGroup<I> {
    const ALLOC: u64 = 16;
    pub(crate) fn new(gpu: &Gpu, config: RenderGroupConfig) -> Self {
        Self {
            buffer: if config.init {
                Some(InstanceBuffer::<I>::empty(gpu, Self::ALLOC))
            } else {
                None
            },
            update_buffer: true,
            data: Vec::with_capacity(Self::ALLOC as usize),
            config,
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

impl<I: Instance> RenderGroupImpl for RenderGroup<I> {
    fn apply(&mut self, groups: &EntityGroupManager, gpu: &Gpu) {
        if self.update_buffer
            || (groups.render_groups_changed() && self.config.buffer_on_group_change)
        {
            self.buffer
                .get_or_insert_with(|| InstanceBuffer::<I>::empty(gpu, Self::ALLOC))
                .write(gpu, &self.data);
            self.data.clear();
            self.update_buffer = match self.config.call {
                BufferCall::Manual => false,
                BufferCall::EveryFrame => true,
            }
        }
    }

    fn update_buffer(&self) -> bool {
        self.update_buffer
    }

    fn set_update_buffer(&mut self, update_buffer: bool) {
        self.update_buffer = update_buffer;
    }

    fn deinit(&mut self) {
        self.buffer = None;
    }
}

pub struct RenderGroupManager {
    buffers: FxHashMap<&'static str, Box<dyn RenderGroupImpl>>,
}

impl RenderGroupManager {
    pub(crate) fn new() -> Self {
        Self {
            buffers: Default::default(),
        }
    }

    pub(crate) fn register_component<I: Instance>(
        &mut self,
        name: &'static str,
        config: RenderGroupConfig,
    ) {
        if self.buffers.contains_key(name) {
            panic!("Component {} already defined!", name);
        }

        self.buffers.insert(
            name,
            Box::new(RenderGroup::<I>::new(GLOBAL_GPU.get().unwrap(), config)),
        );
    }

    pub(crate) fn apply_buffers(&mut self, groups: &EntityGroupManager, gpu: &Gpu) {
        for buffer in self.buffers.values_mut() {
            buffer.apply(groups, gpu);
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

    pub fn iter(&self) -> Iter<'_, &str, Box<dyn RenderGroupImpl>> {
        return self.buffers.iter();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, &str, Box<dyn RenderGroupImpl>> {
        return self.buffers.iter_mut();
    }
}
