use downcast_rs::{impl_downcast, Downcast};
use std::cell::{Ref, RefMut};

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

use crate::{
    Arena, ComponentBufferManager, Entity, EntityHandle, EntityIdentifier, EntityIndex,
    EntityTypeId, GroupHandle, GroupManager, World
};

#[allow(unused_variables)]
pub trait EntityType: Downcast {
    type Entity: EntityIdentifier
    where
        Self: Sized;
    fn buffer(&self, buffers: &mut ComponentBufferManager, groups: &GroupManager, world: &World);
    fn entity_type_id(&self) -> EntityTypeId;
    fn remove_group(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
    ) -> Option<Box<dyn EntityType>> {
        None
    }
    fn add_group(&mut self) {}

    fn iter_dyn<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Entity> + 'a>;
    fn iter_dyn_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Entity> + 'a>;

    fn iter_render<'a>(
        &'a self,
        groups: &GroupManager,
    ) -> impl Iterator<Item = &Self::Entity> + Clone + 'a
    where
        Self: Sized;
}
impl_downcast!(EntityType);

pub trait SingleEntityRef<'a, E: Entity> {
    fn get(self) -> Option<Ref<'a, E>>;
}

pub trait SingleEntityRefMut<'a, E: Entity> {
    fn get_mut(self) -> Option<RefMut<'a, E>>;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct SingleEntity<E: Entity> {
    entity: Option<E>,
}

impl<E: Entity> Default for SingleEntity<E> {
    fn default() -> Self {
        Self { entity: None }
    }
}

impl<'a, E: Entity> SingleEntityRef<'a, E> for Ref<'a, SingleEntity<E>> {
    fn get(self) -> Option<Ref<'a, E>> {
        Ref::filter_map(self, |ty| ty.entity.as_ref()).ok()
    }
}

impl<'a, E: Entity> SingleEntityRefMut<'a, E> for RefMut<'a, SingleEntity<E>> {
    fn get_mut(self) -> Option<RefMut<'a, E>> {
        RefMut::filter_map(self, |ty| ty.entity.as_mut()).ok()
    }
}

impl<E: EntityIdentifier> SingleEntity<E> {
    pub fn handle(&self) -> Option<EntityHandle> {
        self.entity
            .as_ref()
            .map(|_e| EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID))
    }

    pub fn is_some(&self) -> bool {
        self.entity.is_some()
    }

    // pub fn get(&self) -> Option<&E> {
    //     self.entity.as_ref()
    // }

    // pub fn get_mut(&mut self) -> Option<&mut E> {
    //     self.entity.as_mut()
    // }

    pub fn remove(&mut self, world: &mut World) -> Option<E> {
        if let Some(mut entity) = self.entity.take() {
            entity.finish(world);
            return Some(entity);
        }
        None
    }

    pub fn set(&mut self, world: &mut World, mut new: E) -> EntityHandle {
        let handle = EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
        new.init(handle, world);
        if let Some(mut _old) = self.entity.replace(new) {
            _old.finish(world);
        }
        handle
    }

    pub fn set_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        let handle = EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
        let mut new = create(handle);
        new.init(handle, world);
        if let Some(mut _old) = self.entity.replace(new) {
            _old.finish(world);
        }
        handle
    }
}

impl<E: EntityIdentifier> EntityType for SingleEntity<E> {
    type Entity = E;
    fn buffer(&self, buffers: &mut ComponentBufferManager, groups: &GroupManager, world: &World) {
        E::buffer(self.iter_render(groups), buffers, world);
    }

    fn entity_type_id(&self) -> EntityTypeId {
        E::IDENTIFIER
    }

