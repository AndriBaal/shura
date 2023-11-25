use std::fmt::{Display, Formatter, Result};

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

use crate::{
    Arena, ComponentBufferManager, Entity, EntityConfig, EntityHandle, EntityIndex, EntityStorage,
    EntityTypeImplementation, GroupHandle, World,
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

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
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

    // fn buffer(&mut self, gpu: &Gpu, config: &EntityConfig, world: &World) {
    //     if config.buffer == BufferConfig::EveryFrame || self.force_buffer {
    //         self.force_buffer = false;
    //         let instances = self
    //             .entities
    //             .iter()
    //             .filter_map(|entity| {
    //                 if entity.component().active() {
    //                     Some(entity.component().instance(world))
    //                 } else {
    //                     None
    //                 }
    //             })
    //             .collect::<Vec<<E::Component as Component>::Instance>>();

    //         if let Some(buffer) = self.buffer.as_mut() {
    //             buffer.write(gpu, &instances);
    //         } else {
    //             self.buffer = Some(gpu.create_instance_buffer(&instances));
    //         }
    //     }
    // }

    // fn buffer_with(
    //     &mut self,
    //     gpu: &Gpu,
    //     config: &EntityConfig,
    //     world: &World,
    //     mut each: impl FnMut(&mut E),
    // ) {
    //     if config.buffer == BufferConfig::EveryFrame || self.force_buffer {
    //         self.force_buffer = false;
    //         let instances = self
    //             .entities
    //             .iter_mut()
    //             .filter_map(|entity| {
    //                 (each)(entity);
    //                 if entity.component().active() {
    //                     Some(entity.component().instance(world))
    //                 } else {
    //                     None
    //                 }
    //             })
    //             .collect::<Vec<<E::Component as Component>::Instance>>();

    //         if let Some(buffer) = self.buffer.as_mut() {
    //             buffer.write(gpu, &instances);
    //         } else {
    //             self.buffer = Some(gpu.create_instance_buffer(&instances));
    //         }
    //     }
    // }
}

// #[cfg(feature = "rayon")]
// impl<E: Entity + Send + Sync> EntityTypeGroup<E>
// where
//     <E::Component as Component>::Instance: Send,
// {
//     fn par_buffer(&mut self, gpu: &Gpu, config: &EntityConfig, world: &World) {
//         if config.buffer == BufferConfig::EveryFrame || self.force_buffer {
//             self.force_buffer = false;
//             let instances = self
//                 .entities
//                 .items
//                 .par_iter_mut()
//                 .filter_map(|entity| match entity {
//                     ArenaEntry::Free { .. } => None,
//                     ArenaEntry::Occupied { data, .. } => {
//                         if data.component().active() {
//                             Some(data.component().instance(world))
//                         } else {
//                             None
//                         }
//                     }
//                 })
//                 .collect::<Vec<<E::Component as Component>::Instance>>();

//             if let Some(buffer) = self.buffer.as_mut() {
//                 buffer.write(gpu, &instances);
//             } else {
//                 self.buffer = Some(gpu.create_instance_buffer(&instances));
//             }
//         }
//     }

//     fn par_buffer_with(
//         &mut self,
//         gpu: &Gpu,
//         config: &EntityConfig,
//         world: &World,
//         each: impl Fn(&mut E) + Send + Sync,
//     ) {
//         if config.buffer == BufferConfig::EveryFrame || self.force_buffer {
//             self.force_buffer = false;
//             let instances = self
//                 .entities
//                 .items
//                 .par_iter_mut()
//                 .filter_map(|entity| match entity {
//                     ArenaEntry::Free { .. } => None,
//                     ArenaEntry::Occupied { data, .. } => {
//                         (each)(data);
//                         if data.component().active() {
//                             Some(data.component().instance(world))
//                         } else {
//                             None
//                         }
//                     }
//                 })
//                 .collect::<Vec<<E::Component as Component>::Instance>>();

