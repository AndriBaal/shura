use crate::{
    entity::EntityHandle,
    graphics::{Instance, InstanceRenderGroup, RenderGroupManager},
    math::AABB,
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

#[allow(unused_variables)]
pub trait MetaComponent: Downcast {
    fn is_bundle(&self) -> bool {
        false
    }
    fn component<'a>(&'a self, tag: &'static str) -> Option<&'a dyn MetaComponent> {
        None
    }
    fn component_mut<'a>(&'a mut self, tag: &'static str) -> Option<&'a mut dyn MetaComponent> {
        None
    }
}
impl_downcast!(MetaComponent);

pub trait BufferComponentBundleIterator<'a, CB: ComponentBundle>:
    Iterator<Item = &'a CB> + Clone + 'a
{
}
impl<'a, CB: ComponentBundle, I: Iterator<Item = &'a CB> + Clone + 'a>
    BufferComponentBundleIterator<'a, CB> for I
{
}

pub trait ComponentBundle: MetaComponent {
    fn buffer<'a>(
        entites: impl BufferComponentBundleIterator<'a, Self>,
        buffers: &mut RenderGroupManager,
        world: &World,
        cam2d: &AABB,
    ) where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
    fn remove_from_world(&self, _world: &mut World) {}
    fn tags() -> &'static [&'static str]
    where
        Self: Sized;
}

#[allow(unused_variables)]
pub trait Component: MetaComponent {
    type Instance: Instance
    where
        Self: Sized;
    fn buffer(&self, world: &World, cam2d: &AABB, render_group: &mut InstanceRenderGroup<Self::Instance>)
    where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn remove_from_world(&self, _world: &mut World) {}
}


impl<I: Instance + Clone> MetaComponent for I {}
impl<I: Instance + Clone> Component for I {
    type Instance = I where Self: Sized;

    fn buffer(&self, _world: &World, _cam2d: &AABB, render_group: &mut InstanceRenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        render_group.push(*self)
    }
    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}
    fn finish(&mut self, _world: &mut World) {}
}

macro_rules! impl_collection_inner {
    () => {
        type Instance = C::Instance;
        fn buffer(
            &self,
            world: &World,
            cam2d: &AABB,
            render_group: &mut InstanceRenderGroup<Self::Instance>,
        ) {
            for component in self.iter() {
                component.buffer(world, cam2d, render_group);
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
        impl<C: MetaComponent> MetaComponent for $collection {}
        impl<C: Component> Component for $collection {
            impl_collection_inner!();
        }
    };
}

macro_rules! impl_collection_map {
    ($collection: ty) => {
        impl<K: 'static, C: MetaComponent> MetaComponent for $collection {}
        impl<K: 'static, C: Component> Component for $collection {
            type Instance = C::Instance;
            fn buffer(
                &self,
                world: &World,
                cam2d: &AABB,
                render_group: &mut InstanceRenderGroup<Self::Instance>,
            ) {
                for component in self.values() {
                    component.buffer(world, cam2d, render_group);
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

impl<const U: usize, C: MetaComponent> MetaComponent for [C; U] {}
impl<const U: usize, C: Component> Component for [C; U] {
    impl_collection_inner!();
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