    fn iter_dyn<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Entity> + 'a> {
        Box::new(self.entity.iter().map(|e| e as &dyn Entity))
    }

    fn iter_dyn_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Entity> + 'a> {
        Box::new(self.entity.iter_mut().map(|e| e as &mut dyn Entity))
    }

    fn iter_render<'a>(
        &'a self,
        _groups: &GroupManager,
    ) -> impl Iterator<Item = &Self::Entity> + Clone + 'a
    where
        Self: Sized,
    {
        self.entity.iter()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct Entities<E: Entity> {
    entities: Arena<E>,
}

impl<E: Entity> Default for Entities<E> {
    fn default() -> Self {
        Self {
            entities: Arena::new(),
        }
    }
}

impl<E: EntityIdentifier> Entities<E> {
    pub fn retain(&mut self, world: &mut World, mut keep: impl FnMut(&mut E, &mut World) -> bool) {
        self.entities.retain(|_, entity| {
            if keep(entity, world) {
                true
            } else {
                entity.finish(world);
                false
            }
        });
    }

    pub fn index(&self, index: usize) -> Option<&E> {
        self.entities.get_unknown_gen(index)
    }

    pub fn index_mut(&mut self, index: usize) -> Option<&mut E> {
        self.entities.get_unknown_gen_mut(index)
    }

    pub fn get(&self, handle: EntityHandle) -> Option<&E> {
        self.entities.get(handle.entity_index().0)
    }

    pub fn get_mut(&mut self, handle: EntityHandle) -> Option<&mut E> {
        self.entities.get_mut(handle.entity_index().0)
    }

    pub fn get2_mut(
        &mut self,
        handle1: EntityHandle,
        handle2: EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        self.entities
            .get2_mut(handle1.entity_index().0, handle2.entity_index().0)
    }

    pub fn remove(&mut self, world: &mut World, handle: EntityHandle) -> Option<E> {
        if let Some(mut entity) = self.entities.remove(handle.entity_index().0) {
            entity.finish(world);
            return Some(entity);
        }
        None
    }

    pub fn remove_all(&mut self, world: &mut World) -> Vec<E> {
        let mut result = Vec::with_capacity(self.entities.len());
        let entities = std::mem::take(&mut self.entities);
        for mut entity in entities {
            entity.finish(world);
            result.push(entity)
        }
        result
    }

    pub fn add(&mut self, world: &mut World, mut new: E) -> EntityHandle {
        let mut handle = Default::default();
        self.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        let mut handle = Default::default();
        self.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
            let mut new = create(handle);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        let entities = entities.into_iter();
        let mut handles = Vec::with_capacity(entities.size_hint().0);
        for mut entity in entities {
            self.entities.insert_with(|idx| {
                let handle =
                    EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
                entity.init(handle, world);
                handles.push(handle);
                entity
            });
        }
        handles
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() > 0
    }

    pub fn iter<'a>(&'a self) -> impl EntityIterator<'a, E> {
        self.entities.iter()
    }

    pub fn iter_with_handles<'a>(&'a self) -> impl EntityIteratorWithHandle<'a, E> {
        self.entities.iter_with_index().map(|(idx, c)| {
            (
                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                c,
            )
        })
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl EntityIteratorMut<'a, E> {
        self.entities.iter_mut()
    }

    pub fn iter_mut_with_handles<'a>(&'a mut self) -> impl EntityIteratorMutWithHandle<'a, E> {
        self.entities.iter_mut_with_index().map(|(idx, c)| {
            (
                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                c,
            )
        })
    }
}

impl<E: EntityIdentifier> EntityType for Entities<E> {
    type Entity = E;

    fn buffer(&self, buffers: &mut ComponentBufferManager, groups: &GroupManager, world: &World) {
        E::buffer(self.iter_render(groups), buffers, world);
    }
    
    fn entity_type_id(&self) -> EntityTypeId {
        E::IDENTIFIER
    }

    fn iter_dyn<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Entity> + 'a> {
        Box::new(self.entities.iter().map(|e| e as &dyn Entity))
    }

    fn iter_dyn_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Entity> + 'a> {
        Box::new(self.entities.iter_mut().map(|e| e as &mut dyn Entity))
    }

    fn iter_render<'a>(
        &'a self,
        _groups: &GroupManager,
    ) -> impl Iterator<Item = &Self::Entity> + Clone + 'a
    where
        Self: Sized,
    {
        self.entities.iter()
    }
}