//             if let Some(buffer) = self.buffer.as_mut() {
//                 buffer.write(gpu, &instances);
//             } else {
//                 self.buffer = Some(gpu.create_instance_buffer(&instances));
//             }
//         }
//     }
// }

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
                    (each)(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for entity in &multiple.entities {
                    (each)(entity);
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        for entity in &group.entities {
                            (each)(entity);
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
                    (each)(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                for entity in &mut multiple.entities {
                    (each)(entity);
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        for entity in &mut group.entities {
                            (each)(entity);
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
                    (each)(
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
                    (each)(
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        entity,
                    );
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get(group_handle.0) {
                        for (idx, entity) in group.entities.iter_with_index() {
                            (each)(
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
                    (each)(
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
                    (each)(
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID),
                        entity,
                    );
                }
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        for (idx, entity) in group.entities.iter_mut_with_index() {
                            (each)(
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
                    let entity = entity;
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
                return None;
            }
        };
    }

    pub fn index_mut(&mut self, group: GroupHandle, index: usize) -> Option<&mut E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if index == 0 {
                    return entity.as_mut();
                }
                return None;
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.get_unknown_gen_mut(index);
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(group.0) {
                    return group.entities.get_unknown_gen_mut(index);
                }
                return None;
            }
        };
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
                return None;
            }
        };
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
                return None;
            }
        };
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
                return (e1, e2);
            }
        };
    }

    pub fn remove(&mut self, world: &mut World, handle: EntityHandle) -> Option<E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(mut entity) = entity.take() {
                    entity.finish(world);
                    return Some(entity);
                }
                return None;
            }
            EntityTypeStorage::Multiple(multiple) => {
                if let Some(mut entity) = multiple.entities.remove(handle.entity_index().0) {
                    entity.finish(world);
                    return Some(entity);
                }
                return None;
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(mut entity) = group.entities.remove(handle.entity_index().0) {
                        entity.finish(world);
                        return Some(entity);
                    }
                }
                return None;
            }
        };
    }

    pub fn remove_all(&mut self, world: &mut World, group_handles: &[GroupHandle]) -> Vec<E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                let mut result = Vec::with_capacity(1);
                if let Some(mut entity) = entity.take() {
                    entity.finish(world);
                    result.push(entity);
                }
                return result;
            }
            EntityTypeStorage::Multiple(multiple) => {
                let mut result = Vec::with_capacity(multiple.entities.len());
                let entities = std::mem::replace(&mut multiple.entities, Default::default());
                for mut entity in entities {
                    entity.finish(world);
                    result.push(entity)
                }
                return result;
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut result = Vec::new();
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        let entities = std::mem::replace(&mut group.entities, Default::default());
                        for mut entity in entities {
                            entity.finish(world);
                            result.push(entity);
                        }
                    }
                }
                return result;
            }
        };
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
                return handle;
            }
            EntityTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.entities.insert_with(|idx| {
                    handle =
                        EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, GroupHandle::INVALID);
                    new.init(handle, world);
                    new
                });
                return handle;
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.entities.insert_with(|idx| {
                    handle = EntityHandle::new(EntityIndex(idx), E::IDENTIFIER, group_handle);
                    new.init(handle, world);
                    new
                });
                return handle;
            }
        };
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
                return handle;
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
                return handle;
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
                return handle;
            }
        };
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
                return handles;
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
                return handles;
            }
        };
    }

    // pub fn force_buffer(&mut self, group_handles: &[GroupHandle]) {
    //     match &mut self.storage {
    //         EntityTypeStorage::Single { force_buffer, .. } => {
    //         }
    //         EntityTypeStorage::Multiple(multiple) => {
    //         }
    //         EntityTypeStorage::MultipleGroups(groups) => {
    //             for group in group_handles {
    //                 if let Some(group) = groups.get_mut(group.0) {
    //                 }
    //             }
    //         }
    //     };
    // }

    pub fn len(&self, group_handles: &[GroupHandle]) -> usize {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if entity.is_some() {
                    return 1;
                } else {
                    return 0;
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                return multiple.entities.len();
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                let mut len = 0;
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        len += group.entities.len();
                    }
                }
                return len;
            }
        };
    }

    pub fn iter<'a>(
        &'a self,
        group_handles: &[GroupHandle],
    ) -> EntityIter<'a, E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    return Box::new(std::iter::once(entity));
                } else {
                    return Box::new(std::iter::empty::<&E>());
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
                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_with_handles<'a>(
        &'a self,
        group_handles: &'a [GroupHandle],
    ) -> EntityIterHandles<'a, E> {
        match &self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    return Box::new(std::iter::once((
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    )));
                } else {
                    return Box::new(std::iter::empty::<(EntityHandle, &'a E)>());
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
                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_mut<'a>(
        &'a mut self,
        group_handles: &[GroupHandle],
    ) -> EntityIterMut<'a, E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    return Box::new(std::iter::once(entity));
                } else {
                    return Box::new(std::iter::empty::<&mut E>());
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
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                            iters.push(group.entities.iter_mut());
                        };
                    }
                }

                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_mut_with_handles<'a>(
        &'a mut self,
        group_handles: &'a [GroupHandle],
    ) -> EntityIterHandlesMut<'a, E> {
        match &mut self.storage {
            EntityTypeStorage::Single(entity) => {
                if let Some(entity) = entity {
                    return Box::new(std::iter::once((
                        EntityHandle::new(
                            EntityIndex::INVALID,
                            E::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        entity,
                    )));
                } else {
                    return Box::new(std::iter::empty::<(EntityHandle, &mut E)>());
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
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
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

                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    // pub fn iter_render<'a>(
    //     &'a self,
    //     group_handles: &[GroupHandle],
    // ) -> Box<
    //     dyn DoubleEndedIterator<
    //             Item = (
    //                 &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //                 InstanceIndex,
    //                 &'a E,
    //             ),
    //         > + 'a,
    // > {
    //     match &self.storage {
    //         EntityTypeStorage::Single { entity, buffer, .. } => {
    //             if let Some(entity) = entity {
    //                 return Box::new(std::iter::once((
    //                     buffer.as_ref().expect(BUFFER_ERROR),
    //                     InstanceIndex::new(0),
    //                     entity,
    //                 )));
    //             } else {
    //                 return Box::new(std::iter::empty::<(
    //                     &InstanceBuffer<<E::Component as Component>::Instance>,
    //                     InstanceIndex,
    //                     &E,
    //                 )>());
    //             }
    //         }
    //         EntityTypeStorage::Multiple(multiple) => {
    //             return Box::new(multiple.entities.iter().enumerate().map(|(i, c)| {
    //                 (
    //                     multiple.buffer.as_ref().expect(BUFFER_ERROR),
    //                     InstanceIndex::new(i as u32),
    //                     c,
    //                 )
    //             }));
    //         }
    //         EntityTypeStorage::MultipleGroups(groups) => {
    //             let mut iters = Vec::with_capacity(groups.len());
    //             for group in group_handles {
    //                 if let Some(group) = groups.get(group.0) {
    //                     if !group.entities.is_empty() {
    //                         iters.push(group.entities.iter().enumerate().map(|(i, c)| {
    //                             (
    //                                 group.buffer.as_ref().expect(BUFFER_ERROR),
    //                                 InstanceIndex::new(i as u32),
    //                                 c,
    //                             )
    //                         }));
    //                     }
    //                 }
    //             }
    //             return Box::new(iters.into_iter().flatten());
    //         }
    //     };
    // }

    // pub(crate) fn render_each<'a>(
    //     &'a self,
    //     renderer: &mut Renderer<'a>,
    //     mut each: impl FnMut(
    //         &mut Renderer<'a>,
    //         &'a E,
    //         &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //         InstanceIndex,
    //     ),
    // ) {
    //     match &self.storage {
    //         EntityTypeStorage::Single { buffer, entity, .. } => {
    //             if let Some(entity) = entity {
    //                 let buffer = buffer.as_ref().expect(BUFFER_ERROR);
    //                 if buffer.instance_amount() > 0 {
    //                     (each)(renderer, entity, buffer, InstanceIndex::new(0));
    //                 }
    //             }
    //         }
    //         EntityTypeStorage::Multiple(multiple) => {
    //             let buffer = multiple.buffer.as_ref().expect(BUFFER_ERROR);
    //             if buffer.instance_amount() > 0 {
    //                 for (instance, entity) in multiple.entities.iter().enumerate() {
    //                     (each)(
    //                         renderer,
    //                         entity,
    //                         buffer,
    //                         InstanceIndex::new(instance as u32),
    //                     );
    //                 }
    //             }
    //         }
    //         EntityTypeStorage::MultipleGroups(groups) => {
    //             for group in groups {
    //                 let buffer = group.buffer.as_ref().expect(BUFFER_ERROR);
    //                 if buffer.instance_amount() > 0 {
    //                     for (instance, entity) in group.entities.iter().enumerate() {
    //                         (each)(
    //                             renderer,
    //                             entity,
    //                             buffer,
    //                             InstanceIndex::new(instance as u32),
    //                         );
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // pub(crate) fn render_single<'a>(
    //     &'a self,
    //     renderer: &mut Renderer<'a>,
    //     each: impl FnOnce(
    //         &mut Renderer<'a>,
    //         &'a E,
    //         &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //         InstanceIndex,
    //     ),
    // ) {
    //     match &self.storage {
    //         EntityTypeStorage::Single { buffer, entity, .. } => {
    //             if let Some(entity) = entity {
    //                 let buffer = buffer.as_ref().expect(BUFFER_ERROR);
    //                 if buffer.instance_amount() > 0 {
    //                     (each)(renderer, entity, buffer, InstanceIndex::new(0));
    //                 }
    //             }
    //         }
    //         _ => {
    //             panic!("Cannot get single on entity without EntityStorage::Single!")
    //         }
    //     }
    // }

    // pub(crate) fn render_all<'a>(
    //     &'a self,
    //     renderer: &mut Renderer<'a>,
    //     mut all: impl FnMut(
    //         &mut Renderer<'a>,
    //         &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //         InstanceIndices,
    //     ),
    // ) {
    //     match &self.storage {
    //         EntityTypeStorage::Single { buffer, .. } => {
    //             let buffer = buffer.as_ref().expect(BUFFER_ERROR);
    //             if buffer.instance_amount() > 0 {
    //                 (all)(renderer, buffer, InstanceIndices::new(0, 1));
    //             }
    //         }
    //         EntityTypeStorage::Multiple(multiple) => {
    //             let buffer = multiple.buffer.as_ref().expect(BUFFER_ERROR);
    //             if buffer.instance_amount() > 0 {
    //                 (all)(renderer, buffer, buffer.instances());
    //             }
    //         }
    //         EntityTypeStorage::MultipleGroups(groups) => {
    //             for group in groups {
    //                 let buffer = group.buffer.as_ref().expect(BUFFER_ERROR);
    //                 if buffer.instance_amount() > 0 {
    //                     (all)(renderer, buffer, buffer.instances());
    //                 }
    //             }
    //         }
    //     }
    // }

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

                return Some(EntityHandle::new(
                    entity_index,
                    E::IDENTIFIER,
                    new_group_handle,
                ));
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
                return None;
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
                return handle;
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
                return handle;
            }
            _ => panic!("Cannot get single on entity without EntityStorage::Single!"),
        }
    }

    // pub fn buffer_with(
    //     &mut self,
    //     world: &World,
    //     gpu: &Gpu,
    //     group_handles: &[GroupHandle],
    //     mut each: impl FnMut(&mut E),
    // ) {
    //     assert!(self.config.buffer != BufferConfig::Never);
    //     match &mut self.storage {
    //         EntityTypeStorage::Single {
    //             entity,
    //             buffer,
    //             force_buffer,
    //         } => {
    //             if self.config.buffer == BufferConfig::EveryFrame || *force_buffer {
    //                 *force_buffer = false;
    //                 let instance = {
    //                     if let Some(entity) = entity {
    //                         (each)(entity);
    //                         if entity.component().active() {
    //                             Some(entity.component().instance(world))
    //                         } else {
    //                             None
    //                         }
    //                     } else {
    //                         None
    //                     }
    //                 };

    //                 if let Some(buffer) = buffer.as_mut() {
    //                     buffer.write(
    //                         gpu,
    //                         instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
    //                     );
    //                 } else {
    //                     *buffer = Some(gpu.create_instance_buffer(
    //                         instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
    //                     ));
    //                 }
    //             }
    //         }
    //         EntityTypeStorage::Multiple(multiple) => {
    //             multiple.buffer_with(gpu, &self.config, world, each)
    //         }
    //         EntityTypeStorage::MultipleGroups(groups) => {
    //             for group in group_handles {
    //                 if let Some(group) = groups.get_mut(group.0) {
    //                     group.buffer_with(gpu, &self.config, world, &mut each)
    //                 }
    //             }
    //         }
    //     };
    // }
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
        buffers: &mut ComponentBufferManager,
        world: &World,
        active_groups: &[GroupHandle],
    ) {
        let iter = self.iter(active_groups);
        E::buffer(iter, buffers, world);
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
        return None;
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
                    (each)(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => {
                multiple.entities.items.par_iter().for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data);
                    }
                })
            }
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        group.entities.items.par_iter().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                (each)(data);
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
                    (each)(entity);
                }
            }
            EntityTypeStorage::Multiple(multiple) => multiple
                .entities
                .items
                .par_iter_mut()
                .for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data);
                    }
                }),
            EntityTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.entities.items.par_iter_mut().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                (each)(data);
                            }
                        })
                    }
                }
            }
        };
    }
}

