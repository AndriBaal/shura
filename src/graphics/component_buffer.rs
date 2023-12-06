use crate::{Gpu, GroupManager, Instance, InstanceBuffer};
use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BufferConfig {
    call: BufferCall,
    mapped: bool,
    buffer_on_group_change: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            call: BufferCall::EveryFrame,
            mapped: false,
            buffer_on_group_change: true,
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
}
impl_downcast!(ComponentBufferImpl);

pub struct ComponentBuffer<I: Instance> {
    buffer: InstanceBuffer<I>,
    config: BufferConfig,
    data: Vec<I>,
    update_buffer: bool,
}

impl<I: Instance> ComponentBuffer<I> {
    pub(crate) fn new(gpu: &Gpu, config: BufferConfig) -> Self {
        const ALLOC: u64 = 16;
        Self {
            buffer: InstanceBuffer::<I>::empty(gpu, ALLOC),
            update_buffer: true,
            data: Vec::with_capacity(ALLOC as usize),
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

    pub fn update_buffer(&self) -> bool {
        self.update_buffer
    }

    pub fn buffer(&self) -> &InstanceBuffer<I> {
        &self.buffer
    }

    pub fn set_update_buffer(&mut self, update_buffer: bool) {
        self.update_buffer = update_buffer;
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
            self.buffer.write(gpu, &self.data);
            self.data.clear();
            self.update_buffer = match self.config.call {
                BufferCall::Manual => false,
                BufferCall::EveryFrame => true,
            }
        }
    }
}

pub struct ComponentBufferManager {
    buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl ComponentBufferManager {
    pub(crate) fn empty() -> Self {
        Self {
            buffers: Default::default(),
        }
    }

    pub(crate) fn new(buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>) -> Self {
        let mut component_buffer_manager = Self::empty();
        component_buffer_manager.init(buffers);
        component_buffer_manager
    }

    pub(crate) fn init(&mut self, buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>) {
        self.buffers = buffers;
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
}
