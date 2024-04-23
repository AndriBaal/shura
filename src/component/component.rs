use crate::{
    entity::EntityHandle,
    graphics::{Instance, RenderGroup, RenderGroupManager},
    math::AABB,
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

#[derive(Clone, Copy)]
pub enum ComponentType<'a> {
    Component(&'a dyn Component),
    ComponentBundle(&'a dyn ComponentBundle),
}

impl<'a> ComponentType<'a> {
    pub fn as_component(self) -> Option<&'a dyn Component> {
        match self {
            Self::Component(component) => Some(component),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn as_bundle(self) -> Option<&'a dyn ComponentBundle> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => Some(bundle),
        }
    }

    pub fn cast_as_component<C: Component>(self) -> Option<&'a C> {
        match self {
            Self::Component(component) => component.downcast_ref(),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn cast_as_bundle<CB: ComponentBundle>(self) -> Option<&'a CB> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => bundle.downcast_ref(),
        }
    }
}

pub enum ComponentTypeMut<'a> {
    Component(&'a mut dyn Component),
    ComponentBundle(&'a mut dyn ComponentBundle),
}

impl<'a> ComponentTypeMut<'a> {
    pub fn as_component(self) -> Option<&'a dyn Component> {
        match self {
            Self::Component(component) => Some(component),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn as_bundle(self) -> Option<&'a dyn ComponentBundle> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => Some(bundle),
        }
    }

    pub fn as_component_mut(self) -> Option<&'a mut dyn Component> {
        match self {
            Self::Component(component) => Some(component),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn as_bundle_mut(self) -> Option<&'a mut dyn ComponentBundle> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => Some(bundle),
        }
    }

    pub fn cast_as_component<C: Component>(self) -> Option<&'a C> {
        match self {
            Self::Component(component) => component.downcast_ref(),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn cast_as_bundle<CB: ComponentBundle>(self) -> Option<&'a CB> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => bundle.downcast_ref(),
        }
    }

    pub fn cast_as_component_mut<C: Component>(self) -> Option<&'a mut C> {
        match self {
            Self::Component(component) => component.downcast_mut(),
            Self::ComponentBundle(_) => None,
        }
    }

    pub fn cast_as_bundle_mut<CB: ComponentBundle>(self) -> Option<&'a mut CB> {
        match self {
            Self::Component(_) => None,
            Self::ComponentBundle(bundle) => bundle.downcast_mut(),
        }
    }
}

pub trait BufferComponentBundleIterator<'a, CB: ComponentBundle>:
    Iterator<Item = &'a CB> + Clone + 'a
{
}
impl<'a, CB: ComponentBundle, I: Iterator<Item = &'a CB> + Clone + 'a>
    BufferComponentBundleIterator<'a, CB> for I
{
}

pub trait ComponentBundle: 'static + Downcast {
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
    fn component<'a>(&'a self, tag: &'static str) -> Option<ComponentType<'a>>;
    fn component_mut<'a>(&'a mut self, tag: &'static str) -> Option<ComponentTypeMut<'a>>;
}
impl_downcast!(ComponentBundle);

#[allow(unused_variables)]
pub trait Component: Downcast {
    type Instance: Instance
    where
        Self: Sized;
    fn buffer(&self, world: &World, cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn remove_from_world(&self, _world: &mut World) {}
}
impl_downcast!(Component);

impl<I: Instance + Clone> Component for I {
    type Instance = I where Self: Sized;

    fn buffer(&self, _world: &World, _cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
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
            render_group: &mut RenderGroup<Self::Instance>,
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
                cam2d: &AABB,
                render_group: &mut RenderGroup<Self::Instance>,
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

impl<const U: usize, C: Component> Component for [C; U] {
    impl_collection_inner!();
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
