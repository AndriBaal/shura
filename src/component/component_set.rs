use crate::{
    Component, ComponentHandle, ComponentType, ComponentTypeId, Gpu, GroupHandle, InstanceBuffer,
    InstanceIndex, InstanceIndices, InstancePosition, Renderer, World,
};
use std::cell::{Ref, RefMut};

/// Set of components from  the same type only from the specified (groups)[crate::Group]
pub struct ComponentSet<'a, C: Component> {
    ty: Ref<'a, ComponentType<C>>,
    groups: &'a [GroupHandle],
}

impl<'a, C: Component> ComponentSet<'a, C> {
    pub(crate) fn new(
        ty: Ref<'a, ComponentType<C>>,
        groups: &'a [GroupHandle],
    ) -> ComponentSet<'a, C> {
        Self { ty, groups }
    }

    pub fn component_type_id(&self) -> ComponentTypeId {
        self.ty.component_type_id()
    }

    pub fn for_each(&self, each: impl FnMut(&C)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_with_handles(&self, each: impl FnMut(ComponentHandle, &C)) {
        self.ty.for_each_with_handles(self.groups, each);
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

    pub fn len(&self) -> usize {
        self.ty.len(self.groups)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.ty.iter(self.groups)
    }

    pub fn iter_with_handles(&self) -> impl DoubleEndedIterator<Item = (ComponentHandle, &C)> {
        self.ty.iter_with_handles(self.groups)
    }

    pub fn try_single(&self) -> Option<&C> {
        self.ty.try_single()
    }

    pub fn single(&self) -> &C {
        self.ty.try_single().unwrap()
    }

    pub fn try_single_ref(self) -> Option<Ref<'a, C>> {
        Ref::filter_map(self.ty, |ty| ty.try_single()).ok()
    }

    pub fn single_ref(self) -> Ref<'a, C> {
        Ref::map(self.ty, |ty| ty.single())
    }

    pub fn render_each(
        &'a self,
        renderer: &mut Renderer<'a>,
        each: impl FnMut(&mut Renderer<'a>, &'a C, &'a InstanceBuffer<InstancePosition>, InstanceIndex),
    ) {
        self.ty.render_each(renderer, each)
    }

    pub fn render_single(
        &'a self,
        renderer: &mut Renderer<'a>,
        each: impl FnOnce(&mut Renderer<'a>, &'a C, &'a InstanceBuffer<InstancePosition>, InstanceIndex),
    ) {
        self.ty.render_single(renderer, each)
    }

    pub fn render_all(
        &'a self,
        renderer: &mut Renderer<'a>,
        all: impl FnMut(&mut Renderer<'a>, &'a InstanceBuffer<InstancePosition>, InstanceIndices),
    ) {
        self.ty.render_all(renderer, all)
    }
}

#[cfg(feature = "rayon")]
impl<'a, C: Component + Send + Sync> ComponentSet<'a, C> {
    pub fn par_for_each(&self, each: impl Fn(&C) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }
}

/// Set of mutable components from  the same type only from the specified (groups)[crate::Group]
pub struct ComponentSetMut<'a, C: Component> {
    ty: RefMut<'a, ComponentType<C>>,
    groups: &'a [GroupHandle],
}

