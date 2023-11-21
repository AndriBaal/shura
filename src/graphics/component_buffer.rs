use crate::{Entity, Gpu, Instance, InstanceBuffer};
use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferConfig {
    Manual,
    EveryFrame,
}

pub(crate) trait ComponentBufferImpl: Downcast {
    fn apply(&mut self, gpu: &Gpu);
}
impl_downcast!(ComponentBufferImpl);

pub struct ComponentBuffer<I: Instance> {
    buffer: InstanceBuffer<I>,
    operation: BufferConfig,
    data: Vec<I>,
    update_buffer: bool,
}

impl<I: Instance> ComponentBuffer<I> {
    pub(crate) fn new(gpu: &Gpu, operation: BufferConfig) -> Self {
        const ALLOC: u64 = 16;
        Self {
            buffer: InstanceBuffer::<I>::empty(gpu, ALLOC),
            update_buffer: true,
            data: Vec::with_capacity(ALLOC as usize),
            operation,
        }
    }

    pub fn push_components_from_entities<'a, E: Entity>(
        &mut self,
        entites: impl Iterator<Item = &'a E>,
        each: impl Fn(&E) -> I,
    ) {
        for entity in entites {
            self.data.push((each)(entity));
        }
    }

    pub fn push(&mut self, instance: I) {
        self.data.push(instance);
    }

    pub fn update_buffer(&self) -> bool {
        return self.update_buffer;
    }

    pub fn buffer(&self) -> &InstanceBuffer<I> {
        return &self.buffer;
    }

    pub fn set_update_buffer(&mut self, update_buffer: bool) {
        self.update_buffer = update_buffer;
    }
}

impl<I: Instance> ComponentBufferImpl for ComponentBuffer<I> {
    fn apply(&mut self, gpu: &Gpu) {
        if self.update_buffer {
            self.buffer.write(gpu, &self.data);
            self.data.clear();
            self.update_buffer = match self.operation {
                BufferConfig::Manual => false,
                BufferConfig::EveryFrame => true,
            }
        }
    }
}

pub struct ComponentBufferManager {
    buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl ComponentBufferManager {
    pub(crate) fn empty() -> Self {
        return Self {
            buffers: Default::default(),
        };
    }

    pub(crate) fn new(buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>) -> Self {
        let mut component_buffer_manager = Self::empty();
        component_buffer_manager.init(buffers);
        return component_buffer_manager;
    }

    pub(crate) fn init(&mut self, buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>) {
        self.buffers = buffers;
    }

    pub(crate) fn apply_buffers(&mut self, gpu: &Gpu) {
        for buffer in self.buffers.values_mut() {
            buffer.apply(gpu);
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
