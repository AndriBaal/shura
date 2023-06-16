use crate::{
    BoxedComponent, ComponentController, ComponentHandle, ComponentType, GroupHandle,
    InstanceBuffer, InstanceIndex,
};
use std::marker::PhantomData;

#[derive(Clone, Copy)]
/// Set of components from  the same type only from the specified (groups)[crate::Group]
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

    pub fn for_each(&mut self, each: impl FnMut(&C)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn index(&self, index: usize) -> Option<&C> {
        self.index_of(GroupHandle::DEFAULT_GROUP, 0)
    }

    pub fn index_of(&self, group: GroupHandle, index: usize) -> Option<&C> {
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

/// Set of mutable components from  the same type only from the specified (groups)[crate::Group]
pub struct ComponentSetMut<'a, C: ComponentController> {
    ty: &'a mut ComponentType,
    groups: &'a [GroupHandle],
    marker: PhantomData<C>,
    check: bool,
}

impl<'a, C: ComponentController> ComponentSetMut<'a, C> {
    pub(crate) fn new(
        ty: &'a mut ComponentType,
        groups: &'a [GroupHandle],
        check: bool,
    ) -> ComponentSetMut<'a, C> {
        Self {
            ty,
            groups,
            check,
            marker: PhantomData,
        }
    }

    pub fn for_each(&self, each: impl FnMut(&C)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_mut(&mut self, each: impl FnMut(&mut C)) {
        self.ty.for_each_mut(self.groups, each);
    }

    pub fn retain(&mut self, keep: impl FnMut(&mut C) -> bool) {
        self.ty.retain(self.groups, keep);
    }

    pub fn index(&self, index: usize) -> Option<&C> {
        self.index_of(GroupHandle::DEFAULT_GROUP, 0)
    }

    pub fn index_mut(&mut self, index: usize) -> Option<&mut C> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, 0)
    }

    pub fn index_of(&self, group: GroupHandle, index: usize) -> Option<&C> {
        self.ty.index(group, index)
    }

    pub fn index_mut_of(&mut self, group: GroupHandle, index: usize) -> Option<&mut C> {
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

    pub fn remove_all(&mut self) -> Vec<C> {
        self.ty.remove_all(self.groups)
    }

    pub fn add(&mut self, component: C) -> ComponentHandle {
        self.add_to(GroupHandle::DEFAULT_GROUP, component)
    }

    pub fn add_to(&mut self, group_handle: GroupHandle, component: C) -> ComponentHandle {
        self.ty.add(group_handle, component)
    }

    pub fn add_many(&mut self, components: impl IntoIterator<Item = C>) -> Vec<ComponentHandle> {
        self.add_many_to(GroupHandle::DEFAULT_GROUP, components)
    }

    pub fn add_many_to(
        &mut self,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.ty.add_many::<C>(group_handle, components)
    }

    pub fn add_with(&mut self, create: impl FnOnce(ComponentHandle) -> C) -> ComponentHandle {
        self.add_with_to(GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to(
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
        self.ty.iter_mut(self.groups, self.check)
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
        self.ty.iter_mut_with_handles(self.groups, self.check)
    }
}
