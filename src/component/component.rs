#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle};
use crate::{
    data::arena::Arena, Color, ComponentConfig, ComponentHandle, ComponentIdentifier,
    ComponentTypeId, Context, EndReason, Gpu, InstanceBuffer, InstancePosition, RenderTarget,
    World,
};
use downcast_rs::*;

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

#[allow(unused_variables)]
pub trait Position: Downcast {
    fn instance(&self, world: &World) -> InstancePosition;
    fn active(&self) -> bool;
    fn init(&mut self, handle: ComponentHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}
impl_downcast!(Position);

/// All components need to implement from this trait. This is not done manually, but with the derive macro [Component](crate::Component).
pub trait Component: ComponentIdentifier + Sized + 'static {
    fn position(&self) -> &dyn Position;
    fn component_type_id(&self) -> ComponentTypeId;
    fn init(&mut self, handle: ComponentHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
    fn buffer_with(helper: BufferHelper<Self>, each: impl Fn(&mut Self) + Send + Sync);
    fn buffer(helper: BufferHelper<Self>);
}

pub(crate) enum BufferHelperType<'a, C: Component> {
    Single { offset: u64, component: &'a mut C },
    All { components: &'a mut Arena<C> },
}

pub struct BufferHelper<'a, C: Component> {
    inner: BufferHelperType<'a, C>,
    pub gpu: &'a Gpu,
    pub world: &'a World,
    pub buffer: &'a mut InstanceBuffer<InstancePosition>,
}

impl<'a, C: Component> BufferHelper<'a, C> {
    pub(crate) fn new(
        world: &'a World,
        gpu: &'a Gpu,
        buffer: &'a mut InstanceBuffer<InstancePosition>,
        inner: BufferHelperType<'a, C>,
    ) -> Self {
        Self {
            world,
            inner,
            buffer,
            gpu,
        }
    }

    pub fn buffer(&mut self) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                if component.position().active() {
                    self.buffer.write_offset(
                        self.gpu,
                        *offset,
                        &[component.position().instance(self.world)],
                    );
                } else {
                    self.buffer.write_offset(self.gpu, *offset, &[]);
                }
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .iter_mut()
                    .filter_map(|component| {
                        if component.position().active() {
                            Some(component.position().instance(self.world))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }

    pub fn buffer_with(&mut self, each: impl Fn(&mut C)) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                if component.position().active() {
                    self.buffer.write_offset(
                        self.gpu,
                        *offset,
                        &[component.position().instance(self.world)],
                    );
                } else {
                    self.buffer.write_offset(self.gpu, *offset, &[]);
                }
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .iter_mut()
                    .filter_map(|component| {
                        each(component);
                        if component.position().active() {
                            Some(component.position().instance(self.world))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }
}

#[cfg(feature = "rayon")]
impl<'a, C: Component + Send + Sync> BufferHelper<'a, C> {
    pub fn par_buffer_with(&mut self, each: impl Fn(&mut C) + Send + Sync) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                if component.position().active() {
                    self.buffer.write_offset(
                        self.gpu,
                        *offset,
                        &[component.position().instance(self.world)],
                    );
                } else {
                    self.buffer.write_offset(self.gpu, *offset, &[]);
                }
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .items
                    .par_iter_mut()
                    .filter_map(|component| match component {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => {
                            each(data);
                            if data.position().active() {
                                Some(data.position().instance(self.world))
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }

    pub fn par_buffer(&mut self) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                if component.position().active() {
                    self.buffer.write_offset(
                        self.gpu,
                        *offset,
                        &[component.position().instance(self.world)],
                    );
                } else {
                    self.buffer.write_offset(self.gpu, *offset, &[]);
                }
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .items
                    .par_iter_mut()
                    .filter_map(|component| match component {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => {
                            if data.position().active() {
                                Some(data.position().instance(self.world))
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }
}
