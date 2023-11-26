use crate::{Entity, EntityHandle, EntityType, EntityTypeId, GroupHandle, World};
use std::cell::{Ref, RefMut};

pub struct EntitySet<'a, E: Entity> {
    ty: Ref<'a, EntityType<E>>,
    groups: &'a [GroupHandle],
}

impl<'a, E: Entity> EntitySet<'a, E> {
    pub(crate) fn new(ty: Ref<'a, EntityType<E>>, groups: &'a [GroupHandle]) -> EntitySet<'a, E> {
        Self { ty, groups }
    }

    pub fn entity_type_id(&self) -> EntityTypeId {
        self.ty.entity_type_id()
    }

    pub fn for_each(&self, each: impl FnMut(&E)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_with_handles(&self, each: impl FnMut(EntityHandle, &E)) {
        self.ty.for_each_with_handles(self.groups, each);
    }

    pub fn index(&self, index: usize) -> Option<&E> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_of(&self, group: GroupHandle, index: usize) -> Option<&E> {
        self.ty.index(group, index)
    }

    pub fn get(&self, handle: EntityHandle) -> Option<&E> {
        self.ty.get(handle)
    }

    pub fn len(&self) -> usize {
        self.ty.len(self.groups)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &E> {
        self.ty.iter(self.groups)
    }

    pub fn iter_with_handles(&self) -> impl DoubleEndedIterator<Item = (EntityHandle, &E)> {
        self.ty.iter_with_handles(self.groups)
    }

    pub fn try_single(&self) -> Option<&E> {
        self.ty.try_single()
    }

    pub fn single(&self) -> &E {
        self.ty.try_single().unwrap()
    }

    pub fn try_single_ref(self) -> Option<Ref<'a, E>> {
        Ref::filter_map(self.ty, |ty| ty.try_single()).ok()
    }

    pub fn single_ref(self) -> Ref<'a, E> {
        Ref::map(self.ty, |ty| ty.single())
    }
}

#[cfg(feature = "rayon")]
impl<'a, E: Entity + Send + Sync> EntitySet<'a, E> {
    pub fn par_for_each(&self, each: impl Fn(&E) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }

    pub fn par_for_each_collect<C: crate::Component>(
        &self,
        world: &World,
        each: impl Fn(&E) -> &C + Send + Sync,
        collection: &mut Vec<C::Instance>,
    ) where
        C::Instance: Send + Sync,
    {
        self.ty
            .par_for_each_collect(world, self.groups, each, collection);
    }
}

pub struct EntitySetMut<'a, E: Entity> {
    ty: RefMut<'a, EntityType<E>>,
    groups: &'a [GroupHandle],
}

