use crate::{
    entity::EntityHandle,
    graphics::{Instance, RenderGroup},
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait Component: Downcast {
    type Instance: Instance where Self: Sized;
    fn buffer(
        &self,
        world: &World,
        render_group: &mut RenderGroup<Self::Instance>,
    ) where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
    // fn children(&self) -> impl Iterator<Item = &Self> where Self: Sized;
    // fn children_mut(&mut self) -> impl Iterator<Item = &mut Self> where Self: Sized;
    // fn components<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a>;
    // fn components_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a>;
}
impl_downcast!(Component);

impl <I: Instance + Clone> Component for I {
    type Instance = I where Self: Sized;

    fn buffer(
        &self,
        world: &World,
        render_group: &mut RenderGroup<Self::Instance>,
    ) where
        Self: Sized {
        render_group.push(self.clone())
    }

    fn init(&mut self, handle: EntityHandle, world: &mut World) {
    }

    fn finish(&mut self, world: &mut World) {
    }
}

macro_rules! impl_collection_inner {
    () => {
        type Instance = C::Instance;
        fn buffer(
            &self,
            world: &World,
            render_group: &mut RenderGroup<Self::Instance>,
        ) {
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

        // fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a> {
        //     Box::new(self.iter().map(|c| c as _))
        // }

        // fn instances_mut<'a>(
        //     &'a mut self,
        // ) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a> {
        //     Box::new(self.iter_mut().map(|c| c as _))
        // }
    }
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
            fn buffer(
                &self,
                world: &World,
                render_group: &mut RenderGroup<Self::Instance>,
            ) {
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

            // fn instances<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a> {
            //     Box::new(self.values().map(|c| c as _))
            // }

            // fn instances_mut<'a>(
            //     &'a mut self,
            // ) -> Box<dyn Iterator<Item = &mut dyn Component> + 'a> {
            //     Box::new(self.values_mut().map(|c| c as _))
            // }
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
