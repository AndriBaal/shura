use crate::{
    BoxedComponent, ComponentController, ComponentHandle, ComponentType, GroupHandle,
    InstanceBuffer, InstanceIndex,
};
use std::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct ComponentSet<'a, C: ComponentController> {
    ty: &'a ComponentType,
    groups: &'a [GroupHandle],
    marker: PhantomData<C>,
}

impl<'a, C: ComponentController> ComponentSet<'a, C> {
    pub(crate) fn new(ty: &'a ComponentType, groups: &'a [GroupHandle]) -> ComponentSet<'a, C> {
        Self {
            ty,
            groups,
            marker: PhantomData,
        }
    }

    pub fn each(&mut self, each: impl FnMut(&C)) {
        self.ty.each(self.groups, each);
    }

    pub fn index(&self, group: GroupHandle, index: usize) -> Option<&C> {
        self.ty.index(group, index)
    }

    pub fn get(&self, handle: ComponentHandle) -> Option<&C> {
        self.ty.get(handle)
    }

    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        self.ty.get_boxed(handle)
    }

    pub fn len(&self) -> usize {
        self.ty.len(self.groups)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.ty.iter(self.groups)
    }
}

pub struct ComponentSetMut<'a, C: ComponentController> {
    ty: &'a mut ComponentType,
    groups: &'a [GroupHandle],
    marker: PhantomData<C>,
}

impl<'a, C: ComponentController> ComponentSetMut<'a, C> {
    pub(crate) fn new(
        ty: &'a mut ComponentType,
        groups: &'a [GroupHandle],
    ) -> ComponentSetMut<'a, C> {
        Self {
            ty,
            groups,
            marker: PhantomData,
        }
    }

    pub fn each(&self, each: impl FnMut(&C)) {
        self.ty.each(self.groups, each);
    }

    pub fn each_mut(&mut self, each: impl FnMut(&mut C)) {
        self.ty.each_mut(self.groups, each);
    }

    pub fn retain(&mut self, keep: impl FnMut(&mut C) -> bool) {
        self.ty.retain(self.groups, keep);
    }

    pub fn index(&self, group: GroupHandle, index: usize) -> Option<&C> {
        self.ty.index(group, index)
    }

    pub fn index_mut(&mut self, group: GroupHandle, index: usize) -> Option<&mut C> {
        self.ty.index_mut(group, index)
    }

    pub fn get(&self, handle: ComponentHandle) -> Option<&C> {
        self.ty.get(handle)
    }

    pub fn get_mut(&mut self, handle: ComponentHandle) -> Option<&mut C> {
        self.ty.get_mut(handle)
    }

    pub fn get2_mut(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C>, Option<&mut C>) {
        self.ty.get2_mut::<C, C>(handle1, handle2)
    }

    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        self.ty.get_boxed(handle)
    }

    pub fn get_boxed_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        self.ty.get_boxed_mut(handle)
    }

    pub fn remove(&mut self, handle: ComponentHandle) -> Option<C> {
        self.ty.remove(handle)
    }

    pub fn remove_boxed(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
        self.ty.remove_boxed(handle)
    }

    pub fn remove_all(&mut self) -> Vec<(GroupHandle, Vec<C>)> {
        self.ty.remove_all(self.groups)
    }

    pub fn add(&mut self, group_handle: GroupHandle, component: C) -> ComponentHandle {
        self.ty.add(group_handle, component)
    }

    pub fn add_many(
        &mut self,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.ty.add_many::<C>(group_handle, components)
    }

    pub fn add_with(
        &mut self,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.ty.add_with(group_handle, create)
    }

    pub fn force_buffer(&mut self) {
        self.ty.force_buffer(self.groups)
    }

    pub fn len(&self) -> usize {
        self.ty.len(self.groups)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.ty.iter(self.groups)
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut C> {
        self.ty.iter_mut(self.groups)
    }

    pub fn iter_render(
        &self,
    ) -> impl DoubleEndedIterator<
        Item = (
            &InstanceBuffer,
            impl DoubleEndedIterator<Item = (InstanceIndex, &C)> + Clone,
        ),
    > {
        self.ty.iter_render(self.groups)
    }

    pub fn iter_with_handles(&self) -> impl DoubleEndedIterator<Item = (ComponentHandle, &C)> {
        self.ty.iter_with_handles(self.groups)
    }

    pub fn iter_mut_with_handles(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &mut C)> {
        self.ty.iter_mut_with_handles(self.groups)
    }
}

// impl<'a, C> IntoIterator for &'a mut ComponentSetMut<'a, C>
// where
//     C: ComponentController,
// {
//     type Item = &'a C;
//     type IntoIter = impl Iterator<Item = &mut C>;

//     fn into_iter(self) -> Self::IntoIter {
//         return self.iter();
//     }
// }

// impl<'a, C> IntoIterator for &'a ComponentSetMut<'a, C>
// where
//     C: ComponentController,
// {
//     type Item = &'a C;
//     type IntoIter = ComponentSetIter<'a, C>;

