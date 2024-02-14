use crate::{
    entity::EntityHandle,
    graphics::{Instance, RenderGroup},
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait Component: Downcast {
    type Instance: Instance
    where
        Self: Sized;
    fn buffer(&self, world: &World, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
    fn remove_from_world(&self, _world: &mut World) {}
}
impl_downcast!(Component);

impl<I: Instance + Clone> Component for I {
    type Instance = I where Self: Sized;

    fn buffer(&self, _world: &World, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        render_group.push(self.clone())
    }

    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}

    fn finish(&mut self, _world: &mut World) {}
}

macro_rules! impl_collection_inner {
    () => {
        type Instance = C::Instance;
        fn buffer(&self, world: &World, render_group: &mut RenderGroup<Self::Instance>) {
            for component in self.iter() {
                component.buffer(world, render_group);
            }
        }

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
            type Instance = C::Instance;
            fn buffer(&self, world: &World, render_group: &mut RenderGroup<Self::Instance>) {
                for component in self.values() {
                    component.buffer(world, render_group);
                }
            }

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
