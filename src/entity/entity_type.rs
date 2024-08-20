use downcast_rs::{impl_downcast, Downcast};
use std::cell::{Ref, RefMut};

#[cfg(feature = "rayon")]
use crate::{arena::ArenaEntry, rayon::prelude::*};

use crate::{
    arena::{Arena, ArenaIndex, ArenaIter, ArenaIterMut},
    entity::{
        Entity, EntityGroupHandle, EntityGroupManager, EntityHandle, ConstTypeId, EntityIdentifier,
        EntityIndex, ConstIdentifier
    },
    physics::World,
};

#[allow(unused_variables)]
pub trait EntityType: Downcast {
    type Entity: EntityIdentifier
    where
        Self: Sized;
    fn entity_type_id(&self) -> ConstTypeId;
    fn remove_group(
        &mut self,
        world: &mut World,
        group_handle: &EntityGroupHandle,
    ) -> Option<Box<dyn EntityType>> {
        None
    }
    fn add_group(&mut self) {}

    fn dyn_get(&self, handle: &EntityHandle) -> Option<&dyn Entity>;
    fn dyn_get_mut(&mut self, handle: &EntityHandle) -> Option<&mut dyn Entity>;
    fn dyn_retain(&mut self, world: &mut World, keep: &dyn Fn(&mut dyn Entity, &mut World) -> bool);
    fn dyn_remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<Box<dyn Entity>>;

    fn dyn_iter<'a>(&'a self) -> Box<dyn Iterator<Item = (EntityHandle, &dyn Entity)> + 'a>;
    fn dyn_iter_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (EntityHandle, &mut dyn Entity)> + 'a>;
    fn iter_render<'a>(
        &'a self,
        groups: &'a EntityGroupManager,
    ) -> Box<dyn Iterator<Item = &dyn Entity> + 'a>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl_downcast!(EntityType);

pub trait SingleEntityRef<'a, E: Entity> {
    fn get_ref(self) -> Option<Ref<'a, E>>;
    fn unwrap(self) -> Ref<'a, E>
    where
        Self: Sized,
    {
        self.get_ref().unwrap()
    }
}

pub trait SingleEntityRefMut<'a, E: Entity> {
    fn get_ref(self) -> Option<RefMut<'a, E>>;
    fn unwrap(self) -> RefMut<'a, E>
    where
        Self: Sized,
    {
        self.get_ref().unwrap()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct SingleEntity<E: Entity> {
    entity: Option<E>,
    generation: u32,
}

impl<E: Entity> Default for SingleEntity<E> {
    fn default() -> Self {
        Self {
            entity: None,
            generation: 0,
        }
    }
}

impl<'a, E: Entity> SingleEntityRef<'a, E> for Ref<'a, SingleEntity<E>> {
    fn get_ref(self) -> Option<Ref<'a, E>> {
        Ref::filter_map(self, |ty| ty.entity.as_ref()).ok()
    }
}

impl<'a, E: Entity> SingleEntityRefMut<'a, E> for RefMut<'a, SingleEntity<E>> {
    fn get_ref(self) -> Option<RefMut<'a, E>> {
        RefMut::filter_map(self, |ty| ty.entity.as_mut()).ok()
    }
}

impl<E: EntityIdentifier> SingleEntity<E> {
    pub fn is_some(&self) -> bool {
        self.entity.is_some()
    }

