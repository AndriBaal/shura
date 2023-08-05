use std::fmt::{Display, Formatter, Result};

#[cfg(feature = "physics")]
use crate::physics::World;

#[cfg(feature = "rayon")]
use crate::rayon::prelude::*;

use crate::{
    data::arena::ArenaEntry, Arena, BoxedComponent, BufferCallback, BufferHelper, BufferHelperType,
    BufferOperation, ComponentBuffer, ComponentConfig, ComponentController, ComponentDerive,
    ComponentHandle, ComponentIndex, ComponentStorage, Gpu, GroupHandle, GroupManager,
    InstanceBuffer, InstanceIndex, InstanceIndices, RenderCamera, Renderer,
};

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// TypeId of a struct that derives from the [component](crate::Component) macro. The diffrence to the [std::any::TypeId] is, that
/// this TypeId is const and is the same on every system.
///
/// # How it works
/// It works by providing a unique identifier to the derive macro. This unique identifier can be passed
/// with the `name` attribute, otherwise it is just the struct name. Then this identifier is hashed to a unique
/// u32. The macro is checking at compile time, that every [ComponentTypeId] is unique.
pub struct ComponentTypeId {
    id: u32,
}

impl ComponentTypeId {
    pub const INVALID: Self = Self { id: 0 };
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Display for ComponentTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.id)
    }
}

/// Trait to identify a struct that derives from  the [Component](crate::Component) macro using
/// a [ComponentTypeId]
pub trait ComponentIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ComponentTypeId;
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum ComponentTypeStorage {
    Single {
        #[cfg_attr(feature = "serde", serde(skip))]
        #[cfg_attr(feature = "serde", serde(default))]
        buffer: Option<InstanceBuffer>,
        #[cfg_attr(feature = "serde", serde(skip))]
        #[cfg_attr(feature = "serde", serde(default = "default_true"))]
        force_buffer: bool,
        #[cfg_attr(feature = "serde", serde(skip))]
        #[cfg_attr(feature = "serde", serde(default))]
        component: Option<BoxedComponent>,
    },
    Multiple(ComponentTypeGroup),
    MultipleGroups(Arena<ComponentTypeGroup>),
}