impl<'a, E: Entity> EntitySetMut<'a, E> {
    pub(crate) fn new(
        ty: RefMut<'a, EntityType<E>>,
        groups: &'a [GroupHandle],
        _check: bool,
    ) -> EntitySetMut<'a, E> {
        #[cfg(debug_assertions)]
        if _check && groups.len() > 1 {
            for (index, value) in groups.iter().enumerate() {
                for other in groups.iter().skip(index + 1) {
                    assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                }
            }
        }
        Self { ty, groups }
    }

    pub fn entity_type_id(&self) -> EntityTypeId {
        self.ty.entity_type_id()
    }

    pub fn change_group(
        &mut self,
        entity: EntityHandle,
        new_group_handle: GroupHandle,
    ) -> Option<EntityHandle> {
        self.ty.change_group(entity, new_group_handle)
    }

    pub fn for_each(&self, each: impl FnMut(&E)) {
        self.ty.for_each(self.groups, each);
    }

    pub fn for_each_mut(&mut self, each: impl FnMut(&mut E)) {
        self.ty.for_each_mut(self.groups, each);
    }

    pub fn for_each_with_handles(&self, each: impl FnMut(EntityHandle, &E)) {
        self.ty.for_each_with_handles(self.groups, each);
    }

    pub fn for_each_mut_with_handles(&mut self, each: impl FnMut(EntityHandle, &mut E)) {
        self.ty.for_each_mut_with_handles(self.groups, each);
    }

    pub fn retain(&mut self, world: &mut World, keep: impl FnMut(&mut E, &mut World) -> bool) {
        self.ty.retain(world, self.groups, keep);
    }

    pub fn index(&self, index: usize) -> Option<&E> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_mut(&mut self, index: usize) -> Option<&mut E> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_of(&self, group: GroupHandle, index: usize) -> Option<&E> {
        self.ty.index(group, index)
    }

    pub fn index_mut_of(&mut self, group: GroupHandle, index: usize) -> Option<&mut E> {
        self.ty.index_mut(group, index)
    }

    pub fn get(&self, handle: EntityHandle) -> Option<&E> {
        self.ty.get(handle)
    }

    pub fn get_mut(&mut self, handle: EntityHandle) -> Option<&mut E> {
        self.ty.get_mut(handle)
    }

    pub fn get2_mut(
        &mut self,
        handle1: EntityHandle,
        handle2: EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        self.ty.get2_mut(handle1, handle2)
    }

    pub fn remove(&mut self, world: &mut World, handle: EntityHandle) -> Option<E> {
        self.ty.remove(world, handle)
    }

    pub fn remove_all(&mut self, world: &mut World) -> Vec<E> {
        self.ty.remove_all(world, self.groups)
    }

    pub fn add(&mut self, world: &mut World, entity: E) -> EntityHandle {
        self.add_to(world, GroupHandle::DEFAULT_GROUP, entity)
    }

    pub fn add_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entity: E,
    ) -> EntityHandle {
        self.ty.add(world, group_handle, entity)
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        self.add_many_to(world, GroupHandle::DEFAULT_GROUP, entities)
    }

    pub fn add_many_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        self.ty.add_many(world, group_handle, entities)
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        self.add_with_to(world, GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        self.ty.add_with(world, group_handle, create)
    }

    pub fn len(&self) -> usize {
        self.ty.len(self.groups)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &E> {
        self.ty.iter(self.groups)
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut E> {
        self.ty.iter_mut(self.groups)
    }

    pub fn iter_with_handles(&self) -> impl DoubleEndedIterator<Item = (EntityHandle, &E)> {
        self.ty.iter_with_handles(self.groups)
    }

    pub fn iter_mut_with_handles(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (EntityHandle, &mut E)> {
        self.ty.iter_mut_with_handles(self.groups)
    }

    pub fn try_single(&self) -> Option<&E> {
        self.ty.try_single()
    }

    pub fn single(&self) -> &E {
        self.ty.single()
    }

    pub fn try_single_mut(&mut self) -> Option<&mut E> {
        self.ty.try_single_mut()
    }

    pub fn single_mut(&mut self) -> &mut E {
        self.ty.single_mut()
    }

    pub fn try_single_ref(self) -> Option<RefMut<'a, E>> {
        RefMut::filter_map(self.ty, |ty| ty.try_single_mut()).ok()
    }

    pub fn single_ref(self) -> RefMut<'a, E> {
        RefMut::map(self.ty, |ty| ty.single_mut())
    }

    pub fn remove_single(&mut self, world: &mut World) -> Option<E> {
        self.ty.remove_single(world)
    }

    pub fn set_single(&mut self, world: &mut World, new: E) -> EntityHandle {
        self.ty.set_single(world, new)
    }

    pub fn set_single_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        self.ty.set_single_with(world, create)
    }
}

#[cfg(feature = "rayon")]
impl<'a, E: Entity + Send + Sync> EntitySetMut<'a, E> {
    pub fn par_for_each(&self, each: impl Fn(&E) + Send + Sync) {
        self.ty.par_for_each(self.groups, each);
    }

    pub fn par_for_each_mut(&mut self, each: impl Fn(&mut E) + Send + Sync) {
        self.ty.par_for_each_mut(self.groups, each);
    }

    pub fn par_for_each_collect<C: crate::Component>(
        &self,
        world: &World,
        each: impl Fn(&E) -> &C + Send + Sync,
        collection: &mut Vec<C::Instance>,
    ) where
        C::Instance: Send + Sync,
    {
        self.ty
            .par_for_each_collect(world, self.groups, each, collection);
    }

    pub fn par_for_each_collect_mut<C: crate::Component>(
        &mut self,
        world: &World,
        each: impl Fn(&mut E) -> &C + Send + Sync,
        collection: &mut Vec<C::Instance>,
    ) where
        C::Instance: Send + Sync,
    {
        self.ty
            .par_for_each_collect_mut(world, self.groups, each, collection);
    }
}
