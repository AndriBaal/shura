#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle};
use crate::{
    data::arena::Arena, Color, ComponentConfig, ComponentHandle, ComponentIdentifier,
    ComponentTypeId, Context, EndReason, Gpu, InstanceBuffer, InstancePosition,
    RenderTarget, World,
};
use downcast_rs::*;

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

#[allow(unused_variables)]
pub trait Position: Downcast {
    fn instance(&self, world: &World) -> InstancePosition;
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

    pub fn buffer(
        &mut self,
        each: impl Fn(&mut C) -> InstancePosition,
    ) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = each(component);
                self.buffer.write_offset(self.gpu, *offset, &[data]);
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .iter_mut()
                    .map(|component| each(component))
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }
}

#[cfg(feature = "rayon")]
impl<'a, C: Component + Send + Sync> BufferHelper<'a, C> {
    pub fn par_buffer(
        &mut self,
        each: impl Fn(&mut C) -> InstancePosition + Send + Sync,
    ) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = each(component);
                self.buffer.write_offset(self.gpu, *offset, &[data]);
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .items
                    .par_iter_mut()
                    .filter_map(|component| match component {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => Some(each(data)),
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(self.gpu, &instances);
            }
        };
    }
}
