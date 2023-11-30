use std::fmt::{Display, Formatter, Result};

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

use crate::{
    Arena, ComponentBufferManager, Entity, EntityConfig, EntityHandle, EntityIndex, EntitySet,
    EntityStorage, EntityTypeImplementation, GroupHandle, World,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityTypeId {
    id: u32,
}

impl EntityTypeId {
    pub const INVALID: Self = Self { id: 0 };
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Display for EntityTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.id)
    }
}

impl std::hash::Hash for EntityTypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

pub trait EntityIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: EntityTypeId;
    fn entity_type_id(&self) -> EntityTypeId;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum EntityTypeStorage<E: Entity> {
    Single(Option<E>),
    Multiple(EntityTypeGroup<E>),
    MultipleGroups(Arena<EntityTypeGroup<E>>),
}

impl<E: Entity> Clone for EntityTypeStorage<E> {
    fn clone(&self) -> Self {
        match self {
            Self::Single(_) => Self::Single(None),
            Self::Multiple(a) => Self::Multiple(a.clone()),
            Self::MultipleGroups(a) => Self::MultipleGroups(a.clone()),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub(crate) struct EntityTypeGroup<E: Entity> {
    pub entities: Arena<E>,
}

impl<E: Entity> Clone for EntityTypeGroup<E> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<E: Entity> EntityTypeGroup<E> {
    pub fn new() -> Self {
        Self {
            entities: Default::default(),
        }
    }
}


#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub(crate) struct EntityType<E: Entity> {
    config: EntityConfig,
    pub(crate) storage: EntityTypeStorage<E>,
}

#[cfg_attr(not(feature = "physics"), allow(unused_mut))]
impl<E: Entity> EntityType<E> {
    pub(crate) fn new(config: EntityConfig) -> Self {
        let storage = match config.storage {
            EntityStorage::Single => EntityTypeStorage::Single(None),
            EntityStorage::Multiple => EntityTypeStorage::Multiple(EntityTypeGroup::new()),
            EntityStorage::Groups => EntityTypeStorage::MultipleGroups(Arena::new()),
        };
        Self { storage, config }
    }

    pub fn entity_type_id(&self) -> EntityTypeId {
        E::IDENTIFIER
    }

    pub fn for_each(&self, group_handles: &[GroupHandle], mut each: impl FnMut(&E)) {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for entity in &multiple.entities {
                    each(entity);
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        for entity in &group.entities {
                            each(entity);
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_mut(&mut self, group_handles: &[GroupHandle], mut each: impl FnMut(&mut E)) {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for entity in &mut multiple.entities {
                    each(entity);
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        for entity in &mut group.entities {
                            each(entity);
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_with_handles(
        &self,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(EntityHandle, &E),
    ) {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    );
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for (idx, entity) in multiple.entities.iter_with_index() {
                    each(
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        entity,
                    );
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get(group_handle.0) {
                        for (idx, entity) in group.entities.iter_with_index() {
                            each(
                                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, *group_handle),
                                entity,
                            );
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_mut_with_handles(
        &mut self,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(EntityHandle, &mut E),
    ) {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    );
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for (idx, entity) in multiple.entities.iter_mut_with_index() {
                    each(
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        entity,
                    );
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        for (idx, entity) in group.entities.iter_mut_with_index() {
                            each(
                                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, *group_handle),
                                entity,
                            );
                        }
                    }
                }
            }
        };
    }

    pub fn retain(
        &mut self,
        world: &mut World,
        group_handles: &[GroupHandle],
        mut keep: impl FnMut(&mut E, &mut World) -> bool,
    ) {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(e) = entity {
                    let e = e;
                    e.finish(world);
                    if !keep(e, world) {
                        *entity = None;
                    }
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                multiple.entities.retain(|_, entity| {
                    if keep(entity, world) {
                        true
                    } else {
                        entity.finish(world);
                        false
                    }
                });
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
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
        };
    }

    pub fn index(&self, group: GroupHandle, index: usize) -> Option<&E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                return entity.as_ref();
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.get_unknown_gen(index);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(group.0) {
                    return group.entities.get_unknown_gen(index);
                }
                None
            }
        }
    }

    pub fn index_mut(&mut self, group: GroupHandle, index: usize) -> Option<&mut E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if index == 0 {
                    return entity.as_mut();
                }
                None
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.get_unknown_gen_mut(index);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(group.0) {
                    return group.entities.get_unknown_gen_mut(index);
                }
                None
            }
        }
    }

    pub fn get(&self, handle: EntityHandle) -> Option<&E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => return entity.as_ref(),
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.get(handle.entity_index().0);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(handle.group_handle().0) {
                    return group.entities.get(handle.entity_index().0);
                }
                None
            }
        }
    }

    pub fn get_mut(&mut self, handle: EntityHandle) -> Option<&mut E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                return entity.as_mut();
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.get_mut(handle.entity_index().0);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    return group.entities.get_mut(handle.entity_index().0);
                }
                None
            }
        }
    }

    pub fn get2_mut(
        &mut self,
        handle1: EntityHandle,
        handle2: EntityHandle,
    ) -> (Option<&mut E>, Option<&mut E>) {
        match &mut self.storage {
            EntityTypeStorage::Single { .. } => {
                panic!("Cannot get 2 on entity with EntityStorage::Single!");
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple
                    .entities
                    .get2_mut(handle1.entity_index().0, handle2.entity_index().0);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut e1 = None;
                let mut e2 = None;
                if handle1.group_handle() == handle2.group_handle() {
                    if let Some(group) = groups.get_mut(handle1.group_handle().0) {
                        (e1, e2) = group
                            .entities
                            .get2_mut(handle1.entity_index().0, handle2.entity_index().0);
                    }
                } else {
                    let (group1, group2) =
                        groups.get2_mut(handle1.group_handle().0, handle2.group_handle().0);
                    if let Some(group) = group1 {
                        e1 = group.entities.get_mut(handle1.entity_index().0);
                    }

                    if let Some(group) = group2 {
                        e2 = group.entities.get_mut(handle2.entity_index().0);
                    }
                }
                (e1, e2)
            }
        }
    }

    pub fn remove(&mut self, world: &mut World, handle: EntityHandle) -> Option<E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(mut entity) = entity.take() {
                    entity.finish(world);
                    return Some(entity);
                }
                None
            }
            EntityTypeStorage::Multiple(multiple) => {
                if let Some(mut entity) = multiple.entities.remove(handle.entity_index().0) {
                    entity.finish(world);
                    return Some(entity);
                }
                None
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(mut entity) = group.entities.remove(handle.entity_index().0) {
                        entity.finish(world);
                        return Some(entity);
                    }
                }
                None
            }
        }
    }

    pub fn remove_all(&mut self, world: &mut World, group_handles: &[GroupHandle]) -> Vec<E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                let mut result = Vec::with_capacity(1);
                if let Some(mut entity) = entity.take() {
                    entity.finish(world);
                    result.push(entity);
                }
                result
            }
            EntityTypeStorage::Multiple(multiple) => {
                let mut result = Vec::with_capacity(multiple.entities.len());
                let entities = std::mem::take(&mut multiple.entities);
                for mut entity in entities {
                    entity.finish(world);
                    result.push(entity)
                }
                result
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut result = Vec::new();
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        let entities = std::mem::take(&mut group.entities);
                        for mut entity in entities {
                            entity.finish(world);
                            result.push(entity);
                        }
                    }
                }
                result
            }
        }
    }

    pub fn add(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        mut new: E,
    ) -> EntityHandle {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                assert!(entity.is_none(), "Single entity is already set!");
                let handle =
                    EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
                new.init(handle, world);
                *entity = Some(new);
                handle
            }
            EntityTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.entities.insert_with(|idx| {
                    handle =
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
                    new.init(handle, world);
                    new
                });
                handle
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.entities.insert_with(|idx| {
                    handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
                    new.init(handle, world);
                    new
                });
                handle
            }
        }
    }

    pub fn add_with(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                assert!(entity.is_none(), "Single entity is already set!");
                let handle =
                    EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
                let mut new = create(handle);
                new.init(handle, world);
                *entity = Some(new);
                handle
            }
            EntityTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.entities.insert_with(|idx| {
                    handle =
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
                    let mut new = create(handle);
                    new.init(handle, world);
                    new
                });
                handle
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.entities.insert_with(|idx| {
                    handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
                    let mut new = create(handle);
                    new.init(handle, world);
                    new
                });
                handle
            }
        }
    }

    pub fn add_many(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        match &mut self.storage {
            EntityTypeStorage::Single { .. } => {
                panic!("Cannot add naby on entity with EntityStorage::Single!");
            }
            EntityTypeStorage::Multiple(multiple) => {
                let entities = entities.into_iter();
                let mut handles = Vec::with_capacity(entities.size_hint().0);
                for mut entity in entities {
                    multiple.entities.insert_with(|idx| {
                        let handle = EntityHandle::new(
                            EntityIndex(idx),
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        );
                        entity.init(handle, world);
                        handles.push(handle);
                        entity
                    });
                }
                handles
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let entities = entities.into_iter();
                let mut handles = Vec::with_capacity(entities.size_hint().0);
                if let Some(group) = groups.get_mut(group_handle.0) {
                    for mut entity in entities {
                        group.entities.insert_with(|idx| {
                            let handle =
                                EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
                            entity.init(handle, world);
                            handles.push(handle);
                            entity
                        });
                    }
                }
                handles
            }
        }
    }

    pub fn len(&self, group_handles: &[GroupHandle]) -> usize {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if entity.is_some() {
                    1
                } else {
                    0
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                multiple.entities.len()
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut len = 0;
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        len += group.entities.len();
                    }
                }
                len
            }
        }
    }


    pub fn is_empty(&self, group_handles: &[GroupHandle]) -> bool {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                entity.is_some()
            }
            EntityTypeStorage::Multiple(multiple) => {
                multiple.entities.len() > 0
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        if !group.entities.is_empty() {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    pub fn iter<'a>(&'a self, group_handles: &[GroupHandle]) -> EntityIter<'a, E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    Box::new(std::iter::once(entity))
                } else {
                    Box::new(std::iter::empty::<&E>())
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.entities.iter());
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        if !group.entities.is_empty() {
                            iters.push(group.entities.iter());
                        }
                    }
                }
                Box::new(iters.into_iter().flatten())
            }
        }
    }

    pub fn iter_with_handles<'a>(
        &'a self,
        group_handles: &'a [GroupHandle],
    ) -> EntityIterHandles<'a, E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    Box::new(std::iter::once((
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    )))
                } else {
                    Box::new(std::iter::empty::<(EntityHandle, &'a E)>())
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.entities.iter_with_index().map(|(idx, c)| {
                    (
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        c,
                    )
                }));
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group_handle in group_handles {
                    if let Some(group) = groups.get(group_handle.0) {
                        if !group.entities.is_empty() {
                            iters.push(group.entities.iter_with_index().map(|(idx, c)| {
                                (
                                    EntityHandle::new(
                                        EntityIndex(idx),
                                        E::IDENTIFIER,
                                        *group_handle,
                                    ),
                                    c,
                                )
                            }));
                        }
                    }
                }
                Box::new(iters.into_iter().flatten())
            }
        }
    }

    pub fn iter_mut<'a>(&'a mut self, group_handles: &[GroupHandle]) -> EntityIterMut<'a, E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    Box::new(std::iter::once(entity))
                } else {
                    Box::new(std::iter::empty::<&mut E>())
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.entities.iter_mut());
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<EntityTypeGroup<E>> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (*ptr).get_mut(group_handle.0) {
                            iters.push(group.entities.iter_mut());
                        };
                    }
                }

                Box::new(iters.into_iter().flatten())
            }
        }
    }

    pub fn iter_mut_with_handles<'a>(
        &'a mut self,
        group_handles: &'a [GroupHandle],
    ) -> EntityIterHandlesMut<'a, E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    Box::new(std::iter::once((
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    )))
                } else {
                    Box::new(std::iter::empty::<(EntityHandle, &mut E)>())
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.entities.iter_mut_with_index().map(|(idx, c)| {
                    (
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        c,
                    )
                }));
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<EntityTypeGroup<E>> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (*ptr).get_mut(group_handle.0) {
                            let type_id = &E::IDENTIFIER;

                            iters.push(group.entities.iter_mut_with_index().map(
                                move |(idx, c)| {
                                    (
                                        EntityHandle::new(
                                            EntityIndex(idx),
                                            *type_id,
                                            *group_handle,
                                        ),
                                        c,
                                    )
                                },
                            ));
                        };
                    }
                }

                Box::new(iters.into_iter().flatten())
            }
        }
    }

    pub fn change_group(
        &mut self,
        entity: EntityHandle,
        new_group_handle: GroupHandle,
    ) -> Option<EntityHandle> {
        match &mut self.storage {
            EntityTypeStorage::MultipleGroups(groups) => {
                let (old_group, new_group) =
                    groups.get2_mut(entity.group_handle().0, new_group_handle.0);
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
            _ => panic!("Cannot get change group on entity without EntityStorage::Group!"),
        }
    }

    pub fn single(&self) -> &E {
        self.try_single().expect("Singleton not defined!")
    }

    pub fn single_mut(&mut self) -> &mut E {
        self.try_single_mut().expect("Singleton not defined!")
    }

    pub fn try_single(&self) -> Option<&E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                return entity.as_ref();
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }

    pub fn try_single_mut(&mut self) -> Option<&mut E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                return entity.as_mut();
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }

    pub fn remove_single(&mut self, world: &mut World) -> Option<E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(mut entity) = entity.take() {
                    entity.finish(world);
                    return Some(entity);
                }
                None
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }

    pub fn set_single(&mut self, world: &mut World, mut new: E) -> EntityHandle {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                let handle =
                    EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
                new.init(handle, world);
                if let Some(mut _old) = entity.replace(new) {
                    _old.finish(world);
                }
                handle
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }

    pub fn set_single_with(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                let handle =
                    EntityHandle::new(EntityIndex::INVALID, E::IDENTIFIER, GroupHandle::INVALID);
                let mut new = create(handle);
                new.init(handle, world);
                if let Some(mut _old) = entity.replace(new) {
                    _old.finish(world);
                }
                handle
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }
}