impl<'a, C: Component> ComponentSetMut<'a, C> {
    pub(crate) fn new(
        ty: RefMut<'a, ComponentType<C>>,
        groups: &'a [GroupHandle],
        check: bool,
    ) -> ComponentSetMut<'a, C> {
        #[cfg(debug_assertions)]
        if check && groups.len() > 1 {
            for (index, value) in groups.iter().enumerate() {
                for other in groups.iter().skip(index + 1) {
                    assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                }
            }
        }
        Self { ty, groups }
    }

    pub fn component_type_id(&self) -> ComponentTypeId {
        self.ty.component_type_id()
    }

    pub fn change_group(
        &mut self,
        component: ComponentHandle,
        new_group_handle: GroupHandle,
    ) -> Option<ComponentHandle> {
        self.ty.change_group(component, new_group_handle)
    }

    pub fn for_each(&self, each: impl FnMut(&C)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_mut(&mut self, each: impl FnMut(&mut C)) {
        self.ty.for_each_mut(self.groups, each);
    }

    pub fn for_each_with_handles(&self, each: impl FnMut(ComponentHandle, &C)) {
        self.ty.for_each_with_handles(self.groups, each);
    }

    pub fn for_each_mut_with_handles(&mut self, each: impl FnMut(ComponentHandle, &mut C)) {
        self.ty.for_each_mut_with_handles(self.groups, each);
    }

    pub fn buffer_for_each_mut(
        &mut self,
        world: &World,
        gpu: &Gpu,
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        self.ty.buffer_for_each_mut(world, gpu, self.groups, each)
    }

    pub fn retain(&mut self, world: &mut World, keep: impl FnMut(&mut C, &mut World) -> bool) {
        self.ty.retain(world, self.groups, keep);
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
        self.ty.get2_mut(handle1, handle2)
    }

    pub fn remove(&mut self, world: &mut World, handle: ComponentHandle) -> Option<C> {
        self.ty.remove(world, handle)
    }

    pub fn remove_all(&mut self, world: &mut World) -> Vec<C> {
        self.ty.remove_all(world, self.groups)
    }

    pub fn add(&mut self, world: &mut World, component: C) -> ComponentHandle {
        self.add_to(world, GroupHandle::DEFAULT_GROUP, component)
    }

    pub fn add_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        self.ty.add(world, group_handle, component)
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(world, GroupHandle::DEFAULT_GROUP, components)
    }

    pub fn add_many_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.ty.add_many(world, group_handle, components)
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(world, GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.ty.add_with(world, group_handle, create)
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
        &'a self,
    ) -> impl DoubleEndedIterator<Item = (&InstanceBuffer<InstancePosition>, InstanceIndex, &C)>
    {
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

    pub fn try_single(&self) -> Option<&C> {
        self.ty.try_single()
    }

    pub fn single(&self) -> &C {
        self.ty.single()
    }

    pub fn try_single_mut(&mut self) -> Option<&mut C> {
        self.ty.try_single_mut()
    }

    pub fn single_mut(&mut self) -> &mut C {
        self.ty.single_mut()
    }

    pub fn try_single_ref(self) -> Option<RefMut<'a, C>> {
        RefMut::filter_map(self.ty, |ty| ty.try_single_mut()).ok()
    }

    pub fn single_ref(self) -> RefMut<'a, C> {
        RefMut::map(self.ty, |ty| ty.single_mut())
    }

    pub fn remove_single(&mut self, world: &mut World) -> Option<C> {
        self.ty.remove_single(world)
    }

    pub fn set_single(&mut self, world: &mut World, new: C) -> ComponentHandle {
        self.ty.set_single(world, new)
    }

    pub fn set_single_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.ty.set_single_with(world, create)
    }

    pub fn render_each(
        &'a self,
        renderer: &mut Renderer<'a>,
        each: impl FnMut(&mut Renderer<'a>, &'a C, &'a InstanceBuffer<InstancePosition>, InstanceIndex),
    ) {
        self.ty.render_each(renderer, each)
    }

    pub fn render_single(
        &'a self,
        renderer: &mut Renderer<'a>,
        each: impl FnOnce(&mut Renderer<'a>, &'a C, &'a InstanceBuffer<InstancePosition>, InstanceIndex),
    ) {
        self.ty.render_single(renderer, each)
    }

    pub fn render_all(
        &'a self,
        renderer: &mut Renderer<'a>,
        all: impl FnMut(&mut Renderer<'a>, &'a InstanceBuffer<InstancePosition>, InstanceIndices),
    ) {
        self.ty.render_all(renderer, all)
    }
}

#[cfg(feature = "rayon")]
impl<'a, C: Component + Send + Sync> ComponentSetMut<'a, C> {
    pub fn par_for_each(&self, each: impl Fn(&C) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }

    pub fn par_for_each_mut(&mut self, each: impl Fn(&mut C) + Send + Sync) {
        self.ty.par_for_each_mut(self.groups, each);
    }
}