pub type EntityIter<'a, E> = Box<dyn DoubleEndedIterator<Item = &'a E> + 'a>;
pub type EntityIterHandles<'a, E> = Box<dyn DoubleEndedIterator<Item = (EntityHandle, &'a E)> + 'a>;
pub type EntityIterMut<'a, E> = Box<dyn DoubleEndedIterator<Item = &'a mut E> + 'a>;
pub type EntityIterHandlesMut<'a, E> = Box<dyn DoubleEndedIterator<Item = (EntityHandle, &'a mut E)> + 'a>;




// #[cfg(feature = "rayon")]
// impl<E: Entity + Send + Sync> EntityType<E>
// where
//     <E::Component as Component>::Instance: Send,
// {
//     pub fn par_buffer_with(
//         &mut self,
//         world: &World,
//         gpu: &Gpu,
//         group_handles: &[GroupHandle],
//         each: impl Fn(&mut E) + Send + Sync,
//     ) {
//         assert!(self.config.buffer != BufferConfig::Never);
//         match &mut self.storage {
//             EntityTypeStorage::Single { .. } => self.buffer_with(world, gpu, group_handles, each),
//             EntityTypeStorage::Multiple(multiple) => {
//                 multiple.par_buffer_with(gpu, &self.config, world, each)
//             }
//             EntityTypeStorage::MultipleGroups(groups) => {
//                 for group in group_handles {
//                     if let Some(group) = groups.get_mut(group.0) {
//                         group.par_buffer_with(gpu, &self.config, world, &each)
//                     }
//                 }
//             }
//         };
//     }

//     pub fn par_buffer(&mut self, world: &World, gpu: &Gpu, group_handles: &[GroupHandle]) {
//         assert!(self.config.buffer != BufferConfig::Never);
//         match &mut self.storage {
//             EntityTypeStorage::Single { .. } => self.buffer(world, gpu, group_handles),
//             EntityTypeStorage::Multiple(multiple) => multiple.par_buffer(gpu, &self.config, world),
//             EntityTypeStorage::MultipleGroups(groups) => {
//                 for group in group_handles {
//                     if let Some(group) = groups.get_mut(group.0) {
//                         group.par_buffer(gpu, &self.config, world)
//                     }
//                 }
//             }
//         };
//     }
// }
