use crate::{context::Context, entity::EntityHandle, physics::World};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait BufferComponentIterator<'a, C: Component>: Iterator<Item = &'a C> + Clone + 'a {}
impl<'a, C: Component, I: Iterator<Item = &'a C> + Clone + 'a> BufferComponentIterator<'a, C>
    for I
{
}

#[allow(unused_variables)]
pub trait Component: Downcast {
    fn buffer<'a>(entites: impl BufferComponentIterator<'a, Self>, ctx: &Context)
    where
        Self: Sized,
    {
    }

    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn remove_from_world(&self, world: &mut World) {}
    fn component<'a>(&'a self, tag: &'static str) -> Option<&'a dyn Component> {
        None
    }
    fn component_mut<'a>(&'a mut self, tag: &'static str) -> Option<&'a mut dyn Component> {
        None
    }
    fn tags() -> &'static [&'static str]
    where
        Self: Sized,
    {
        &[]
    }
}
impl_downcast!(Component);

macro_rules! impl_collection_inner {
    () => {
        fn init(&mut self, handle: EntityHandle, world: &mut World) {
            for component in self.iter_mut() {
                component.init(handle, world);
            }
        }

        fn finish(&mut self, world: &mut World) {
            for component in self.iter_mut() {
                component.finish(world);
            }
        }

        fn remove_from_world(&self, world: &mut World) {
            for component in self.iter() {
                component.remove_from_world(world);
            }
        }
    };
}

macro_rules! impl_collection {
    ($collection: ty) => {
        impl<C: Component> Component for $collection {
            impl_collection_inner!();
        }
    };
}

macro_rules! impl_collection_map {
    ($collection: ty) => {
        impl<K: 'static, C: Component> Component for $collection {
            fn init(&mut self, handle: EntityHandle, world: &mut World) {
                for component in self.values_mut() {
                    component.init(handle, world);
                }
            }

            fn finish(&mut self, world: &mut World) {
                for component in self.values_mut() {
                    component.finish(world);
                }
            }

            fn remove_from_world(&self, world: &mut World) {
                for component in self.values() {
                    component.remove_from_world(world);
                }
            }
        }
    };
}

impl<const U: usize, C: Component> Component for [C; U] {
    impl_collection_inner!();
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