impl<E: Entity> EntityTypeImplementation for EntityType<E> {
    fn add_group(&mut self) {
        match &mut self.storage {
            EntityTypeStorage::MultipleGroups(groups) => {
                groups.insert(EntityTypeGroup::new());
            }
            _ => {}
        }
    }

    fn remove_group(&mut self, world: &mut World, handle: GroupHandle) {
        match &mut self.storage {
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.remove(handle.0) {
                    // Checked because of serializing groups
                    for mut entity in group.entities {
                        entity.finish(world)
                    }
                }
            }
            _ => {}
        }
    }

    fn buffer(
        &self,
        ty: std::cell::Ref<dyn EntityTypeImplementation>,
        buffers: &mut ComponentBufferManager,
        world: &World,
        active_groups: &[GroupHandle],
    ) {
        let ty = std::cell::Ref::map(ty, |ty| ty.downcast_ref::<EntityType<E>>().unwrap());
        let set = EntitySet::new(ty, active_groups);
        E::buffer(set, buffers, world);
    }

    #[cfg(all(feature = "serde", feature = "physics"))]
    fn deinit_non_serialized(&self, world: &mut World) {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    for component in entity.components() {
                        world.remove_no_maintain(component)
                    }
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for entity in &multiple.entities {
                    for component in entity.components() {
                        world.remove_no_maintain(component)
                    }
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    for entity in &group.entities {
                        for component in entity.components() {
                            world.remove_no_maintain(component)
                        }
                    }
                }
            }
        }
    }

    fn is_groups(&self) -> bool {
        return match self.storage  {
            EntityTypeStorage::Single(_) => false,
            EntityTypeStorage::Multiple(_) => false,
            EntityTypeStorage::MultipleGroups(_) => true
        }
    }

    #[cfg(feature = "serde")]
    fn remove_group_serialize(
        &mut self,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Box<dyn std::any::Any>> {
        match &mut self.storage {
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(mut group) = groups.remove(handle.0) {
                    for entity in &mut group.entities {
                        entity.finish(world)
                    }
                    return Some(Box::new(group));
                }
            }
            _ => {}
        }
        None
    }

    fn entity_type_id(&self) -> EntityTypeId {
        E::IDENTIFIER
    }

    fn config(&self) -> EntityConfig {
        self.config
    }
}