//     fn into_iter(self) -> Self::IntoIter {
//         return self.iter();
//     }
// }

// impl<'a, C> IntoIterator for &'a ComponentSet<'a, C>
// where
//     C: ComponentController,
// {
//     type Item = &'a C;
//     type IntoIter = ComponentSetIter<'a, C>;

//     fn into_iter(self) -> Self::IntoIter {
//         return self.iter();
//     }
// }

// /// Iterator over a [ComponentSet], which holds components from multiple [Groups](crate::Group).
// pub struct ComponentSetIter<'a, C>
// where
//     C: ComponentController,
// {
//     ty: &'a ComponentType,
//     groups: &'a [GroupHandle],
//     group_index: usize,
//     component_index: usize,
//     marker: PhantomData<C>,
// }

// impl<'a, C> ComponentSetIter<'a, C>
// where
//     C: ComponentController,
// {
//     pub(crate) fn new(ty: &'a ComponentType, groups: &'a [GroupHandle]) -> ComponentSetIter<'a, C> {
//         ComponentSetIter {
//             ty,
//             groups,
//             group_index: 0,
//             component_index: 0,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C> Iterator for ComponentSetIter<'a, C>
// where
//     C: ComponentController,
// {
//     type Item = &'a C;
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(group) = self.ty.group(self.groups[self.group_index]) {
//             if let Some(component) = group.components.get_unknown_gen(self.component_index) {
//                 self.component_index += 1;
//                 return component.as_ref().downcast_ref::<C>();
//             } else {
//                 self.component_index = 0;
//                 self.group_index = 0;
//                 return self.next();
//             }
//         }
//         return None;
//     }
// }

// impl<'a, C> ExactSizeIterator for ComponentSetIter<'a, C>
// where
//     C: ComponentController,
// {
//     fn len(&self) -> usize {
//         self.ty.len(self.groups)
//     }
// }

// impl<'a, C> DoubleEndedIterator for ComponentSetIter<'a, C>
// where
//     C: ComponentController,
// {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         if let Some(group) = self
//             .ty
//             .group(self.groups[self.groups.len() - 1 - self.group_index])
//         {
//             if let Some(component) = group
//                 .components
//                 .get_unknown_gen(group.components.len() - 1 - self.component_index)
//             {
//                 self.component_index += 1;
//                 return component.as_ref().downcast_ref::<C>();
//             } else {
//                 self.component_index = 0;
//                 self.group_index = 0;
//                 return self.next_back();
//             }
//         }
//         return None;
//     }
// }

// pub struct ComponentSetIterMut<'a, C>
// where
//     C: ComponentController,
// {
//     ty: &'a mut ComponentType,
//     groups: &'a [GroupHandle],
//     group_index: usize,
//     component_index: usize,
//     marker: PhantomData<C>,
// }

// impl<'a, C> ComponentSetIterMut<'a, C>
// where
//     C: ComponentController,
// {
//     pub(crate) fn new(ty: &'a mut ComponentType, groups: &'a [GroupHandle]) -> ComponentSetIterMut<'a, C> {
//         ComponentSetIterMut {
//             ty,
//             groups,
//             group_index: 0,
//             component_index: 0,
//             marker: PhantomData::<C>,
//         }
//     }
// }

// impl<'a, C> Iterator for ComponentSetIterMut<'a, C>
// where
//     C: ComponentController,
// {
//     type Item = &'a mut C;
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(group) = self.ty.group_mut(self.groups[self.group_index]) {
//             if let Some(component) = group.components.get_unknown_gen(self.component_index) {
//                 self.component_index += 1;
//                 return component.as_mut().downcast_mut::<C>();
//             } else {
//                 self.component_index = 0;
//                 self.group_index = 0;
//                 return self.next();
//             }
//         }
//         return None;
//     }
// }

// impl<'a, C> ExactSizeIterator for ComponentSetIterMut<'a, C>
// where
//     C: ComponentController,
// {
//     fn len(&self) -> usize {
//         self.ty.len(self.groups)
//     }
// }

// impl<'a, C> DoubleEndedIterator for ComponentSetIterMut<'a, C>
// where
//     C: ComponentController,
// {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         if let Some(group) = self
//             .ty
//             .group_mut(self.groups[self.groups.len() - 1 - self.group_index])
//         {
//             if let Some(component) = group
//                 .components
//                 .get_unknown_gen_mut(group.components.len() - 1 - self.component_index)
//             {
//                 self.component_index += 1;
//                 return component.as_mut().downcast_mut::<C>();
//             } else {
//                 self.component_index = 0;
//                 self.group_index = 0;
//                 return self.next_back();
//             }
//         }
//         return None;
//     }
// }

// /// A set of components that includes all components of a specific type from a variety of
// /// [Groups](crate::Group).
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
// /// [Groups](crate::Group).
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

// /// Iterator that yields all components from a given [Group](crate::Group) and the
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
