use instant::Instant;

use crate::{
    data::arena::ArenaIndex, ArenaIter, ArenaIterMut, ArenaPath, BoxedComponent,
    ComponentCallbacks, ComponentConfig, ComponentDerive, ComponentGroupId, ComponentType,
    InstanceBuffer, InstanceIndex, ComponentHandle,
};
use std::{iter::Enumerate, marker::PhantomData};

#[derive(Clone)]
pub(crate) struct ComponentCluster {
    paths: Vec<ArenaPath>,
    config: ComponentConfig,
    last_update: Option<Instant>,
}

impl ComponentCluster {
    pub fn new(path: ArenaPath, config: ComponentConfig, now: Instant) -> Self {
        Self {
            paths: vec![path],
            last_update: match &config.update {
                crate::UpdateOperation::AfterDuration(_) => Some(now),
                _ => None,
            },
            config: config,
        }
    }

    pub fn sort(&mut self) {
        self.paths
            .sort_by(|a, b| a.group_index.index().cmp(&b.group_index.index()));
    }

    pub fn clear(&mut self) {
        self.paths.clear();
    }

    pub fn add(&mut self, path: ArenaPath) {
        self.paths.push(path);
    }

    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    pub fn update_time(&mut self, now: Instant) {
        match &mut self.config.update {
            crate::UpdateOperation::AfterDuration(dur) => {
                if now > self.last_update.unwrap() + *dur {
                    self.last_update = Some(now);
                }
            }
            _ => {}
        };
    }

    pub const fn config(&self) -> &ComponentConfig {
        &self.config
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &Vec<ArenaPath> {
        &self.paths
    }
}

/// Paths of currently active ComponentGroup's that contain the component type C. This is equal to
/// [ComponentFilter::Active](crate::ComponentFilter::Active)
pub struct ActiveComponents<'a, C: ComponentDerive> {
    paths: &'a [ArenaPath],
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ActiveComponents<'a, C> {
    pub(crate) fn new(paths: &'a [ArenaPath]) -> Self {
        Self {
            paths,
            marker: PhantomData,
        }
    }

    pub(crate) fn paths(&self) -> &[ArenaPath] {
        self.paths
    }

    pub fn amount_of_groups(&self) -> usize {
        self.paths.len()
    }
}

// #[derive(Clone)]
pub struct ComponentSetMut<'a, C: ComponentDerive> {
    ty: &'a mut ComponentType,
    groups: &'a [ArenaIndex],
    // len: usize,
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ComponentSetMut<'a, C> {
    fn retain(&mut self, mut keep: impl FnMut(&mut C) -> bool) {
        for group in self.groups {
            if let Some(group) = self.ty.groups.get(*group) {
                group.components.retain(|_, component| {
                    let component = component.downcast_mut::<C>().unwrap();
                    keep(component)
                });
            }
        }
    }
    fn len(&self) -> usize {
        self.len()
    }
    fn remove(&mut self, handle: ComponentHandle) -> Option<C> {
        self.ty.remove(handle).and_then(|c| c.downcast::<C>().ok());
    }
    fn index(group_id: ComponentGroupId, index: u32) -> Option<&C> {
        self.ty.remove(handle).and_then(|c| c.downcast::<C>().ok());
    }
    fn index_mut(group_id: ComponentGroupId, index: u32) -> Option<&mut C> {}
    fn component() -> Option<&C> {}
    fn component_mut()  -> Option<&mut C> {}
}

// /// A set of components that includes all components of a specific type from a variety of
// /// [ComponentGroups](crate::ComponentGroup).
// /// A [ComponentSet] can be retrieved from the [Context](crate::Context) with
// /// [components](crate::Context::components) or [components_mut](crate::Context::components_mut).
// pub struct ComponentSet<'a, C: ComponentDerive> {
//     iters: Vec<ArenaIter<'a, BoxedComponent>>,
//     iter_index: usize,
//     len: usize,
//     marker: PhantomData<C>,
// }

// impl<C: ComponentDerive> Clone for ComponentSet<'_, C> {
//     #[inline]
//     fn clone(&self) -> Self {
//         Self {
//             iters: self.iters.clone(),
//             marker: PhantomData,
//             iter_index: self.iter_index,
//             len: self.len,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ComponentSet<'a, C> {
//     pub(crate) fn new(
//         iters: Vec<ArenaIter<'a, BoxedComponent>>,
//         len: usize,
//     ) -> ComponentSet<'a, C> {
//         ComponentSet {
//             iters,
//             iter_index: 0,
//             len,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ExactSizeIterator for ComponentSet<'a, C> {
//     fn len(&self) -> usize {
//         self.len
//     }
// }