#[cfg(feature = "rayon")]
impl<E: Entity + Send + Sync> EntityType<E> {
    pub fn par_for_each(&self, group_handles: &[GroupHandle], each: impl Fn(&E) + Send + Sync) {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                multiple.entities.items.par_iter().for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        each(data);
                    }
                })
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        group.entities.items.par_iter().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                each(data);
                            }
                        })
                    }
                }
            }
        };
    }

    pub fn par_for_each_mut(
        &mut self,
        group_handles: &[GroupHandle],
        each: impl Fn(&mut E) + Send + Sync,
    ) {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    each(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => multiple
                .entities
                .items
                .par_iter_mut()
                .for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        each(data);
                    }
                }),
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.entities.items.par_iter_mut().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                each(data);
                            }
                        })
                    }
                }
            }
        };
    }

    pub fn par_for_each_collect<C: crate::Component>(
        &self,
        world: &World,
        group_handles: &[GroupHandle],
        each: impl Fn(&E) -> &C + Send + Sync,
        collection: &mut Vec<C::Instance>,
    ) where
        C::Instance: Send + Sync,
    {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                collection.extend(entity.iter().map(|e| each(e).instance(world)));
            }
            EntityTypeStorage::Multiple(multiple) => {
                collection.par_extend(multiple.entities.items.par_iter().filter_map(|e| match e {
                    ArenaEntry::Free { .. } => None,
                    ArenaEntry::Occupied { data, .. } => {
                        let component = each(data);
                        if component.active() {
                            Some(component.instance(world))
                        } else {
                            None
                        }
                    }
                }));
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        collection.par_extend(group.entities.items.par_iter().filter_map(
                            |e| match e {
                                ArenaEntry::Free { .. } => None,
                                ArenaEntry::Occupied { data, .. } => {
                                    let component = each(data);
                                    if component.active() {
                                        Some(component.instance(world))
                                    } else {
                                        None
                                    }
                                }
                            },
                        ));
                    }
                }
            }
        };
    }

    pub fn par_for_each_collect_mut<C: crate::Component>(
        &mut self,
        world: &World,
        group_handles: &[GroupHandle],
        each: impl Fn(&mut E) -> &C + Send + Sync,
        collection: &mut Vec<C::Instance>,
    ) where
        C::Instance: Send + Sync,
    {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                collection.extend(entity.iter_mut().map(|e| each(e).instance(world)));
            }
            EntityTypeStorage::Multiple(multiple) => {
                collection.par_extend(multiple.entities.items.par_iter_mut().filter_map(
                    |e| match e {
                        ArenaEntry::Free { .. } => None,
                        ArenaEntry::Occupied { data, .. } => {
                            let component = each(data);
                            if component.active() {
                                Some(component.instance(world))
                            } else {
                                None
                            }
                        }
                    },
                ));
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        collection.par_extend(group.entities.items.par_iter_mut().filter_map(
                            |e| match e {
                                ArenaEntry::Free { .. } => None,
                                ArenaEntry::Occupied { data, .. } => {
                                    let component = each(data);
                                    if component.active() {
                                        Some(component.instance(world))
                                    } else {
                                        None
                                    }
                                }
                            },
                        ));
                    }
                }
            }
        };
    }
}

pub trait EntityIterator: DoubleEndedIterator {}
impl<T> EntityIterator for T where T: DoubleEndedIterator {}
pub type EntityIter<'a, E> = Box<dyn EntityIterator<Item = &'a E> + 'a>;
pub type EntityIterHandles<'a, E> = Box<dyn EntityIterator<Item = (EntityHandle, &'a E)> + 'a>;
pub type EntityIterMut<'a, E> = Box<dyn EntityIterator<Item = &'a mut E> + 'a>;
pub type EntityIterHandlesMut<'a, E> =
    Box<dyn EntityIterator<Item = (EntityHandle, &'a mut E)> + 'a>;