impl Clone for ComponentTypeStorage {
    fn clone(&self) -> Self {
        match self {
            Self::Single { force_buffer, .. } => Self::Single {
                force_buffer: force_buffer.clone(),
                component: None,
                buffer: None,
            },
            Self::Multiple(a) => Self::Multiple(a.clone()),
            Self::MultipleGroups(a) => Self::MultipleGroups(a.clone()),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentTypeGroup {
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub components: Arena<BoxedComponent>,
    force_buffer: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    buffer: Option<InstanceBuffer>,
    // TODO: Maybe change this to a flag, since now you have to manually force_buffer to cover all cases
    last_len: u64,
}

impl Clone for ComponentTypeGroup {
    fn clone(&self) -> Self {
        Self {
            buffer: None,
            components: Default::default(),
            force_buffer: self.force_buffer.clone(),
            last_len: self.last_len.clone(),
        }
    }
}

impl ComponentTypeGroup {
    pub fn new() -> Self {
        Self {
            components: Arena::new(),
            buffer: None,
            last_len: 0,
            force_buffer: true,
        }
    }

    fn resize_buffer(&mut self, gpu: &Gpu, instance_size: u64) {
        let new_len = self.components.len() as u64;
        let instance_capacity = self
            .buffer
            .as_ref()
            .map(|b| b.instance_capacity())
            .unwrap_or(0);
        if new_len > instance_capacity {
            self.buffer = Some(InstanceBuffer::empty(gpu, instance_size, new_len));
        }
    }

    fn buffer(
        &mut self,
        gpu: &Gpu,
        config: &ComponentConfig,
        instance_size: u64,
        callback: BufferCallback,
        #[cfg(feature = "physics")] world: &World,
    ) {
        let len = self.components.len() as u64;
        self.resize_buffer(gpu, instance_size);
        if config.buffer == BufferOperation::EveryFrame || self.force_buffer || len != self.last_len
        {
            self.last_len = len;
            self.force_buffer = false;
            let buffer = self.buffer.as_mut().unwrap();
            callback(
                gpu,
                BufferHelper::new(
                    #[cfg(feature = "physics")]
                    world,
                    buffer,
                    BufferHelperType::All {
                        components: &mut self.components,
                    },
                ),
            )
        }
    }
}

const BUFFER_ERROR: &'static str =
    "This component either has no buffer or it has not been initialized yet!";

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub(crate) struct ComponentType {
    type_id: ComponentTypeId,
    config: ComponentConfig,
    instance_size: u64,
    pub storage: ComponentTypeStorage,
}

#[cfg_attr(not(feature = "physics"), allow(unused_mut))]
impl ComponentType {
    pub(crate) fn with_config<C: ComponentController + ComponentBuffer>(
        config: ComponentConfig,
        group_structure: &GroupManager,
    ) -> Self {
        let storage = match config.storage {
            ComponentStorage::Single => ComponentTypeStorage::Single {
                buffer: None,
                force_buffer: true,
                component: None,
            },
            ComponentStorage::Multiple => ComponentTypeStorage::Multiple(ComponentTypeGroup::new()),
            ComponentStorage::Groups => ComponentTypeStorage::MultipleGroups(Arena {
                items: group_structure
                    .groups
                    .items
                    .iter()
                    .map(|entry| match *entry {
                        ArenaEntry::Free { next_free } => ArenaEntry::Free { next_free },
                        ArenaEntry::Occupied { generation, .. } => ArenaEntry::Occupied {
                            generation,
                            data: ComponentTypeGroup::new(),
                        },
                    })
                    .collect(),
                generation: group_structure.groups.generation,
                free_list_head: group_structure.groups.free_list_head,
                len: group_structure.groups.len(),
            }),
        };
        Self {
            storage,
            config,
            instance_size: C::INSTANCE_SIZE,
            type_id: C::IDENTIFIER,
        }
    }

    pub fn component_type_id(&self) -> ComponentTypeId {
        self.type_id
    }

    pub(crate) fn buffer(
        &mut self,
        #[cfg(feature = "physics")] world: &World,
        callback: BufferCallback,
        active: &[GroupHandle],
        gpu: &Gpu,
    ) {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                for index in active {
                    let group = &mut groups[index.0];
                    group.buffer(
                        gpu,
                        &self.config,
                        self.instance_size,
                        callback,
                        #[cfg(feature = "physics")]
                        world,
                    )
                }
            }
            ComponentTypeStorage::Multiple(multiple) => multiple.buffer(
                gpu,
                &self.config,
                self.instance_size,
                callback,
                #[cfg(feature = "physics")]
                world,
            ),
            ComponentTypeStorage::Single {
                buffer,
                force_buffer,
                component,
            } => {
                if let Some(component) = component {
                    if self.config.buffer == BufferOperation::EveryFrame || *force_buffer {
                        if buffer.is_none() {
                            *buffer = Some(InstanceBuffer::empty(gpu, self.instance_size, 1));
                        }
                        *force_buffer = false;
                        let buffer = buffer.as_mut().unwrap();
                        callback(
                            gpu,
                            BufferHelper::new(
                                #[cfg(feature = "physics")]
                                world,
                                buffer,
                                crate::BufferHelperType::Single {
                                    offset: 0,
                                    component,
                                },
                            ),
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn add_group(&mut self) {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                groups.insert(ComponentTypeGroup::new());
            }
            _ => {}
        }
    }

    pub(crate) fn remove_group(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: GroupHandle,
    ) {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                let _group = groups.remove(handle.0).unwrap();
                #[cfg(feature = "physics")]
                for mut component in _group.components {
                    world.remove(&mut component)
                }
            }
            _ => {}
        }
    }

    pub fn par_for_each<C: ComponentController>(
        &self,
        group_handles: &[GroupHandle],
        each: impl Fn(&C) + Send + Sync,
    ) {
        #[cfg(feature = "rayon")]
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component.downcast_ref::<C>().unwrap());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.components.items.par_iter().for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data.downcast_ref::<C>().unwrap());
                    }
                })
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        group.components.items.par_iter().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                (each)(data.downcast_ref::<C>().unwrap());
                            }
                        })
                    }
                }
            }
        };
        #[cfg(not(feature = "rayon"))]
        self.for_each(group_handles, each)
    }

    pub fn par_for_each_mut<C: ComponentController>(
        &mut self,
        group_handles: &[GroupHandle],
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        #[cfg(feature = "rayon")]
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component.downcast_mut::<C>().unwrap());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => multiple
                .components
                .items
                .par_iter_mut()
                .for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data.downcast_mut::<C>().unwrap());
                    }
                }),
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.components.items.par_iter_mut().for_each(|e| match e {
                            ArenaEntry::Free { .. } => (),
                            ArenaEntry::Occupied { data, .. } => {
                                (each)(data.downcast_mut::<C>().unwrap());
                            }
                        })
                    }
                }
            }
        };
        #[cfg(not(feature = "rayon"))]
        self.for_each_mut(group_handles, each)
    }

    pub fn for_each<C: ComponentController>(
        &self,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(&C),
    ) {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component.downcast_ref::<C>().unwrap());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for component in &multiple.components {
                    (each)(component.downcast_ref::<C>().unwrap());
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        for component in &group.components {
                            (each)(component.downcast_ref::<C>().unwrap());
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_mut<C: ComponentController>(
        &mut self,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(&mut C),
    ) {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component.downcast_mut::<C>().unwrap());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for component in &mut multiple.components {
                    (each)(component.downcast_mut::<C>().unwrap());
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        for component in &mut group.components {
                            (each)(component.downcast_mut::<C>().unwrap());
                        }
                    }
                }
            }
        };
    }

    pub fn retain<C: ComponentController + ComponentDerive>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handles: &[GroupHandle],
        #[cfg(feature = "physics")] mut keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] mut keep: impl FnMut(&mut C) -> bool,
    ) {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(c) = component {
                    let c = c.downcast_mut::<C>().unwrap();
                    #[cfg(feature = "physics")]
                    world.remove(c);
                    if !keep(
                        c,
                        #[cfg(feature = "physics")]
                        world,
                    ) {
                        *force_buffer = true;
                        *component = None;
                    }
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.components.retain(|_, component| {
                    let component = component.downcast_mut::<C>().unwrap();
                    if keep(
                        component,
                        #[cfg(feature = "physics")]
                        world,
                    ) {
                        true
                    } else {
                        #[cfg(feature = "physics")]
                        world.remove(component);
                        false
                    }
                });
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.components.retain(|_, component| {
                            let component = component.downcast_mut::<C>().unwrap();
                            if keep(
                                component,
                                #[cfg(feature = "physics")]
                                world,
                            ) {
                                true
                            } else {
                                #[cfg(feature = "physics")]
                                world.remove(component);
                                false
                            }
                        });
                    }
                }
            }
        };
    }

    pub fn index<C: ComponentController>(&self, group: GroupHandle, index: usize) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if index == 0 {
                    if let Some(c) = component {
                        return c.downcast_ref::<C>();
                    }
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(component) = multiple.components.get_unknown_gen(index) {
                    return component.downcast_ref::<C>();
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(group.0) {
                    if let Some(component) = group.components.get_unknown_gen(index) {
                        return component.downcast_ref::<C>();
                    }
                }
                return None;
            }
        };
    }

    pub fn index_mut<C: ComponentController>(
        &mut self,
        group: GroupHandle,
        index: usize,
    ) -> Option<&mut C> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if index == 0 {
                    if let Some(c) = component {
                        return c.downcast_mut::<C>();
                    }
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(component) = multiple.components.get_unknown_gen_mut(index) {
                    return component.downcast_mut::<C>();
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(group.0) {
                    if let Some(component) = group.components.get_unknown_gen_mut(index) {
                        return component.downcast_mut::<C>();
                    }
                }
                return None;
            }
        };
    }

    pub fn get<C: ComponentController>(&self, handle: ComponentHandle) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(c) = component {
                    return c.downcast_ref::<C>();
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(component) = multiple.components.get(handle.component_index().0) {
                    return component.downcast_ref::<C>();
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(handle.group_handle().0) {
                    if let Some(component) = group.components.get(handle.component_index().0) {
                        return component.downcast_ref::<C>();
                    }
                }
                return None;
            }
        };
    }

    pub fn get_mut<C: ComponentController>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(c) = component {
                    return c.downcast_mut::<C>();
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(component) = multiple.components.get_mut(handle.component_index().0) {
                    return component.downcast_mut::<C>();
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(component) = group.components.get_mut(handle.component_index().0) {
                        return component.downcast_mut::<C>();
                    }
                }
                return None;
            }
        };
    }

    pub fn get2_mut<C1: ComponentController, C2: ComponentController>(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C1>, Option<&mut C2>) {
        match &mut self.storage {
            ComponentTypeStorage::Single { .. } => {
                panic!("Cannot get 2 on component with ComponentStorage::Single!");
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut c1 = None;
                let mut c2 = None;
                let result = multiple
                    .components
                    .get2_mut(handle1.component_index().0, handle2.component_index().0);
                if let Some(component) = result.0 {
                    c1 = component.downcast_mut::<C1>();
                }
                if let Some(component) = result.1 {
                    c2 = component.downcast_mut::<C2>();
                }
                return (c1, c2);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut c1 = None;
                let mut c2 = None;
                if handle1.group_handle() == handle2.group_handle() {
                    if let Some(group) = groups.get_mut(handle1.group_handle().0) {
                        let result = group
                            .components
                            .get2_mut(handle1.component_index().0, handle2.component_index().0);
                        if let Some(component) = result.0 {
                            c1 = component.downcast_mut::<C1>();
                        }
                        if let Some(component) = result.1 {
                            c2 = component.downcast_mut::<C2>();
                        }
                    }
                } else {
                    let (group1, group2) =
                        groups.get2_mut(handle1.group_handle().0, handle2.group_handle().0);
                    if let Some(group) = group1 {
                        if let Some(component) =
                            group.components.get_mut(handle1.component_index().0)
                        {
                            c1 = component.downcast_mut::<C1>();
                        }
                    }

                    if let Some(group) = group2 {
                        if let Some(component) =
                            group.components.get_mut(handle2.component_index().0)
                        {
                            c2 = component.downcast_mut::<C2>();
                        }
                    }
                }
                return (c1, c2);
            }
        };
    }

    pub fn get2_boxed_mut(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut BoxedComponent>, Option<&mut BoxedComponent>) {
        match &mut self.storage {
            ComponentTypeStorage::Single { .. } => {
                panic!("Cannot get 2 on component with ComponentStorage::Single!");
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple
                    .components
                    .get2_mut(handle1.component_index().0, handle2.component_index().0);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut c1 = None;
                let mut c2 = None;
                if handle1.group_handle() == handle2.group_handle() {
                    if let Some(group) = groups.get_mut(handle1.group_handle().0) {
                        (c1, c2) = group
                            .components
                            .get2_mut(handle1.component_index().0, handle2.component_index().0);
                    }
                } else {
                    let (group1, group2) =
                        groups.get2_mut(handle1.group_handle().0, handle2.group_handle().0);
                    if let Some(group) = group1 {
                        c1 = group.components.get_mut(handle1.component_index().0);
                    }

                    if let Some(group) = group2 {
                        c2 = group.components.get_mut(handle2.component_index().0);
                    }
                }
                return (c1, c2);
            }
        };
    }

    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                return component.as_ref();
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple.components.get(handle.component_index().0);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(handle.group_handle().0) {
                    return group.components.get(handle.component_index().0);
                }
                return None;
            }
        };
    }

    pub fn get_boxed_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                return component.as_mut();
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple.components.get_mut(handle.component_index().0);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    return group.components.get_mut(handle.component_index().0);
                }
                return None;
            }
        };
    }

    pub fn remove<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(mut component) = component.take() {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    *force_buffer = true;
                    return component.downcast::<C>().ok().and_then(|b| Some(*b));
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(mut component) = multiple.components.remove(handle.component_index().0)
                {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    return component.downcast::<C>().ok().and_then(|b| Some(*b));
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(mut component) = group.components.remove(handle.component_index().0)
                    {
                        #[cfg(feature = "physics")]
                        world.remove(&mut component);
                        return component.downcast::<C>().ok().and_then(|b| Some(*b));
                    }
                }
                return None;
            }
        };
    }

    pub fn remove_boxed(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<BoxedComponent> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(mut component) = component.take() {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    *force_buffer = true;
                    return Some(component);
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(mut component) = multiple.components.remove(handle.component_index().0)
                {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    return Some(component);
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(mut component) = group.components.remove(handle.component_index().0)
                    {
                        #[cfg(feature = "physics")]
                        world.remove(&mut component);
                        return Some(component);
                    }
                }
                return None;
            }
        };
    }

    pub fn remove_all<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handles: &[GroupHandle],
    ) -> Vec<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                let mut result = Vec::with_capacity(1);
                if let Some(mut component) = component.take() {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    *force_buffer = true;
                    result.push(*component.downcast::<C>().ok().unwrap());
                }
                return result;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut result = Vec::with_capacity(multiple.components.len());
                let components = std::mem::replace(&mut multiple.components, Default::default());
                for mut component in components {
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    result.push(*component.downcast::<C>().ok().unwrap())
                }
                return result;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut result = Vec::new();
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        let components =
                            std::mem::replace(&mut group.components, Default::default());
                        for mut component in components {
                            #[cfg(feature = "physics")]
                            world.remove(&mut component);
                            result.push(*component.downcast::<C>().ok().unwrap());
                        }
                    }
                }
                return result;
            }
        };
    }

    pub fn add<C: ComponentDerive + ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        mut new: C,
    ) -> ComponentHandle {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                assert!(component.is_none(), "Single component is already set!");
                let handle = ComponentHandle::new(
                    ComponentIndex::INVALID,
                    self.type_id,
                    GroupHandle::INVALID,
                );
                #[cfg(feature = "physics")]
                world.add(handle, &mut new);
                *component = Some(Box::new(new));
                *force_buffer = true;
                return handle;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.components.insert_with(|idx| {
                    handle = ComponentHandle::new(
                        ComponentIndex(idx),
                        self.type_id,
                        GroupHandle::INVALID,
                    );
                    #[cfg(feature = "physics")]
                    world.add(handle, &mut new);
                    Box::new(new)
                });
                return handle;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.components.insert_with(|idx| {
                    handle = ComponentHandle::new(ComponentIndex(idx), self.type_id, group_handle);
                    #[cfg(feature = "physics")]
                    world.add(handle, &mut new);
                    Box::new(new)
                });
                return handle;
            }
        };
    }

    pub fn add_with<C: ComponentDerive + ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                assert!(component.is_none(), "Single component is already set!");
                let handle = ComponentHandle::new(
                    ComponentIndex::INVALID,
                    self.type_id,
                    GroupHandle::INVALID,
                );
                let mut new = create(handle);
                #[cfg(feature = "physics")]
                world.add(handle, &mut new);
                *component = Some(Box::new(new));
                *force_buffer = true;
                return handle;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.components.insert_with(|idx| {
                    handle = ComponentHandle::new(
                        ComponentIndex(idx),
                        self.type_id,
                        GroupHandle::INVALID,
                    );
                    let mut new = create(handle);
                    #[cfg(feature = "physics")]
                    world.add(handle, &mut new);
                    Box::new(new)
                });
                return handle;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.components.insert_with(|idx| {
                    handle = ComponentHandle::new(ComponentIndex(idx), self.type_id, group_handle);
                    let mut new = create(handle);
                    #[cfg(feature = "physics")]
                    world.add(handle, &mut new);
                    Box::new(new)
                });
                return handle;
            }
        };
    }

    pub fn add_many<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        match &mut self.storage {
            ComponentTypeStorage::Single { .. } => {
                panic!("Cannot add naby on component with ComponentStorage::Single!");
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let components = components.into_iter();
                let mut handles = Vec::with_capacity(components.size_hint().0);
                for mut component in components {
                    multiple.components.insert_with(|idx| {
                        let handle = ComponentHandle::new(
                            ComponentIndex(idx),
                            self.type_id,
                            GroupHandle::INVALID,
                        );
                        #[cfg(feature = "physics")]
                        world.add(handle, &mut component);
                        handles.push(handle);
                        Box::new(component)
                    });
                }
                return handles;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let components = components.into_iter();
                let mut handles = Vec::with_capacity(components.size_hint().0);
                if let Some(group) = groups.get_mut(group_handle.0) {
                    for mut component in components {
                        group.components.insert_with(|idx| {
                            let handle = ComponentHandle::new(
                                ComponentIndex(idx),
                                self.type_id,
                                group_handle,
                            );
                            #[cfg(feature = "physics")]
                            world.add(handle, &mut component);
                            handles.push(handle);
                            Box::new(component)
                        });
                    }
                }
                return handles;
            }
        };
    }

    pub fn force_buffer(&mut self, group_handles: &[GroupHandle]) {
        match &mut self.storage {
            ComponentTypeStorage::Single { force_buffer, .. } => {
                *force_buffer = true;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.force_buffer = true;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.force_buffer = true;
                    }
                }
            }
        };
    }

    pub fn len(&self, group_handles: &[GroupHandle]) -> usize {
        match &self.storage {
            ComponentTypeStorage::Single { .. } => {
                return 1;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple.components.len();
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut len = 0;
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        len += group.components.len();
                    }
                }
                return len;
            }
        };
    }

    pub fn iter<'a, C: ComponentController>(
        &'a self,
        group_handles: &[GroupHandle],
    ) -> Box<dyn DoubleEndedIterator<Item = &'a C> + 'a> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once(component.downcast_ref::<C>().unwrap()));
                } else {
                    return Box::new(std::iter::empty::<&C>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(
                    multiple
                        .components
                        .iter()
                        .map(|c| c.downcast_ref::<C>().unwrap()),
                );
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        if !group.components.is_empty() {
                            iters.push(
                                group
                                    .components
                                    .iter()
                                    .map(|c| c.downcast_ref::<C>().unwrap()),
                            );
                        }
                    }
                }
                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_with_handles<'a, C: ComponentController>(
        &'a self,
        group_handles: &'a [GroupHandle],
    ) -> Box<dyn DoubleEndedIterator<Item = (ComponentHandle, &'a C)> + 'a> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            self.type_id,
                            GroupHandle::INVALID,
                        ),
                        component.downcast_ref::<C>().unwrap(),
                    )));
                } else {
                    return Box::new(std::iter::empty::<(ComponentHandle, &'a C)>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter_with_index().map(|(idx, c)| {
                    (
                        ComponentHandle::new(
                            ComponentIndex(idx),
                            self.type_id,
                            GroupHandle::INVALID,
                        ),
                        c.downcast_ref::<C>().unwrap(),
                    )
                }));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group_handle in group_handles {
                    if let Some(group) = groups.get(group_handle.0) {
                        if !group.components.is_empty() {
                            iters.push(group.components.iter_with_index().map(|(idx, c)| {
                                (
                                    ComponentHandle::new(
                                        ComponentIndex(idx),
                                        self.type_id,
                                        *group_handle,
                                    ),
                                    c.downcast_ref::<C>().unwrap(),
                                )
                            }));
                        }
                    }
                }
                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_mut<'a, C: ComponentController>(
        &'a mut self,
        group_handles: &[GroupHandle],
        check: bool,
    ) -> Box<dyn DoubleEndedIterator<Item = &'a mut C> + 'a> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once(component.downcast_mut::<C>().unwrap()));
                } else {
                    return Box::new(std::iter::empty::<&mut C>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(
                    multiple
                        .components
                        .iter_mut()
                        .map(|c| c.downcast_mut::<C>().unwrap()),
                );
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if check && groups.len() > 1 {
                    for (index, value) in groups.iter_with_index().enumerate() {
                        for other in groups.iter_with_index().skip(index + 1) {
                            assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                        }
                    }
                }
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<ComponentTypeGroup> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                            iters.push(
                                group
                                    .components
                                    .iter_mut()
                                    .map(|c| c.downcast_mut::<C>().unwrap()),
                            );
                        };
                    }
                }

                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn iter_mut_with_handles<'a, C: ComponentController>(
        &'a mut self,
        group_handles: &'a [GroupHandle],
        check: bool,
    ) -> Box<dyn DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> + 'a> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            self.type_id,
                            GroupHandle::INVALID,
                        ),
                        component.downcast_mut::<C>().unwrap(),
                    )));
                } else {
                    return Box::new(std::iter::empty::<(ComponentHandle, &mut C)>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter_with_index_mut().map(|(idx, c)| {
                    (
                        ComponentHandle::new(
                            ComponentIndex(idx),
                            self.type_id,
                            GroupHandle::INVALID,
                        ),
                        c.downcast_mut::<C>().unwrap(),
                    )
                }));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if check && groups.len() > 1 {
                    for (index, value) in groups.iter_with_index().enumerate() {
                        for other in groups.iter_with_index().skip(index + 1) {
                            assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                        }
                    }
                }
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<ComponentTypeGroup> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                            let type_id = &self.type_id;

                            iters.push(group.components.iter_with_index_mut().map(
                                move |(idx, c)| {
                                    (
                                        ComponentHandle::new(
                                            ComponentIndex(idx),
                                            *type_id,
                                            *group_handle,
                                        ),
                                        c.downcast_mut::<C>().unwrap(),
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

    pub fn iter_render<'a, C: ComponentController>(
        &'a self,
        group_handles: &[GroupHandle],
    ) -> Box<dyn DoubleEndedIterator<Item = (&'a InstanceBuffer, InstanceIndex, &'a C)> + 'a> {
        match &self.storage {
            ComponentTypeStorage::Single {
                component, buffer, ..
            } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        buffer.as_ref().expect(BUFFER_ERROR),
                        InstanceIndex::new(0),
                        component.downcast_ref::<C>().unwrap(),
                    )));
                } else {
                    return Box::new(std::iter::empty::<(&InstanceBuffer, InstanceIndex, &C)>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter().enumerate().map(|(i, c)| {
                    (
                        multiple.buffer.as_ref().expect(BUFFER_ERROR),
                        InstanceIndex::new(i as u32),
                        c.downcast_ref::<C>().unwrap(),
                    )
                }));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        if !group.components.is_empty() {
                            iters.push(group.components.iter().enumerate().map(|(i, c)| {
                                (
                                    group.buffer.as_ref().expect(BUFFER_ERROR),
                                    InstanceIndex::new(i as u32),
                                    c.downcast_ref::<C>().unwrap(),
                                )
                            }));
                        }
                    }
                }
                return Box::new(iters.into_iter().flatten());
            }
        };
    }

    pub fn render_each<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        mut each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        renderer.use_camera(camera);
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                renderer.use_instance_buffer(buffer.as_ref().expect(BUFFER_ERROR));
                if let Some(component) = component {
                    (each)(
                        renderer,
                        component.downcast_ref::<C>().unwrap(),
                        InstanceIndex::new(0),
                    );
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                renderer.use_instance_buffer(multiple.buffer.as_ref().expect(BUFFER_ERROR));
                for (instance, component) in multiple.components.iter().enumerate() {
                    (each)(
                        renderer,
                        component.downcast_ref::<C>().unwrap(),
                        InstanceIndex::new(instance as u32),
                    );
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    renderer.use_instance_buffer(group.buffer.as_ref().expect(BUFFER_ERROR));
                    for (instance, component) in group.components.iter().enumerate() {
                        (each)(
                            renderer,
                            component.downcast_ref::<C>().unwrap(),
                            InstanceIndex::new(instance as u32),
                        );
                    }
                }
            }
        }
    }

    pub fn render_single<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        each: impl FnOnce(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        renderer.use_camera(camera);
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                renderer.use_instance_buffer(buffer.as_ref().expect(BUFFER_ERROR));
                if let Some(component) = component {
                    (each)(
                        renderer,
                        component.downcast_ref::<C>().unwrap(),
                        InstanceIndex::new(0),
                    );
                }
            }
            _ => {
                panic!("Cannot get single on component without ComponentStorage::Single!")
            }
        }
    }

    pub fn render_each_prepare<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        prepare: impl FnOnce(&mut Renderer<'a>),
        mut each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        renderer.use_camera(camera);
        prepare(renderer);
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                renderer.use_instance_buffer(buffer.as_ref().expect(BUFFER_ERROR));
                if let Some(component) = component {
                    (each)(
                        renderer,
                        component.downcast_ref::<C>().unwrap(),
                        InstanceIndex::new(0),
                    );
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                renderer.use_instance_buffer(multiple.buffer.as_ref().expect(BUFFER_ERROR));
                for (instance, component) in multiple.components.iter().enumerate() {
                    (each)(
                        renderer,
                        component.downcast_ref::<C>().unwrap(),
                        InstanceIndex::new(instance as u32),
                    );
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    renderer.use_instance_buffer(group.buffer.as_ref().expect(BUFFER_ERROR));
                    for (instance, component) in group.components.iter().enumerate() {
                        (each)(
                            renderer,
                            component.downcast_ref::<C>().unwrap(),
                            InstanceIndex::new(instance as u32),
                        );
                    }
                }
            }
        }
    }

    pub fn render_all<'a>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        mut all: impl FnMut(&mut Renderer<'a>, InstanceIndices),
    ) {
        renderer.use_camera(camera);
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                if component.is_some() {
                    let buffer = buffer.as_ref().expect(BUFFER_ERROR);
                    renderer.use_instance_buffer(buffer);
                    (all)(renderer, buffer.instances());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if !multiple.components.is_empty() {
                    let buffer = multiple.buffer.as_ref().expect(BUFFER_ERROR);
                    renderer.use_instance_buffer(buffer);
                    (all)(renderer, buffer.instances());
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    if !group.components.is_empty() {
                        let buffer = group.buffer.as_ref().expect(BUFFER_ERROR);
                        renderer.use_instance_buffer(buffer);
                        (all)(renderer, buffer.instances());
                    }
                }
            }
        }
    }

    pub fn single<C: ComponentController>(&self) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return component.downcast_ref::<C>();
                }
                return None;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn single_mut<C: ComponentController>(&mut self) -> Option<&mut C> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return component.downcast_mut::<C>();
                }
                return None;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn remove_single<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
    ) -> Option<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(mut component) = component.take() {
                    *force_buffer = true;
                    #[cfg(feature = "physics")]
                    world.remove(&mut component);
                    return component.downcast::<C>().ok().and_then(|b| Some(*b));
                }
                return None;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn set_single<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        mut new: C,
    ) -> ComponentHandle {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                *force_buffer = true;
                let handle = ComponentHandle::new(
                    ComponentIndex::INVALID,
                    self.type_id,
                    GroupHandle::INVALID,
                );
                #[cfg(feature = "physics")]
                world.add(handle, &mut new);
                if let Some(mut _old) = component.replace(Box::new(new)) {
                    #[cfg(feature = "physics")]
                    world.remove(&mut _old);
                }
                return handle;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn set_single_with<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                let handle = ComponentHandle::new(
                    ComponentIndex::INVALID,
                    self.type_id,
                    GroupHandle::INVALID,
                );
                let mut new = create(handle);
                #[cfg(feature = "physics")]
                world.add(handle, &mut new);
                *force_buffer = true;
                if let Some(mut _old) = component.replace(Box::new(new)) {
                    #[cfg(feature = "physics")]
                    world.remove(&mut _old);
                }
                return handle;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn par_buffer_for_each_mut<C: ComponentController + ComponentBuffer>(
        &mut self,
        #[cfg(feature = "physics")] world: &World,
        gpu: &Gpu,
        group_handles: &[GroupHandle],
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        assert!(C::CONFIG.buffer != BufferOperation::Never);
        #[cfg(feature = "rayon")]
        match &mut self.storage {
            ComponentTypeStorage::Single {
                component, buffer, ..
            } => {
                if let Some(component) = component {
                    if buffer.is_none() {
                        *buffer = Some(InstanceBuffer::empty(gpu, self.instance_size, 1));
                    }
                    let buffer = buffer.as_mut().unwrap();
                    let helper = BufferHelper::new(
                        #[cfg(feature = "physics")]
                        world,
                        buffer,
                        BufferHelperType::Single {
                            offset: 0,
                            component: component,
                        },
                    );
                    C::buffer_with(gpu, helper, each);
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.resize_buffer(gpu, C::INSTANCE_SIZE);
                let helper = BufferHelper::new(
                    #[cfg(feature = "physics")]
                    world,
                    multiple.buffer.as_mut().unwrap(),
                    BufferHelperType::All {
                        components: &mut multiple.components,
                    },
                );
                C::buffer_with(gpu, helper, each);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.resize_buffer(gpu, C::INSTANCE_SIZE);
                        let helper = BufferHelper::new(
                            #[cfg(feature = "physics")]
                            world,
                            group.buffer.as_mut().unwrap(),
                            BufferHelperType::All {
                                components: &mut group.components,
                            },
                        );
                        C::buffer_with(gpu, helper, each);
                    }
                }
            }
        };
        #[cfg(not(feature = "rayon"))]
        self.buffer_for_each_mut(group_handles, each)
    }
}