    pub fn get(&self) -> Option<&E> {
        self.entity.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut E> {
        self.entity.as_mut()
    }

    pub fn is_none(&self) -> bool {
        self.entity.is_none()
    }

    pub fn handle(&self) -> Option<EntityHandle> {
        if self.entity.is_some() {
            return Some(EntityHandle::new(
                EntityIndex(ArenaIndex {
                    index: 0,
                    generation: self.generation,
                }),
                E::IDENTIFIER,
                EntityGroupHandle::INVALID,
            ));
        }
        None
    }

    pub fn get_by_handle(&self, handle: &EntityHandle) -> Option<&E> {
        if let Some(entity) = &self.entity {
            if handle.entity_index.0.generation == self.generation {
                return Some(entity);
            }
        }
        None
    }

    pub fn get_by_handle_mut(&mut self, handle: &EntityHandle) -> Option<&mut E> {
        if let Some(entity) = &mut self.entity {
            if handle.entity_index.0.generation == self.generation {
                return Some(entity);
            }
        }
        None
    }

    pub fn remove(&mut self, world: &mut World) -> Option<E> {
        if let Some(entity) = &mut self.entity {
            entity.finish(world);
        }
        self.entity.take()
    }

    pub fn set(&mut self, world: &mut World, mut new: E) -> EntityHandle {
        self.generation += 1;
        let handle = EntityHandle::new(
            EntityIndex(ArenaIndex {
                index: 0,
                generation: self.generation,
            }),
            E::IDENTIFIER,
            EntityGroupHandle::INVALID,
        );
        new.init(handle, world);
        if let Some(mut old) = self.entity.replace(new) {
            old.finish(world);
        }
        handle
    }

    pub fn set_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        self.generation += 1;
        let handle = EntityHandle::new(
            EntityIndex(ArenaIndex {
                index: 0,
                generation: self.generation,
            }),
            E::IDENTIFIER,
            EntityGroupHandle::INVALID,
        );
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
    fn entity_type_id(&self) -> ConstTypeId {
        E::IDENTIFIER
    }
    fn iter_render<'a>(
        &'a self,
        _groups: &'a EntityGroupManager,
    ) -> Box<dyn Iterator<Item = &dyn Entity> + 'a>
    where
        Self: Sized,
    {
        Box::new(self.entity.iter().map(|e| e as &dyn Entity))
    }

    fn dyn_iter<'a>(&'a self) -> Box<dyn Iterator<Item = (EntityHandle, &dyn Entity)> + 'a> {
        Box::new(self.entity.iter().map(|e| {
            (
                EntityHandle::new(
                    EntityIndex(ArenaIndex {
                        index: 0,
                        generation: self.generation,
                    }),
                    E::IDENTIFIER,
                    EntityGroupHandle::INVALID,
                ),
                e as &dyn Entity,
            )
        }))
    }

    fn dyn_iter_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (EntityHandle, &mut dyn Entity)> + 'a> {
        Box::new(self.entity.iter_mut().map(|e| {
            (
                EntityHandle::new(
                    EntityIndex(ArenaIndex {
                        index: 0,
                        generation: self.generation,
                    }),
                    E::IDENTIFIER,
                    EntityGroupHandle::INVALID,
                ),
                e as &mut dyn Entity,
            )
        }))
    }

    fn dyn_get(&self, handle: &EntityHandle) -> Option<&dyn Entity> {
        if let Some(entity) = &self.entity {
            if handle.entity_index.0.generation == self.generation {
                return Some(entity);
            }
        }
        None
    }

    fn dyn_get_mut(&mut self, handle: &EntityHandle) -> Option<&mut dyn Entity> {
        if let Some(entity) = &mut self.entity {
            if handle.entity_index.0.generation == self.generation {
                return Some(entity);
            }
        }
        None
    }

    fn dyn_retain(
        &mut self,
        world: &mut World,
        keep: &dyn Fn(&mut dyn Entity, &mut World) -> bool,
    ) {
        if let Some(entity) = &mut self.entity {
            let result = keep(entity, world);
            if !result {
                entity.finish(world);
                self.entity = None;
            }
        }
    }

    fn dyn_remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<Box<dyn Entity>> {
        if handle.entity_index.0.generation == self.generation {
            if let Some(mut entity) = self.entity.take() {
                entity.finish(world);
                return Some(Box::new(entity));
            }
        }
        None
    }

    fn len(&self) -> usize {
        self.entity.is_some() as _
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
    pub fn retain(&mut self, world: &mut World, keep: impl Fn(&mut E, &mut World) -> bool) {
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

    pub fn get(&self, handle: &EntityHandle) -> Option<&E> {
        if handle.type_id != E::IDENTIFIER {
            return None;
        }
        self.entities.get(handle.entity_index.0)
    }

    pub fn get_mut(&mut self, handle: &EntityHandle) -> Option<&mut E> {
        if handle.type_id != E::IDENTIFIER {
            return None;
        }
        self.entities.get_mut(handle.entity_index.0)
    }

    pub fn get2_mut(
        &mut self,
        mut handle1: EntityHandle,
        mut handle2: EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        if handle1.type_id != E::IDENTIFIER {
            handle1 = EntityHandle::INVALID;
        }
        if handle2.type_id != E::IDENTIFIER {
            handle2 = EntityHandle::INVALID;
        }
        self.entities
            .get2_mut(handle1.entity_index.0, handle2.entity_index.0)
    }

    pub fn remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<E> {
        if handle.type_id != E::IDENTIFIER {
            return None;
        }
        if let Some(mut entity) = self.entities.remove(handle.entity_index.0) {
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
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, EntityGroupHandle::INVALID);
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
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, EntityGroupHandle::INVALID);
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
                    EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, EntityGroupHandle::INVALID);
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

    pub fn iter(&self) -> ArenaIter<'_, E> {
        self.entities.iter()
    }

    pub fn iter_with_handles(&self) -> impl ExactSizeIterator<Item = (EntityHandle, &'_ E)> {
        self.entities.iter_with_index().map(|(idx, c)| {
            (
                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, EntityGroupHandle::INVALID),
                c,
            )
        })
    }

    pub fn iter_mut(&mut self) -> ArenaIterMut<'_, E> {
        self.entities.iter_mut()
    }

    pub fn iter_mut_with_handles(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (EntityHandle, &'_ mut E)> {
        self.entities.iter_mut_with_index().map(|(idx, c)| {
            (
                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, EntityGroupHandle::INVALID),
                c,
            )
        })
    }
}