#[cfg(feature = "rayon")]
impl<E: Entity + Send + Sync> Entities<E> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = &E> {
        self.entities.items.par_iter().filter_map(|e| match e {
            ArenaEntry::Free { .. } => None,
            ArenaEntry::Occupied { data, .. } => Some(data),
        })
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = &mut E> {
        self.entities.items.par_iter_mut().filter_map(|e| match e {
            ArenaEntry::Free { .. } => None,
            ArenaEntry::Occupied { data, .. } => Some(data),
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct GroupedEntities<ET: EntityType> {
    groups: Arena<ET>,
}

impl<ET: EntityType> Default for GroupedEntities<ET> {
    fn default() -> Self {
        Self {
            groups: Arena::new(),
        }
    }
}

impl<ET: EntityType> GroupedEntities<ET> {
    pub fn get_group(&self, group: GroupHandle) -> Option<&ET> {
        return self.groups.get(group.0);
    }

    pub fn get_group_mut(&mut self, group: GroupHandle) -> Option<&mut ET> {
        return self.groups.get_mut(group.0);
    }
}

impl<E: EntityIdentifier> GroupedEntities<Entities<E>> {
    pub fn retain(
        &mut self,
        world: &mut World,
        group_handles: &[GroupHandle],
        mut keep: impl FnMut(&mut E, &mut World) -> bool,
    ) {
        for group in group_handles {
            if let Some(group) = self.groups.get_mut(group.0) {
                group.entities.retain(|_, entity| {
                    let entity = entity;
                    if keep(entity, world) {
                        true
                    } else {
                        entity.finish(world);
                        false
                    }
                });
            }
        }
    }

    pub fn index(&self, group: GroupHandle, index: usize) -> Option<&E> {
        if let Some(group) = self.groups.get(group.0) {
            return group.entities.get_unknown_gen(index);
        }
        None
    }

    pub fn index_mut(&mut self, group: GroupHandle, index: usize) -> Option<&mut E> {
        if let Some(group) = self.groups.get_mut(group.0) {
            return group.entities.get_unknown_gen_mut(index);
        }
        None
    }

    pub fn get(&self, handle: EntityHandle) -> Option<&E> {
        if let Some(group) = self.groups.get(handle.group_handle().0) {
            return group.entities.get(handle.entity_index().0);
        }
        None
    }

    pub fn get_mut(&mut self, handle: EntityHandle) -> Option<&mut E> {
        if let Some(group) = self.groups.get_mut(handle.group_handle().0) {
            return group.entities.get_mut(handle.entity_index().0);
        }
        None
    }

    pub fn get2_mut(
        &mut self,
        handle1: EntityHandle,
        handle2: EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        let mut e1 = None;
        let mut e2 = None;
        if handle1.group_handle() == handle2.group_handle() {
            if let Some(group) = self.groups.get_mut(handle1.group_handle().0) {
                (e1, e2) = group
                    .entities
                    .get2_mut(handle1.entity_index().0, handle2.entity_index().0);
            }
        } else {
            let (group1, group2) = self
                .groups
                .get2_mut(handle1.group_handle().0, handle2.group_handle().0);
            if let Some(group) = group1 {
                e1 = group.entities.get_mut(handle1.entity_index().0);
            }

            if let Some(group) = group2 {
                e2 = group.entities.get_mut(handle2.entity_index().0);
            }
        }
        (e1, e2)
    }

    pub fn remove(&mut self, world: &mut World, handle: EntityHandle) -> Option<E> {
        if let Some(group) = self.groups.get_mut(handle.group_handle().0) {
            if let Some(mut entity) = group.entities.remove(handle.entity_index().0) {
                entity.finish(world);
                return Some(entity);
            }
        }
        None
    }

    pub fn remove_all(&mut self, world: &mut World, group_handles: &[GroupHandle]) -> Vec<E> {
        let mut result = Vec::new();
        for group_handle in group_handles {
            if let Some(group) = self.groups.get_mut(group_handle.0) {
                let entities = std::mem::take(&mut group.entities);
                for mut entity in entities {
                    entity.finish(world);
                    result.push(entity);
                }
            }
        }
        result
    }

    pub fn add(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        mut new: E,
    ) -> EntityHandle {
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
            let mut new = create(handle);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        let entities = entities.into_iter();
        let mut handles = Vec::with_capacity(entities.size_hint().0);
        if let Some(group) = self.groups.get_mut(group_handle.0) {
            for mut entity in entities {
                group.entities.insert_with(|idx| {
                    let handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
                    entity.init(handle, world);
                    handles.push(handle);
                    entity
                });
            }
        }
        handles
    }

    pub fn len(&self, group_handles: &[GroupHandle]) -> usize {
        let mut len = 0;
        for group in group_handles {
            if let Some(group) = self.groups.get(group.0) {
                len += group.entities.len();
            }
        }
        len
    }

    pub fn is_empty(&self, group_handles: &[GroupHandle]) -> bool {
        for group in group_handles {
            if let Some(group) = self.groups.get(group.0) {
                if !group.entities.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    pub fn change_group(
        &mut self,
        entity: EntityHandle,
        new_group_handle: GroupHandle,
    ) -> Option<EntityHandle> {
        assert!(entity.entity_type_id() == E::IDENTIFIER);
        let (old_group, new_group) = self
            .groups
            .get2_mut(entity.group_handle().0, new_group_handle.0);
        let old_group = old_group?;
        let new_group = new_group?;
        let entity = old_group.entities.remove(entity.entity_index().0)?;
        let entity_index = EntityIndex(new_group.entities.insert(entity));

        Some(EntityHandle::new(
            entity_index,
            E::IDENTIFIER,
            new_group_handle,
        ))
    }
}

impl<ET: EntityType + Default> EntityType for GroupedEntities<ET> {
    type Entity = ET::Entity;

    fn buffer(&self, buffers: &mut ComponentBufferManager, groups: &GroupManager, world: &World) {
        ET::Entity::buffer(self.iter_render(groups), buffers, world);
    }

    fn add_group(&mut self) {
        self.groups.insert(Default::default());
    }

    fn remove_group(
        &mut self,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Box<dyn EntityType>> {
        if let Some(mut group) = self.groups.remove(handle.0) {
            for entity in group.iter_dyn_mut() {
                entity.finish(world)
            }
            return Some(Box::new(group));
        }
        return None;
    }

    fn entity_type_id(&self) -> EntityTypeId {
        ET::Entity::IDENTIFIER
    }

    fn iter_dyn<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Entity> + 'a> {
        Box::new(self.groups.iter().map(|g| g.iter_dyn()).flatten())
    }

    fn iter_dyn_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut dyn Entity> + 'a> {
        Box::new(self.groups.iter_mut().map(|g| g.iter_dyn_mut()).flatten())
    }

    fn iter_render<'a>(
        &'a self,
        groups: &GroupManager,
    ) -> impl Iterator<Item = &Self::Entity> + Clone + 'a
    where
        Self: Sized,
    {
        groups
            .render_groups()
            .iter()
            .map(|g| self.get_group(*g).unwrap().iter_render(groups))
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
    }
}

pub trait RenderEntityIterator<'a, E: Entity>: Iterator<Item = &'a E> + Clone + 'a {}
impl<'a, E: Entity, I: Iterator<Item = &'a E> + Clone + 'a> RenderEntityIterator<'a, E> for I {}

pub trait EntityIterator<'a, E: Entity>:
    DoubleEndedIterator<Item = &'a E> + ExactSizeIterator<Item = &'a E> + Clone
{
}
impl<
        'a,
        E: Entity,
        I: DoubleEndedIterator<Item = &'a E> + ExactSizeIterator<Item = &'a E> + Clone,
    > EntityIterator<'a, E> for I
{
}

pub trait EntityIteratorWithHandle<'a, E: Entity>:
    DoubleEndedIterator<Item = (EntityHandle, &'a E)>
    + ExactSizeIterator<Item = (EntityHandle, &'a E)>
    + Clone
{
}

impl<
        'a,
        E: Entity,
        I: DoubleEndedIterator<Item = (EntityHandle, &'a E)>
            + ExactSizeIterator<Item = (EntityHandle, &'a E)>
            + Clone,
    > EntityIteratorWithHandle<'a, E> for I
{
}

pub trait EntityIteratorMut<'a, E: Entity>:
    DoubleEndedIterator<Item = &'a mut E> + ExactSizeIterator<Item = &'a mut E>
{
}
impl<
        'a,
        E: Entity,
        I: DoubleEndedIterator<Item = &'a mut E> + ExactSizeIterator<Item = &'a mut E>,
    > EntityIteratorMut<'a, E> for I
{
}

pub trait EntityIteratorMutWithHandle<'a, E: Entity>:
    DoubleEndedIterator<Item = (EntityHandle, &'a mut E)>
    + ExactSizeIterator<Item = (EntityHandle, &'a mut E)>
{
}
impl<
        'a,
        E: Entity,
        I: DoubleEndedIterator<Item = (EntityHandle, &'a mut E)>
            + ExactSizeIterator<Item = (EntityHandle, &'a mut E)>,
    > EntityIteratorMutWithHandle<'a, E> for I
{
}
