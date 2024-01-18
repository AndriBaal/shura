use crate::graphics::Instance;
use crate::physics::World;
use crate::{entity::EntityHandle, graphics::RenderGroup};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait ComponentCollection: Downcast {
    type Component: Component
    where
        Self: Sized;

    fn buffer_all(
        &self,
        world: &World,
        buffer: &mut RenderGroup<<Self::Component as Component>::Instance>,
    ) where
        Self: Sized;
    fn init_all(&mut self, handle: EntityHandle, world: &mut World);
    fn finish_all(&mut self, world: &mut World);
    fn components<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a>;
    fn components_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a>;
    // fn iter<'a>(&'a self) -> impl Iterator<Item = &Self::Component> + 'a where Self: Sized;
}
impl_downcast!(ComponentCollection);

#[allow(unused_variables)]
pub trait Component: Downcast {
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
impl_downcast!(Component);

impl<C: Component> ComponentCollection for C {
    type Component = C;

    fn buffer_all(
        &self,
        world: &World,
        buffer: &mut RenderGroup<<Self::Component as Component>::Instance>,
    ) {
        if self.active() {
            buffer.push(self.instance(world))
        }
    }

    fn init_all(&mut self, handle: EntityHandle, world: &mut World) {
        self.init(handle, world)
    }

    fn finish_all(&mut self, world: &mut World) {
        self.finish(world)
    }

    fn components<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a> {
        Box::new(std::iter::once(self as _))
    }

    fn components_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a> {
        Box::new(std::iter::once(self as _))
    }

    // fn iter<'a>(&'a self) -> impl Iterator<Item = &Self::Component> + 'a where Self: Sized {
    //     std::iter::once(self)
    // }
}

macro_rules! impl_collection {
    ($collection: ty) => {
        impl<C: Component> ComponentCollection for $collection {
            type Component = C;

            fn buffer_all(
                &self,
                world: &World,
                buffer: &mut RenderGroup<<Self::Component as Component>::Instance>,
            ) {
                for component in self.iter() {
                    component.buffer_all(world, buffer);
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

            fn components<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a> {
                Box::new(self.iter().map(|c| c as _))
            }

            fn components_mut<'a>(
                &'a mut self,
            ) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a> {
                Box::new(self.iter_mut().map(|c| c as _))
            }
        }
    };
}

macro_rules! impl_collection_map {
    ($collection: ty) => {
        impl<K: 'static, C: Component> ComponentCollection for $collection {
            type Component = C;

            fn buffer_all(
                &self,
                world: &World,
                buffer: &mut RenderGroup<<Self::Component as Component>::Instance>,
            ) {
                for component in self.values() {
                    component.buffer_all(world, buffer);
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

            fn components<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a> {
                Box::new(self.values().map(|c| c as _))
            }

            fn components_mut<'a>(
                &'a mut self,
            ) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a> {
                Box::new(self.values_mut().map(|c| c as _))
            }
            // fn iter<'a>(&'a self) -> impl Iterator<Item = &Self::Component> + 'a where Self: Sized {
            //     self.values()
            // }
        }
    };
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