impl<'a, E: EntityIdentifier> IntoIterator for &'a Entities<E> {
    type Item = &'a E;
    type IntoIter = ArenaIter<'a, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, E: EntityIdentifier> IntoIterator for &'a mut Entities<E> {
    type Item = &'a mut E;
    type IntoIter = ArenaIterMut<'a, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<E: EntityIdentifier> EntityType for Entities<E> {
    type Entity = E;

    fn entity_type_id(&self) -> ConstTypeId {
        E::IDENTIFIER
    }

    fn iter_render<'a>(
        &'a self,
        _groups: &'a EntityGroupManager,
    ) -> Box<dyn Iterator<Item = &dyn Entity> + 'a>
    where
        Self: Sized,
    {
        Box::new(self.entities.iter().map(|e| e as &dyn Entity))
    }

    fn dyn_iter<'a>(&'a self) -> Box<dyn Iterator<Item = (EntityHandle, &dyn Entity)> + 'a> {
        Box::new(self.iter_with_handles().map(|(h, e)| (h, e as &dyn Entity)))
    }

    fn dyn_iter_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (EntityHandle, &mut dyn Entity)> + 'a> {
        Box::new(
            self.iter_mut_with_handles()
                .map(|(h, e)| (h, e as &mut dyn Entity)),
        )
    }

    fn dyn_get(&self, handle: &EntityHandle) -> Option<&dyn Entity> {
        if let Some(entity) = self.get(handle) {
            return Some(entity);
        }
        None
    }

    fn dyn_get_mut(&mut self, handle: &EntityHandle) -> Option<&mut dyn Entity> {
        if let Some(entity) = self.get_mut(handle) {
            return Some(entity);
        }
        None
    }

    fn dyn_retain(
        &mut self,
        world: &mut World,
        keep: &dyn Fn(&mut dyn Entity, &mut World) -> bool,
    ) {
        self.entities.retain(|_, entity| {
            if keep(entity, world) {
                true
            } else {
                entity.finish(world);
                false
            }
        });
    }

    fn dyn_remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<Box<dyn Entity>> {
        if handle.type_id != E::IDENTIFIER {
            return None;
        }
        if let Some(mut entity) = self.entities.remove(handle.entity_index.0) {
            entity.finish(world);
            return Some(Box::new(entity));
        }
        None
    }

    fn len(&self) -> usize {
        self.len()
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
    pub fn get_group(&self, group: &EntityGroupHandle) -> Option<&ET> {
        return self.groups.get(group.0);
    }

    pub fn get_group_mut(&mut self, group: &EntityGroupHandle) -> Option<&mut ET> {
        return self.groups.get_mut(group.0);
    }
}