// impl<'a, C: ComponentDerive> Iterator for ComponentSet<'a, C> {
//     type Item = &'a C;
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(iter) = self.iters.get_mut(self.iter_index) {
//             if let Some((_, c)) = iter.next() {
//                 return c.as_ref().downcast_ref::<C>();
//             } else {
//                 self.iter_index += 1;
//                 return self.next();
//             }
//         }
//         return None;
//     }
// }

// impl<'a, C: ComponentDerive> DoubleEndedIterator for ComponentSet<'a, C> {
//     fn next_back(&mut self) -> Option<&'a C> {
//         let len = self.iters.len();
//         if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
//             if let Some((_, c)) = iter.next_back() {
//                 return c.as_ref().downcast_ref::<C>();
//             } else {
//                 self.iter_index += 1;
//                 return self.next_back();
//             }
//         }
//         return None;
//     }
// }

// /// A set of components that includes all components of a specific type from a variety of
// /// [ComponentGroups](crate::ComponentGroup).
// /// A [ComponentSet] can be retrieved from the [Context](crate::Context) with
// /// [components](crate::Context::components) or [components_mut](crate::Context::components_mut).
// pub struct ComponentSetMut<'a, C: ComponentDerive> {
//     iters: Vec<ArenaIterMut<'a, BoxedComponent>>,
//     iter_index: usize,
//     len: usize,
//     marker: PhantomData<C>,
// }

// impl<'a, C: ComponentDerive> ComponentSetMut<'a, C> {
//     pub(crate) fn new(
//         iters: Vec<ArenaIterMut<'a, BoxedComponent>>,
//         len: usize,
//     ) -> ComponentSetMut<'a, C> {
//         ComponentSetMut {
//             iters,
//             iter_index: 0,
//             len,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ExactSizeIterator for ComponentSetMut<'a, C> {
//     fn len(&self) -> usize {
//         self.len
//     }
// }

// impl<'a, C: ComponentDerive> Iterator for ComponentSetMut<'a, C> {
//     type Item = &'a mut C;
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(iter) = self.iters.get_mut(self.iter_index) {
//             if let Some((_, c)) = iter.next() {
//                 return c.as_mut().downcast_mut::<C>();
//             } else {
//                 self.iter_index += 1;
//                 return self.next();
//             }
//         }
//         return None;
//     }
// }

// impl<'a, C: ComponentDerive> DoubleEndedIterator for ComponentSetMut<'a, C> {
//     fn next_back(&mut self) -> Option<&'a mut C> {
//         let len = self.iters.len();
//         if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
//             if let Some((_, c)) = iter.next_back() {
//                 return c.as_mut().downcast_mut::<C>();
//             } else {
//                 self.iter_index += 1;
//                 return self.next_back();
//             }
//         }
//         return None;
//     }
// }

// /// Iterator that yields all components from a given [ComponentGroup](crate::ComponentGroup) and the
// /// corresponding [InstanceBuffer]
// pub struct ComponentRenderGroup<'a, C: ComponentDerive> {
//     iters: Vec<(&'a InstanceBuffer, ComponentIterRender<'a, C>)>,
//     iter_index: usize,
//     len: usize,
//     marker: PhantomData<C>,
// }

// impl<C: ComponentDerive> Clone for ComponentRenderGroup<'_, C> {
//     #[inline]
//     fn clone(&self) -> Self {
//         Self {
//             iters: self.iters.clone(),
//             marker: PhantomData,
//             iter_index: self.iter_index,
//             len: self.len,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ComponentRenderGroup<'a, C> {
//     pub(crate) fn new(
//         iters: Vec<(&'a InstanceBuffer, ComponentIterRender<'a, C>)>,
//         len: usize,
//     ) -> ComponentRenderGroup<'a, C> {
//         ComponentRenderGroup {
//             iters,
//             iter_index: 0,
//             len,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ExactSizeIterator for ComponentRenderGroup<'a, C> {
//     fn len(&self) -> usize {
//         self.len
//     }
// }

// impl<'a, C: ComponentDerive> Iterator for ComponentRenderGroup<'a, C> {
//     type Item = (&'a InstanceBuffer, ComponentIterRender<'a, C>);
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(iter) = self.iters.get_mut(self.iter_index) {
//             self.iter_index += 1;
//             return Some(iter.clone());
//         }
//         return None;
//     }
// }

// impl<'a, C: ComponentDerive> DoubleEndedIterator for ComponentRenderGroup<'a, C> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         let len = self.len();
//         if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
//             self.iter_index += 1;
//             return Some(iter.clone());
//         }
//         return None;
//     }
// }

// /// Iterator that yields a component and the corresponding [InstanceIndex] in the [InstanceBuffer]
// pub struct ComponentIterRender<'a, C: ComponentDerive> {
//     iter: Enumerate<ArenaIter<'a, BoxedComponent>>,
//     marker: PhantomData<C>,
// }

// impl<C: ComponentDerive> Clone for ComponentIterRender<'_, C> {
//     #[inline]
//     fn clone(&self) -> Self {
//         Self {
//             iter: self.iter.clone(),
//             marker: PhantomData,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ComponentIterRender<'a, C> {
//     pub(crate) fn new(
//         iter: Enumerate<ArenaIter<'a, BoxedComponent>>,
//     ) -> ComponentIterRender<'a, C> {
//         ComponentIterRender {
//             iter,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C: ComponentDerive> ExactSizeIterator for ComponentIterRender<'a, C> {
//     fn len(&self) -> usize {
//         self.iter.len()
//     }
// }

// impl<'a, C: ComponentDerive> Iterator for ComponentIterRender<'a, C> {
//     type Item = (InstanceIndex, &'a C);
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some((i, entry)) = self.iter.next() {
//             let i = i as u32;
//             return Some((
//                 InstanceIndex { index: i },
//                 entry.1.downcast_ref::<C>().unwrap(),
//             ));
//         }
//         return None;
//     }
// }

// impl<'a, C: ComponentDerive> DoubleEndedIterator for ComponentIterRender<'a, C> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         if let Some((i, entry)) = self.iter.next_back() {
//             let i = i as u32;
//             return Some((
//                 InstanceIndex { index: i - 1 },
//                 entry.1.downcast_ref::<C>().unwrap(),
//             ));
//         }
//         return None;
//     }
// }
