use crate::{
    entity::EntityHandle,
    graphics::{Instance, RenderGroup},
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait Component: Downcast {
    type ComponentInstance: ComponentInstance
    where
        Self: Sized;

    fn buffer_all(
        &self,
        world: &World,
        render_groups: &mut RenderGroup<<Self::ComponentInstance as ComponentInstance>::Instance>,
    ) where
        Self: Sized;
    fn init_all(&mut self, handle: EntityHandle, world: &mut World);
    fn finish_all(&mut self, world: &mut World);
    fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn ComponentInstance> + 'a>;
    fn instances_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn ComponentInstance> + 'a>;
}
impl_downcast!(Component);

#[allow(unused_variables)]
pub trait ComponentInstance: Downcast {
    type Instance: Instance
    where
        Self: Sized;
    fn instance(&self, world: &World) -> Self::Instance
    where
        Self: Sized;
    fn active(&self) -> bool;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}
impl_downcast!(ComponentInstance);

impl<C: ComponentInstance> Component for C {
    type ComponentInstance = C;

    fn buffer_all(
        &self,
        world: &World,
        render_group: &mut RenderGroup<<Self::ComponentInstance as ComponentInstance>::Instance>,
    ) {
        if self.active() {
            render_group.push(self.instance(world))
        }
    }

    fn init_all(&mut self, handle: EntityHandle, world: &mut World) {
        self.init(handle, world)
    }

    fn finish_all(&mut self, world: &mut World) {
        self.finish(world)
    }

    fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn ComponentInstance> + 'a> {
        Box::new(std::iter::once(self as _))
    }

    fn instances_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn ComponentInstance> + 'a> {
        Box::new(std::iter::once(self as _))
    }
}

macro_rules! impl_collection {
    ($collection: ty) => {
        impl<C: ComponentInstance> Component for $collection {
            type ComponentInstance = C;

            fn buffer_all(
                &self,
                world: &World,
                render_group: &mut RenderGroup<<Self::ComponentInstance as ComponentInstance>::Instance>,
            ) {
                for component in self.iter() {
                    component.buffer_all(world, render_group);
                }
            }

            fn init_all(&mut self, handle: EntityHandle, world: &mut World) {
                for component in self.iter_mut() {
                    component.init_all(handle, world);
                }
            }

            fn finish_all(&mut self, world: &mut World) {
                for component in self.iter_mut() {
                    component.finish_all(world);
                }
            }

            fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn ComponentInstance> + 'a> {
                Box::new(self.iter().map(|c| c as _))
            }

            fn instances_mut<'a>(
                &'a mut self,
            ) -> Box<dyn Iterator<Item = &mut dyn ComponentInstance> + 'a> {
                Box::new(self.iter_mut().map(|c| c as _))
            }
        }
    };
}

macro_rules! impl_collection_map {
    ($collection: ty) => {
        impl<K: 'static, C: ComponentInstance> Component for $collection {
            type ComponentInstance = C;

            fn buffer_all(
                &self,
                world: &World,
                render_group: &mut RenderGroup<<Self::ComponentInstance as ComponentInstance>::Instance>,
            ) {
                for component in self.values() {
                    component.buffer_all(world, render_group);
                }
            }

            fn init_all(&mut self, handle: EntityHandle, world: &mut World) {
                for component in self.values_mut() {
                    component.init_all(handle, world);
                }
            }

            fn finish_all(&mut self, world: &mut World) {
                for component in self.values_mut() {
                    component.finish_all(world);
                }
            }

            fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn ComponentInstance> + 'a> {
                Box::new(self.values().map(|c| c as _))
            }

            fn instances_mut<'a>(
                &'a mut self,
            ) -> Box<dyn Iterator<Item = &mut dyn ComponentInstance> + 'a> {
                Box::new(self.values_mut().map(|c| c as _))
            }
        }
    };
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
