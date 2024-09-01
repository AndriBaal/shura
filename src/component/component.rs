use crate::{entity::EntityHandle, physics::World};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::{BTreeMap, HashMap, LinkedList, VecDeque};

pub trait ComponentIdentifier: Component {
    const NAME: &'static str;
}

#[allow(unused_variables)]
pub trait Component: Downcast {
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn remove_from_world(&self, world: &mut World) {}
    fn component_dyn(&self, tag: &str) -> Option<&dyn Component> {
        None
    }
    fn component_mut_dyn(&mut self, tag: &str) -> Option<&mut dyn Component> {
        None
    }
    fn each_self_dyn(&self, each: &mut dyn FnMut(&dyn Component)) {
        each(self.as_component())
    }
    fn each_self_mut_dyn(&mut self, each: &mut dyn FnMut(&mut dyn Component)) {
        each(self.as_component_mut())
    }
    fn as_component(&self) -> &dyn Component;
    fn as_component_mut(&mut self) -> &mut dyn Component;
    fn tags() -> &'static [&'static str]
    where
        Self: Sized,
    {
        &[]
    }
    fn tags_recursive() -> Vec<(&'static str, Vec<&'static str>)>
    where
        Self: Sized,
    {
        vec![]
    }
}
impl_downcast!(Component);

impl dyn Component {
    pub fn component<C: ComponentIdentifier>(&self) -> Option<&C> {
        self.component_dyn(C::NAME)
            .and_then(|c| c.downcast_ref::<C>())
    }

    pub fn component_mut<C: ComponentIdentifier>(&mut self) -> Option<&mut C> {
        self.component_mut_dyn(C::NAME)
            .and_then(|c| c.downcast_mut::<C>())
    }

    pub fn each<C: ComponentIdentifier>(
        &self,
        tag: &'static str,
        mut each: impl FnMut(&C) + 'static,
    ) {
        if let Some(component) = self.component_dyn(tag) {
            component.each_self_dyn(&mut move |c| {
                let c = c.downcast_ref::<C>().unwrap();
                each(c)
            })
        }
    }

    pub fn each_mut<C: ComponentIdentifier>(
        &mut self,
        tag: &'static str,
        mut each: impl FnMut(&mut C) + 'static,
    ) {
        if let Some(component) = self.component_mut_dyn(tag) {
            component.each_self_mut_dyn(&mut move |c| {
                let c = c.downcast_mut::<C>().unwrap();
                each(c)
            })
        }
    }
}

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

        fn each_self_dyn(&self, each: &mut dyn FnMut(&dyn Component)) {
            for component in self.iter() {
                each(component)
            }
        }

        fn each_self_mut_dyn(&mut self, each: &mut dyn FnMut(&mut dyn Component)) {
            for component in self.iter_mut() {
                each(component)
            }
        }

        fn as_component(&self) -> &dyn Component {
            self as _
        }

        fn as_component_mut(&mut self) -> &mut dyn Component {
            self as _
        }
    };
}

macro_rules! impl_collection {
    ($collection: ty) => {
        impl<C: ComponentIdentifier> ComponentIdentifier for $collection {
            const NAME: &'static str = C::NAME;
        }

        impl<C: Component> Component for $collection {
            impl_collection_inner!();
        }
    };
}

macro_rules! impl_collection_map {
    ($collection: ty) => {
        impl<K: 'static, C: ComponentIdentifier> ComponentIdentifier for $collection {
            const NAME: &'static str = C::NAME;
        }

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

            fn each_self_dyn(&self, each: &mut dyn FnMut(&dyn Component)) {
                for component in self.values() {
                    each(component)
                }
            }

            fn each_self_mut_dyn(&mut self, each: &mut dyn FnMut(&mut dyn Component)) {
                for component in self.values_mut() {
                    each(component)
                }
            }

            fn as_component(&self) -> &dyn Component {
                self as _
            }

            fn as_component_mut(&mut self) -> &mut dyn Component {
                self as _
            }
        }
    };
}

impl<const U: usize, C: Component> Component for [C; U] {
    impl_collection_inner!();
}
impl<const U: usize, C: ComponentIdentifier> ComponentIdentifier for [C; U] {
    const NAME: &'static str = C::NAME;
}

impl_collection!(Vec<C>);
impl_collection!(Option<C>);
impl_collection!(LinkedList<C>);
impl_collection!(VecDeque<C>);
impl_collection_map!(BTreeMap<K, C>);
impl_collection_map!(HashMap<K, C>);