impl<E: EntityIdentifier> GroupedEntities<Entities<E>> {
    pub fn retain(
        &mut self,
        world: &mut World,
        group_handles: &[EntityGroupHandle],
        keep: impl Fn(&mut E, &mut World) -> bool,
    ) {
        for group in group_handles {
            if let Some(group) = self.groups.get_mut(group.0) {
                group.entities.retain(|_, entity| {
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

    pub fn index(&self, group: &EntityGroupHandle, index: usize) -> Option<&E> {
        if let Some(group) = self.groups.get(group.0) {
            return group.entities.get_unknown_gen(index);
        }
        None
    }

    pub fn index_mut(&mut self, group: &EntityGroupHandle, index: usize) -> Option<&mut E> {
        if let Some(group) = self.groups.get_mut(group.0) {
            return group.entities.get_unknown_gen_mut(index);
        }
        None
    }

    pub fn get(&self, handle: &EntityHandle) -> Option<&E> {
        if let Some(group) = self.groups.get(handle.group_handle().0) {
            return group.entities.get(handle.entity_index.0);
        }
        None
    }

    pub fn get_mut(&mut self, handle: &EntityHandle) -> Option<&mut E> {
        if let Some(group) = self.groups.get_mut(handle.group_handle().0) {
            return group.entities.get_mut(handle.entity_index.0);
        }
        None
    }

    pub fn get2_mut(
        &mut self,
        handle1: &EntityHandle,
        handle2: &EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        let mut e1 = None;
        let mut e2 = None;
        if handle1.group_handle() == handle2.group_handle() {
            if let Some(group) = self.groups.get_mut(handle1.group_handle().0) {
                (e1, e2) = group
                    .entities
                    .get2_mut(handle1.entity_index.0, handle2.entity_index.0);
            }
        } else {
            let (group1, group2) = self
                .groups
                .get2_mut(handle1.group_handle().0, handle2.group_handle().0);
            if let Some(group) = group1 {
                e1 = group.entities.get_mut(handle1.entity_index.0);
            }

            if let Some(group) = group2 {
                e2 = group.entities.get_mut(handle2.entity_index.0);
            }
        }
        (e1, e2)
    }

    pub fn remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<E> {
        if let Some(group) = self.groups.get_mut(handle.group_handle().0) {
            if let Some(mut entity) = group.entities.remove(handle.entity_index.0) {
                entity.finish(world);
                return Some(entity);
            }
        }
        None
    }

    pub fn remove_all(&mut self, world: &mut World, group_handles: &[EntityGroupHandle]) -> Vec<E> {
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
        group_handle: &EntityGroupHandle,
        mut new: E,
    ) -> EntityHandle {
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, *group_handle);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        group_handle: &EntityGroupHandle,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.entities.insert_with(|idx| {
            handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, *group_handle);
            let mut new = create(handle);
            new.init(handle, world);
            new
        });
        handle
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        group_handle: &EntityGroupHandle,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        let entities = entities.into_iter();
        let mut handles = Vec::with_capacity(entities.size_hint().0);
        if let Some(group) = self.groups.get_mut(group_handle.0) {
            for mut entity in entities {
                group.entities.insert_with(|idx| {
                    let handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, *group_handle);
                    entity.init(handle, world);
                    handles.push(handle);
                    entity
                });
            }
        }
        handles
    }

    pub fn len(&self, group_handles: &[EntityGroupHandle]) -> usize {
        let mut len = 0;
        for group in group_handles {
            if let Some(group) = self.groups.get(group.0) {
                len += group.entities.len();
            }
        }
        len
    }

    pub fn is_empty(&self, group_handles: &[EntityGroupHandle]) -> bool {
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
        entity: &EntityHandle,
        new_group_handle: &EntityGroupHandle,
    ) -> Option<EntityHandle> {
        assert!(entity.entity_type_id() == E::IDENTIFIER);
        let (old_group, new_group) = self
            .groups
            .get2_mut(entity.group_handle().0, new_group_handle.0);
        let old_group = old_group?;
        let new_group = new_group?;
        let entity = old_group.entities.remove(entity.entity_index.0)?;
        let entity_index = EntityIndex(new_group.entities.insert(entity));

        Some(EntityHandle::new(
            entity_index,
            E::IDENTIFIER,
            *new_group_handle,
        ))
    }
}

impl<ET: EntityType + Default> EntityType for GroupedEntities<ET> {
    type Entity = ET::Entity;

    fn add_group(&mut self) {
        self.groups.insert(Default::default());
    }

    fn remove_group(
        &mut self,
        world: &mut World,
        handle: &EntityGroupHandle,
    ) -> Option<Box<dyn EntityType>> {
        if let Some(mut group) = self.groups.remove(handle.0) {
            for (_, entity) in group.dyn_iter_mut() {
                entity.finish(world)
            }
            return Some(Box::new(group));
        }
        None
    }

    fn entity_type_id(&self) -> ConstTypeId {
        ET::Entity::IDENTIFIER
    }

    fn iter_render<'a>(
        &'a self,
        groups: &'a EntityGroupManager,
    ) -> Box<dyn Iterator<Item = &dyn Entity> + 'a>
    where
        Self: Sized,
    {
        Box::new(groups
            .render_groups()
            .iter()
            .flat_map(|g| self.get_group(g).unwrap().iter_render(groups)))
    }

    fn dyn_iter<'a>(&'a self) -> Box<dyn Iterator<Item = (EntityHandle, &dyn Entity)> + 'a> {
        Box::new(self.groups.iter().flat_map(|g| g.dyn_iter()))
    }

    fn dyn_iter_mut<'a>(
        &'a mut self,
    ) -> Box<dyn Iterator<Item = (EntityHandle, &mut dyn Entity)> + 'a> {
        Box::new(self.groups.iter_mut().flat_map(|g| g.dyn_iter_mut()))
    }

    fn dyn_get(&self, handle: &EntityHandle) -> Option<&dyn Entity> {
        self.groups
            .get(handle.group_handle.0)
            .and_then(|e| e.dyn_get(handle))
    }

    fn dyn_get_mut(&mut self, handle: &EntityHandle) -> Option<&mut dyn Entity> {
        self.groups
            .get_mut(handle.group_handle.0)
            .and_then(|e| e.dyn_get_mut(handle))
    }

    fn dyn_retain(
        &mut self,
        world: &mut World,
        keep: &dyn Fn(&mut dyn Entity, &mut World) -> bool,
    ) {
        for group in &mut self.groups {
            group.dyn_retain(world, keep);
        }
    }

    fn dyn_remove(&mut self, world: &mut World, handle: &EntityHandle) -> Option<Box<dyn Entity>> {
        self.groups
            .get_mut(handle.group_handle.0)
            .and_then(|e| e.dyn_remove(world, handle))
    }

    fn len(&self) -> usize {
        self.groups.iter().map(|g| g.len()).sum()
    }
}
