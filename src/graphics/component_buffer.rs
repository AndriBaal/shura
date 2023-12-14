use std::collections::hash_map::{Iter, IterMut};

use crate::{
    entity::GroupManager,
    graphics::{Gpu, Instance, InstanceBuffer, GLOBAL_GPU},
};
use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BufferConfig {
    pub call: BufferCall,
    pub init: bool,
    pub mapped: bool,
    pub buffer_on_group_change: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            call: BufferCall::EveryFrame,
            mapped: false,
            buffer_on_group_change: true,
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

pub trait ComponentBufferImpl: Downcast {
    fn apply(&mut self, groups: &GroupManager, gpu: &Gpu);
    fn deinit(&mut self);
    fn update_buffer(&self) -> bool;
    fn set_update_buffer(&mut self, update_buffer: bool);
}
impl_downcast!(ComponentBufferImpl);

pub struct ComponentBuffer<I: Instance> {
    buffer: Option<InstanceBuffer<I>>,
    config: BufferConfig,
    data: Vec<I>,
    update_buffer: bool,
}

impl<I: Instance> ComponentBuffer<I> {
    const ALLOC: u64 = 16;
    pub(crate) fn new(gpu: &Gpu, config: BufferConfig) -> Self {
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

    // pub fn push_from_entities<E: Entity, ET: EntityType<Entity = E>, C: Component<Instance = I>>(
    //     &mut self,
    //     world: &World,
    //     entites: &ET,
    //     each: impl Fn(&E) -> &C,
    // ) {
    //     // entites.for_each(|entity| {
    //     //     let component = (each)(entity);
    //     //     if component.active() {
    //     //         self.data.push(component.instance(world));
    //     //     }
    //     // });
    // }

    // pub fn push_from_entities_mut<E: Entity, C: Component<Instance = I>>(
    //     &mut self,
    //     world: &World,
    //     entites: &mut EntitySetMut<'_, E>,
    //     each: impl Fn(&E) -> &C,
    // ) {
    //     entites.for_each_mut(|entity| {
    //         let component = (each)(entity);
    //         if component.active() {
    //             self.data.push(component.instance(world));
    //         }
    //     });
    // }

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
impl<I: Instance + Send + Sync> ComponentBuffer<I> {
    pub fn par_extend(&mut self, instances: impl ParallelIterator<Item = I>) {
        self.data.par_extend(instances);
    }
}

impl<I: Instance> ComponentBufferImpl for ComponentBuffer<I> {
    fn apply(&mut self, groups: &GroupManager, gpu: &Gpu) {
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

pub struct ComponentBufferManager {
    buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl ComponentBufferManager {
    pub(crate) fn new() -> Self {
        Self {
            buffers: Default::default(),
        }
    }

    pub(crate) fn register_component<I: Instance>(
        &mut self,
        name: &'static str,
        config: BufferConfig,
    ) {
        if self.buffers.contains_key(name) {
            panic!("Component {} already defined!", name);
        }

        self.buffers.insert(
            name,
            Box::new(ComponentBuffer::<I>::new(GLOBAL_GPU.get().unwrap(), config)),
        );
    }

    pub(crate) fn apply_buffers(&mut self, groups: &GroupManager, gpu: &Gpu) {
        for buffer in self.buffers.values_mut() {
            buffer.apply(groups, gpu);
        }
    }

    pub fn get<I: Instance>(&self, name: &'static str) -> Option<&ComponentBuffer<I>> {
        self.buffers
            .get(name)
            .and_then(|b| b.downcast_ref::<ComponentBuffer<I>>())
    }

    pub fn get_mut<I: Instance>(&mut self, name: &'static str) -> Option<&mut ComponentBuffer<I>> {
        self.buffers
            .get_mut(name)
            .and_then(|b| b.downcast_mut::<ComponentBuffer<I>>())
    }

    pub fn iter(&self) -> Iter<'_, &str, Box<dyn ComponentBufferImpl>> {
        return self.buffers.iter();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, &str, Box<dyn ComponentBufferImpl>> {
        return self.buffers.iter_mut();
    }
}
