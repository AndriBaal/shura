use crate::{
    entity::{ConstIdentifier, ConstTypeId, EntityHandle},
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait ComponentIdentifier: ConstIdentifier + Component {}

#[allow(unused_variables)]
pub trait Component: Downcast {
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn component(&self, idx: u32) -> Option<&dyn Component> {
        None
    }
    fn component_mut(&mut self, idx: u32) -> Option<&mut dyn Component> {
        None
    }
    fn component_identifiers() -> &'static [(ConstTypeId, u32)]
    where
        Self: Sized,
    {
        &[]
    }

    fn component_identifiers_recursive() -> Vec<(ConstTypeId, Vec<u32>)>
    where
        Self: Sized,
    {
        vec![]
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
