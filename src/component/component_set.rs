use crate::{
    BoxedComponent, ComponentController, ComponentHandle, ComponentType, GroupHandle,
    InstanceBuffer, InstanceIndex,
};
use std::marker::PhantomData;

#[cfg(feature = "physics")]
use crate::physics::World;

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

    pub fn par_for_each(&self, each: impl Fn(&C) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }

    pub fn index(&self, index: usize) -> Option<&C> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
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

    pub fn single(&self) -> Option<&C> {
        self.ty.single()
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

    pub fn par_for_each(&self, each: impl Fn(&C) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }

    pub fn par_for_each_mut(&mut self, each: impl Fn(&mut C) + Send + Sync) {
        self.ty.par_for_each_mut(self.groups, each);
    }

    pub fn for_each(&self, each: impl FnMut(&C)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_mut(&mut self, each: impl FnMut(&mut C)) {
        self.ty.for_each_mut(self.groups, each);
    }

    pub fn retain(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        #[cfg(feature = "physics")] keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] keep: impl FnMut(&mut C) -> bool,
    ) {
        self.ty.retain(
            #[cfg(feature = "physics")]
            world,
            self.groups,
            keep,
        );
    }

    pub fn index(&self, index: usize) -> Option<&C> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_mut(&mut self, index: usize) -> Option<&mut C> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, index)
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

    pub fn remove(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<C> {
        self.ty.remove(
            #[cfg(feature = "physics")]
            world,
            handle,
        )
    }

    pub fn remove_boxed(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<BoxedComponent> {
        self.ty.remove_boxed(
            #[cfg(feature = "physics")]
            world,
            handle,
        )
    }

    pub fn remove_all(&mut self, #[cfg(feature = "physics")] world: &mut World) -> Vec<C> {
        self.ty.remove_all(
            #[cfg(feature = "physics")]
            world,
            self.groups,
        )
    }

    pub fn add(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        component: C,
    ) -> ComponentHandle {
        self.add_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            component,
        )
    }

    pub fn add_to(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        self.ty.add(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            component,
        )
    }

    pub fn add_many(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            components,
        )
    }

    pub fn add_many_to(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.ty.add_many::<C>(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            components,
        )
    }

    pub fn add_with(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            create,
        )
    }

    pub fn add_with_to(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.ty.add_with(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            create,
        )
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
    ) -> impl DoubleEndedIterator<Item = (&InstanceBuffer, InstanceIndex, &C)> {
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

    pub fn single(&self) -> Option<&C> {
        self.ty.single()
    }

    pub fn single_mut(&mut self) -> Option<&mut C> {
        self.ty.single_mut()
    }

    pub fn remove_single(&mut self, #[cfg(feature = "physics")] world: &mut World) -> Option<C> {
        self.ty.remove_single(
            #[cfg(feature = "physics")]
            world,
        )
    }

    pub fn set_single(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        new: C,
    ) -> ComponentHandle {
        self.ty.set_single(
            #[cfg(feature = "physics")]
            world,
            new,
        )
    }

    pub fn set_single_with(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.ty.set_single_with(
            #[cfg(feature = "physics")]
            world,
            create,
        )
    }
}
