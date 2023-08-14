#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle};
use crate::{
    data::arena::Arena, Color, ComponentConfig, ComponentHandle, ComponentIdentifier,
    ComponentRenderer, ComponentTypeId, Context, EndReason, Gpu, InstanceBuffer, InstancePosition,
    RenderTarget, World,
};
use downcast_rs::*;

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

/// Fields names of a struct used for deserialization and serialization
pub trait FieldNames {
    const FIELDS: &'static [&'static str];
}

#[allow(unused_variables)]
pub trait Position: Downcast {
    fn instance(&self, world: &World) -> InstancePosition;
    fn init(&mut self, handle: ComponentHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}
impl_downcast!(Position);

/// All components need to implement from this trait. This is not done manually, but with the derive macro [Component](crate::Component).
#[cfg(feature = "rayon")]
pub trait ComponentDerive {
    fn position(&self) -> &dyn Position;
    fn component_type_id(&self) -> ComponentTypeId;
    fn init(&mut self, handle: ComponentHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
#[cfg(not(feature = "rayon"))]
pub trait ComponentDerive {
    fn position(&self) -> &dyn Position;
    fn component_type_id(&self) -> ComponentTypeId;
    fn init(&mut self, handle: ComponentHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}

#[allow(unused_variables)]
/// A controller is used to define the behaviour of a component, by the given config and callbacks.
pub trait ComponentController: ComponentDerive + ComponentIdentifier + ComponentBuffer
where
    Self: Sized,
{
    const CONFIG: ComponentConfig = ComponentConfig::DEFAULT;
    /// This component gets updated if the component's [group](crate::Group) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(ctx: &mut Context) {}

    #[cfg(feature = "physics")]
    /// Collision Event between 2 components. It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [Group](crate::Group) is inactive or disabled.
    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
    }

    fn render<'a>(ctx: &'a Context, renderer: &mut ComponentRenderer<'a>) {}

    /// Method called when the game is closed or the scene gets removed
    fn end(ctx: &mut Context, reason: EndReason) {}

    fn render_target<'a>(ctx: &'a Context) -> (Option<Color>, &'a RenderTarget) {
        return (None, &ctx.defaults.world_target);
    }
}

impl<C: ComponentDerive + ?Sized> ComponentDerive for Box<C> {
    fn position(&self) -> &dyn Position {
        (**self).position()
    }

    fn component_type_id(&self) -> ComponentTypeId {
        (**self).component_type_id()
    }

    fn init(&mut self, handle: ComponentHandle, world: &mut World) {
        (**self).init(handle, world)
    }

    fn finish(&mut self, world: &mut World) {
        (**self).finish(world)
    }
}

pub(crate) enum BufferHelperType<'a, C: ComponentDerive> {
    Single { offset: u64, component: &'a mut C },
    All { components: &'a mut Arena<C> },
}

pub struct BufferHelper<'a, C: ComponentDerive> {
    inner: BufferHelperType<'a, C>,
    pub world: &'a World,
    pub buffer: &'a mut InstanceBuffer,
}

impl<'a, C: ComponentDerive> BufferHelper<'a, C> {
    pub(crate) fn new(
        world: &'a World,
        buffer: &'a mut InstanceBuffer,
        inner: BufferHelperType<'a, C>,
    ) -> Self {
        Self {
            world,
            inner,
            buffer,
        }
    }

    pub fn buffer<B: bytemuck::Pod + bytemuck::Zeroable + Send>(
        &mut self,
        gpu: &Gpu,
        each: impl Fn(&mut C) -> B + Send + Sync,
    ) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = each(component);
                self.buffer
                    .write_offset(gpu, *offset, bytemuck::cast_slice(&[data]));
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .iter_mut()
                    .map(|component| each(component))
                    .collect::<Vec<B>>();
                self.buffer.write(gpu, bytemuck::cast_slice(&instances));
            }
        };
    }

    pub fn buffer_uncasted(&mut self, gpu: &Gpu) {
        match &self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = component.position().instance(self.world);
                self.buffer
                    .write_offset(gpu, *offset, bytemuck::cast_slice(&[data]));
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .iter()
                    .map(|component| component.position().instance(self.world))
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(gpu, bytemuck::cast_slice(&instances));
            }
        };
    }
}

#[cfg(feature = "rayon")]
impl<'a, C: ComponentDerive + Send + Sync> BufferHelper<'a, C> {
    pub fn par_buffer<B: bytemuck::Pod + bytemuck::Zeroable + Send>(
        &mut self,
        gpu: &Gpu,
        each: impl Fn(&mut C) -> B + Send + Sync,
    ) {
        match &mut self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = each(component);
                self.buffer
                    .write_offset(gpu, *offset, bytemuck::cast_slice(&[data]));
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .items
                    .par_iter_mut()
                    .filter_map(|component| match component {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => Some(each(data)),
                    })
                    .collect::<Vec<B>>();
                self.buffer.write(gpu, bytemuck::cast_slice(&instances));
            }
        };
    }

    pub fn par_buffer_uncasted(&mut self, gpu: &Gpu) {
        match &self.inner {
            BufferHelperType::Single { offset, component } => {
                let data = component.position().instance(self.world);
                self.buffer
                    .write_offset(gpu, *offset, bytemuck::cast_slice(&[data]));
            }
            BufferHelperType::All { components } => {
                let instances = components
                    .items
                    .par_iter()
                    .filter_map(|component| match component {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => {
                            Some(data.position().instance(self.world))
                        }
                    })
                    .collect::<Vec<InstancePosition>>();
                self.buffer.write(gpu, bytemuck::cast_slice(&instances));
            }
        };
    }
}

pub trait ComponentBuffer: Sized + ComponentDerive {
    const INSTANCE_SIZE: u64 = InstancePosition::SIZE;
    fn buffer_with(
        gpu: &Gpu,
        mut helper: BufferHelper<Self>,
        each: impl Fn(&mut Self) + Send + Sync,
    ) {
        helper.buffer(gpu, |c: &mut Self| {
            each(c);
            c.position().instance(helper.world)
        })
    }
    fn buffer(gpu: &Gpu, mut helper: BufferHelper<Self>) {
        helper.buffer_uncasted(gpu)
    }
}
