use std::fmt::{Display, Formatter, Result};

#[cfg(feature = "rayon")]
use crate::{data::arena::ArenaEntry, rayon::prelude::*};

use crate::{
    Arena, BufferOperation, Component, ComponentConfig, ComponentHandle, ComponentIndex,
    ComponentStorage, ComponentTypeImplementation, Gpu, GroupHandle, InstanceBuffer,
    InstanceHandler, InstanceIndex, InstanceIndices, Renderer, World,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
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

impl std::hash::Hash for ComponentTypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

/// Trait to identify a struct that derives from  the [Component](crate::Component) macro using
/// a [ComponentTypeId]
pub trait ComponentIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ComponentTypeId;
    fn component_type_id(&self) -> ComponentTypeId;
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum ComponentTypeStorage<C: Component> {
    Single {
        #[cfg_attr(feature = "serde", serde(skip))]
        #[cfg_attr(feature = "serde", serde(default))]
        buffer: Option<InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>>,
        #[cfg_attr(feature = "serde", serde(skip))]
        #[cfg_attr(feature = "serde", serde(default = "default_true"))]
        force_buffer: bool,
        component: Option<C>,
    },
    Multiple(ComponentTypeGroup<C>),
    MultipleGroups(Arena<ComponentTypeGroup<C>>),
}

impl<C: Component> Clone for ComponentTypeStorage<C> {
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
pub(crate) struct ComponentTypeGroup<C: Component> {
    pub components: Arena<C>,
    force_buffer: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    buffer: Option<InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>>,
}

impl<C: Component> Clone for ComponentTypeGroup<C> {
    fn clone(&self) -> Self {
        Self {
            buffer: None,
            components: Default::default(),
            force_buffer: true,
        }
    }
}

impl<C: Component> ComponentTypeGroup<C> {
    pub fn new() -> Self {
        Self {
            components: Arena::new(),
            buffer: None,
            force_buffer: true,
        }
    }

    fn buffer(&mut self, gpu: &Gpu, config: &ComponentConfig, world: &World) {
        if config.buffer == BufferOperation::EveryFrame || self.force_buffer {
            self.force_buffer = false;
            let instances = self
                .components
                .iter()
                .filter_map(|component| {
                    if component.handler().active() {
                        Some(component.handler().instance(world))
                    } else {
                        None
                    }
                })
                .collect::<Vec<<C::InstanceHandler as InstanceHandler>::Instance>>();

            if let Some(buffer) = self.buffer.as_mut() {
                buffer.write(gpu, &instances);
            } else {
                self.buffer = Some(gpu.create_instance_buffer(&instances));
            }
        }
    }

    fn buffer_with(
        &mut self,
        gpu: &Gpu,
        config: &ComponentConfig,
        world: &World,
        mut each: impl FnMut(&mut C),
    ) {
        if config.buffer == BufferOperation::EveryFrame || self.force_buffer {
            self.force_buffer = false;
            let instances = self
                .components
                .iter_mut()
                .filter_map(|component| {
                    (each)(component);
                    if component.handler().active() {
                        Some(component.handler().instance(world))
                    } else {
                        None
                    }
                })
                .collect::<Vec<<C::InstanceHandler as InstanceHandler>::Instance>>();

            if let Some(buffer) = self.buffer.as_mut() {
                buffer.write(gpu, &instances);
            } else {
                self.buffer = Some(gpu.create_instance_buffer(&instances));
            }
        }
    }
}

#[cfg(feature = "rayon")]
impl<C: Component + Send + Sync> ComponentTypeGroup<C>
where
    <C::InstanceHandler as InstanceHandler>::Instance: Send,
{
    fn par_buffer(&mut self, gpu: &Gpu, config: &ComponentConfig, world: &World) {
        if config.buffer == BufferOperation::EveryFrame || self.force_buffer {
            self.force_buffer = false;
            let instances = self
                .components
                .items
                .par_iter_mut()
                .filter_map(|component| match component {
                    ArenaEntry::Free { .. } => None,
                    ArenaEntry::Occupied { data, .. } => {
                        if data.handler().active() {
                            Some(data.handler().instance(world))
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<<C::InstanceHandler as InstanceHandler>::Instance>>();

            if let Some(buffer) = self.buffer.as_mut() {
                buffer.write(gpu, &instances);
            } else {
                self.buffer = Some(gpu.create_instance_buffer(&instances));
            }
        }
    }

    fn par_buffer_with(
        &mut self,
        gpu: &Gpu,
        config: &ComponentConfig,
        world: &World,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        if config.buffer == BufferOperation::EveryFrame || self.force_buffer {
            self.force_buffer = false;
            let instances = self
                .components
                .items
                .par_iter_mut()
                .filter_map(|component| match component {
                    ArenaEntry::Free { .. } => None,
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data);
                        if data.handler().active() {
                            Some(data.handler().instance(world))
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<<C::InstanceHandler as InstanceHandler>::Instance>>();

            if let Some(buffer) = self.buffer.as_mut() {
                buffer.write(gpu, &instances);
            } else {
                self.buffer = Some(gpu.create_instance_buffer(&instances));
            }
        }
    }
}

const BUFFER_ERROR: &'static str =
    "This component either has no buffer or it has not been initialized yet!";

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub(crate) struct ComponentType<C: Component> {
    config: ComponentConfig,
    pub(crate) storage: ComponentTypeStorage<C>,
}

#[cfg_attr(not(feature = "physics"), allow(unused_mut))]
impl<C: Component> ComponentType<C> {
    pub(crate) fn new(config: ComponentConfig) -> Self {
        let storage = match config.storage {
            ComponentStorage::Single => ComponentTypeStorage::Single {
                buffer: None,
                force_buffer: true,
                component: None,
            },
            ComponentStorage::Multiple => ComponentTypeStorage::Multiple(ComponentTypeGroup::new()),
            ComponentStorage::Groups => ComponentTypeStorage::MultipleGroups(Arena::new()),
        };
        Self { storage, config }
    }

    pub fn component_type_id(&self) -> ComponentTypeId {
        C::IDENTIFIER
    }

    pub fn for_each(&self, group_handles: &[GroupHandle], mut each: impl FnMut(&C)) {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component);
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for component in &multiple.components {
                    (each)(component);
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        for component in &group.components {
                            (each)(component);
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_mut(&mut self, group_handles: &[GroupHandle], mut each: impl FnMut(&mut C)) {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component);
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for component in &mut multiple.components {
                    (each)(component);
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        for component in &mut group.components {
                            (each)(component);
                        }
                    }
                }
            }
        };
    }

    pub fn for_each_with_handles(
        &self,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(ComponentHandle, &C),
    ) {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
                    );
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for (idx, component) in multiple.components.iter_with_index() {
                    (each)(
                        ComponentHandle::new(
                            ComponentIndex(idx),
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
                    );
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get(group_handle.0) {
                        for (idx, component) in group.components.iter_with_index() {
                            (each)(
                                ComponentHandle::new(
                                    ComponentIndex(idx),
                                    C::IDENTIFIER,
                                    *group_handle,
                                ),
                                component,
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
        mut each: impl FnMut(ComponentHandle, &mut C),
    ) {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
                    );
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for (idx, component) in multiple.components.iter_mut_with_index() {
                    (each)(
                        ComponentHandle::new(
                            ComponentIndex(idx),
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
                    );
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group_handle in group_handles {
                    if let Some(group) = groups.get_mut(group_handle.0) {
                        for (idx, component) in group.components.iter_mut_with_index() {
                            (each)(
                                ComponentHandle::new(
                                    ComponentIndex(idx),
                                    C::IDENTIFIER,
                                    *group_handle,
                                ),
                                component,
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
        mut keep: impl FnMut(&mut C, &mut World) -> bool,
    ) {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(c) = component {
                    let c = c;
                    c.finish(world);
                    if !keep(c, world) {
                        *force_buffer = true;
                        *component = None;
                    }
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.components.retain(|_, component| {
                    let component = component;
                    if keep(component, world) {
                        true
                    } else {
                        component.finish(world);
                        false
                    }
                });
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.components.retain(|_, component| {
                            let component = component;
                            if keep(component, world) {
                                true
                            } else {
                                component.finish(world);
                                false
                            }
                        });
                    }
                }
            }
        };
    }

    pub fn index(&self, group: GroupHandle, index: usize) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                return component.as_ref();
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple.components.get_unknown_gen(index);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get(group.0) {
                    return group.components.get_unknown_gen(index);
                }
                return None;
            }
        };
    }

    pub fn index_mut(&mut self, group: GroupHandle, index: usize) -> Option<&mut C> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if index == 0 {
                    return component.as_mut();
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return multiple.components.get_unknown_gen_mut(index);
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(group.0) {
                    return group.components.get_unknown_gen_mut(index);
                }
                return None;
            }
        };
    }

    pub fn get(&self, handle: ComponentHandle) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => return component.as_ref(),
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

    pub fn get_mut(&mut self, handle: ComponentHandle) -> Option<&mut C> {
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

    pub fn get2_mut(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C>, Option<&mut C>) {
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

    pub fn remove(&mut self, world: &mut World, handle: ComponentHandle) -> Option<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(mut component) = component.take() {
                    component.finish(world);
                    *force_buffer = true;
                    return Some(component);
                }
                return None;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                if let Some(mut component) = multiple.components.remove(handle.component_index().0)
                {
                    component.finish(world);
                    return Some(component);
                }
                return None;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.get_mut(handle.group_handle().0) {
                    if let Some(mut component) = group.components.remove(handle.component_index().0)
                    {
                        component.finish(world);
                        return Some(component);
                    }
                }
                return None;
            }
        };
    }

    pub fn remove_all(&mut self, world: &mut World, group_handles: &[GroupHandle]) -> Vec<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                let mut result = Vec::with_capacity(1);
                if let Some(mut component) = component.take() {
                    component.finish(world);
                    *force_buffer = true;
                    result.push(component);
                }
                return result;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut result = Vec::with_capacity(multiple.components.len());
                let components = std::mem::replace(&mut multiple.components, Default::default());
                for mut component in components {
                    component.finish(world);
                    result.push(component)
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
                            component.finish(world);
                            result.push(component);
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
                    C::IDENTIFIER,
                    GroupHandle::INVALID,
                );
                new.init(handle, world);
                *component = Some(new);
                *force_buffer = true;
                return handle;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.components.insert_with(|idx| {
                    handle = ComponentHandle::new(
                        ComponentIndex(idx),
                        C::IDENTIFIER,
                        GroupHandle::INVALID,
                    );
                    new.init(handle, world);
                    new
                });
                return handle;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.components.insert_with(|idx| {
                    handle = ComponentHandle::new(ComponentIndex(idx), C::IDENTIFIER, group_handle);
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
                    C::IDENTIFIER,
                    GroupHandle::INVALID,
                );
                let mut new = create(handle);
                new.init(handle, world);
                *component = Some(new);
                *force_buffer = true;
                return handle;
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let mut handle = Default::default();
                multiple.components.insert_with(|idx| {
                    handle = ComponentHandle::new(
                        ComponentIndex(idx),
                        C::IDENTIFIER,
                        GroupHandle::INVALID,
                    );
                    let mut new = create(handle);
                    new.init(handle, world);
                    new
                });
                return handle;
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let group = &mut groups[group_handle.0];
                let mut handle = Default::default();
                group.components.insert_with(|idx| {
                    handle = ComponentHandle::new(ComponentIndex(idx), C::IDENTIFIER, group_handle);
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
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        );
                        component.init(handle, world);
                        handles.push(handle);
                        component
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
                                C::IDENTIFIER,
                                group_handle,
                            );
                            component.init(handle, world);
                            handles.push(handle);
                            component
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
            ComponentTypeStorage::Single { component, .. } => {
                if component.is_some() {
                    return 1;
                } else {
                    return 0;
                }
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

    pub fn iter<'a>(
        &'a self,
        group_handles: &[GroupHandle],
    ) -> Box<dyn DoubleEndedIterator<Item = &'a C> + 'a> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once(component));
                } else {
                    return Box::new(std::iter::empty::<&C>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter().map(|c| c));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        if !group.components.is_empty() {
                            iters.push(group.components.iter().map(|c| c));
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
    ) -> Box<dyn DoubleEndedIterator<Item = (ComponentHandle, &'a C)> + 'a> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
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
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        c,
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
                                        C::IDENTIFIER,
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
    ) -> Box<dyn DoubleEndedIterator<Item = &'a mut C> + 'a> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once(component));
                } else {
                    return Box::new(std::iter::empty::<&mut C>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter_mut().map(|c| c));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<ComponentTypeGroup<C>> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                            iters.push(group.components.iter_mut().map(|c| c));
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
    ) -> Box<dyn DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> + 'a> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        ComponentHandle::new(
                            ComponentIndex::INVALID,
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        component,
                    )));
                } else {
                    return Box::new(std::iter::empty::<(ComponentHandle, &mut C)>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter_mut_with_index().map(|(idx, c)| {
                    (
                        ComponentHandle::new(
                            ComponentIndex(idx),
                            C::IDENTIFIER,
                            GroupHandle::INVALID,
                        ),
                        c,
                    )
                }));
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut iters = Vec::with_capacity(groups.len());
                let ptr: *mut Arena<ComponentTypeGroup<C>> = groups as *mut _;
                unsafe {
                    for group_handle in group_handles {
                        if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                            let type_id = &C::IDENTIFIER;

                            iters.push(group.components.iter_mut_with_index().map(
                                move |(idx, c)| {
                                    (
                                        ComponentHandle::new(
                                            ComponentIndex(idx),
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

    pub fn iter_render<'a>(
        &'a self,
        group_handles: &[GroupHandle],
    ) -> Box<
        dyn DoubleEndedIterator<
                Item = (
                    &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
                    InstanceIndex,
                    &'a C,
                ),
            > + 'a,
    > {
        match &self.storage {
            ComponentTypeStorage::Single {
                component, buffer, ..
            } => {
                if let Some(component) = component {
                    return Box::new(std::iter::once((
                        buffer.as_ref().expect(BUFFER_ERROR),
                        InstanceIndex::new(0),
                        component,
                    )));
                } else {
                    return Box::new(std::iter::empty::<(
                        &InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
                        InstanceIndex,
                        &C,
                    )>());
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                return Box::new(multiple.components.iter().enumerate().map(|(i, c)| {
                    (
                        multiple.buffer.as_ref().expect(BUFFER_ERROR),
                        InstanceIndex::new(i as u32),
                        c,
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

    pub(crate) fn render_each<'a>(
        &'a self,
        renderer: &mut Renderer<'a>,
        mut each: impl FnMut(
            &mut Renderer<'a>,
            &'a C,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndex,
        ),
    ) {
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                if let Some(component) = component {
                    let buffer = buffer.as_ref().expect(BUFFER_ERROR);
                    if buffer.instance_amount() > 0 {
                        (each)(renderer, component, buffer, InstanceIndex::new(0));
                    }
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let buffer = multiple.buffer.as_ref().expect(BUFFER_ERROR);
                if buffer.instance_amount() > 0 {
                    for (instance, component) in multiple.components.iter().enumerate() {
                        (each)(
                            renderer,
                            component,
                            buffer,
                            InstanceIndex::new(instance as u32),
                        );
                    }
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    let buffer = group.buffer.as_ref().expect(BUFFER_ERROR);
                    if buffer.instance_amount() > 0 {
                        for (instance, component) in group.components.iter().enumerate() {
                            (each)(
                                renderer,
                                component,
                                buffer,
                                InstanceIndex::new(instance as u32),
                            );
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn render_single<'a>(
        &'a self,
        renderer: &mut Renderer<'a>,
        each: impl FnOnce(
            &mut Renderer<'a>,
            &'a C,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndex,
        ),
    ) {
        match &self.storage {
            ComponentTypeStorage::Single {
                buffer, component, ..
            } => {
                if let Some(component) = component {
                    let buffer = buffer.as_ref().expect(BUFFER_ERROR);
                    if buffer.instance_amount() > 0 {
                        (each)(renderer, component, buffer, InstanceIndex::new(0));
                    }
                }
            }
            _ => {
                panic!("Cannot get single on component without ComponentStorage::Single!")
            }
        }
    }

    pub(crate) fn render_all<'a>(
        &'a self,
        renderer: &mut Renderer<'a>,
        mut all: impl FnMut(
            &mut Renderer<'a>,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndices,
        ),
    ) {
        match &self.storage {
            ComponentTypeStorage::Single { buffer, .. } => {
                let buffer = buffer.as_ref().expect(BUFFER_ERROR);
                if buffer.instance_amount() > 0 {
                    (all)(renderer, buffer, InstanceIndices::new(0, 1));
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let buffer = multiple.buffer.as_ref().expect(BUFFER_ERROR);
                if buffer.instance_amount() > 0 {
                    (all)(renderer, buffer, buffer.instances());
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    let buffer = group.buffer.as_ref().expect(BUFFER_ERROR);
                    if buffer.instance_amount() > 0 {
                        (all)(renderer, buffer, buffer.instances());
                    }
                }
            }
        }
    }

    pub fn change_group(
        &mut self,
        component: ComponentHandle,
        new_group_handle: GroupHandle,
    ) -> Option<ComponentHandle> {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                let (old_group, new_group) =
                    groups.get2_mut(component.group_handle().0, new_group_handle.0);
                let old_group = old_group?;
                let new_group = new_group?;
                let component = old_group.components.remove(component.component_index().0)?;
                let component_index = ComponentIndex(new_group.components.insert(component));

                return Some(ComponentHandle::new(
                    component_index,
                    C::IDENTIFIER,
                    new_group_handle,
                ));
            }
            _ => panic!("Cannot get change group on component without ComponentStorage::Group!"),
        }
    }

    pub fn single(&self) -> &C {
        self.try_single().expect("Singleton not defined!")
    }

    pub fn single_mut(&mut self) -> &mut C {
        self.try_single_mut().expect("Singleton not defined!")
    }

    pub fn try_single(&self) -> Option<&C> {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                return component.as_ref();
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn try_single_mut(&mut self) -> Option<&mut C> {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                return component.as_mut();
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn remove_single(&mut self, world: &mut World) -> Option<C> {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                if let Some(mut component) = component.take() {
                    *force_buffer = true;
                    component.finish(world);
                    return Some(component);
                }
                return None;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn set_single(&mut self, world: &mut World, mut new: C) -> ComponentHandle {
        match &mut self.storage {
            ComponentTypeStorage::Single {
                force_buffer,
                component,
                ..
            } => {
                *force_buffer = true;
                let handle = ComponentHandle::new(
                    ComponentIndex::INVALID,
                    C::IDENTIFIER,
                    GroupHandle::INVALID,
                );
                new.init(handle, world);
                if let Some(mut _old) = component.replace(new) {
                    _old.finish(world);
                }
                return handle;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn set_single_with(
        &mut self,
        world: &mut World,
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
                    C::IDENTIFIER,
                    GroupHandle::INVALID,
                );
                let mut new = create(handle);
                new.init(handle, world);
                *force_buffer = true;
                if let Some(mut _old) = component.replace(new) {
                    _old.finish(world);
                }
                return handle;
            }
            _ => panic!("Cannot get single on component without ComponentStorage::Single!"),
        }
    }

    pub fn buffer_with(
        &mut self,
        world: &World,
        gpu: &Gpu,
        group_handles: &[GroupHandle],
        mut each: impl FnMut(&mut C),
    ) {
        assert!(self.config.buffer != BufferOperation::Never);
        match &mut self.storage {
            ComponentTypeStorage::Single {
                component,
                buffer,
                force_buffer,
            } => {
                if self.config.buffer == BufferOperation::EveryFrame || *force_buffer {
                    *force_buffer = false;
                    let instance = {
                        if let Some(component) = component {
                            (each)(component);
                            if component.handler().active() {
                                Some(component.handler().instance(world))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(buffer) = buffer.as_mut() {
                        buffer.write(
                            gpu,
                            instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
                        );
                    } else {
                        *buffer = Some(gpu.create_instance_buffer(
                            instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
                        ));
                    }
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.buffer_with(gpu, &self.config, world, each)
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.buffer_with(gpu, &self.config, world, &mut each)
                    }
                }
            }
        };
    }
}

impl<C: Component> ComponentTypeImplementation for ComponentType<C> {
    fn add_group(&mut self) {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                groups.insert(ComponentTypeGroup::new());
            }
            _ => {}
        }
    }

    fn remove_group(&mut self, world: &mut World, handle: GroupHandle) {
        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(group) = groups.remove(handle.0) {
                    // Checked because of serializing groups
                    for mut component in group.components {
                        component.finish(world)
                    }
                }
            }
            _ => {}
        }
    }

    fn buffer(&mut self, world: &World, gpu: &Gpu, active: &[GroupHandle]) {
        assert!(self.config.buffer != BufferOperation::Never);

        match &mut self.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                for index in active {
                    let group = &mut groups[index.0];
                    group.buffer(gpu, &self.config, world)
                }
            }
            ComponentTypeStorage::Multiple(multiple) => multiple.buffer(gpu, &self.config, world),
            ComponentTypeStorage::Single {
                buffer,
                force_buffer,
                component,
            } => {
                if self.config.buffer == BufferOperation::EveryFrame || *force_buffer {
                    *force_buffer = false;
                    let instance = {
                        if let Some(component) = component {
                            if component.handler().active() {
                                Some(component.handler().instance(world))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(buffer) = buffer.as_mut() {
                        buffer.write(
                            gpu,
                            instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
                        );
                    } else {
                        *buffer = Some(gpu.create_instance_buffer(
                            instance.as_ref().map(core::slice::from_ref).unwrap_or(&[]),
                        ));
                    }
                }
            }
        }
    }

    #[cfg(all(feature = "serde", feature = "physics"))]
    fn deinit_non_serialized(&self, world: &mut World) {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    world.remove_no_maintain(component.handler())
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                for component in &multiple.components {
                    world.remove_no_maintain(component.handler())
                }
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in groups {
                    for component in &group.components {
                        world.remove_no_maintain(component.handler())
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
            ComponentTypeStorage::MultipleGroups(groups) => {
                if let Some(mut group) = groups.remove(handle.0) {
                    for component in &mut group.components {
                        component.finish(world)
                    }
                    return Some(Box::new(group));
                }
            }
            _ => {}
        }
        return None;
    }

    fn component_type_id(&self) -> ComponentTypeId {
        C::IDENTIFIER
    }

    fn config(&self) -> ComponentConfig {
        self.config
    }
}

#[cfg(feature = "rayon")]
impl<C: Component + Send + Sync> ComponentType<C> {
    pub fn par_for_each(&self, group_handles: &[GroupHandle], each: impl Fn(&C) + Send + Sync) {
        match &self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component);
                }
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.components.items.par_iter().for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data);
                    }
                })
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get(group.0) {
                        group.components.items.par_iter().for_each(|e| match e {
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
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        match &mut self.storage {
            ComponentTypeStorage::Single { component, .. } => {
                if let Some(component) = component {
                    (each)(component);
                }
            }
            ComponentTypeStorage::Multiple(multiple) => multiple
                .components
                .items
                .par_iter_mut()
                .for_each(|e| match e {
                    ArenaEntry::Free { .. } => (),
                    ArenaEntry::Occupied { data, .. } => {
                        (each)(data);
                    }
                }),
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.components.items.par_iter_mut().for_each(|e| match e {
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

#[cfg(feature = "rayon")]
impl<C: Component + Send + Sync> ComponentType<C>
where
    <C::InstanceHandler as InstanceHandler>::Instance: Send,
{
    pub fn par_buffer_with(
        &mut self,
        world: &World,
        gpu: &Gpu,
        group_handles: &[GroupHandle],
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        assert!(self.config.buffer != BufferOperation::Never);
        match &mut self.storage {
            ComponentTypeStorage::Single { .. } => {
                self.buffer_with(world, gpu, group_handles, each)
            }
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.par_buffer_with(gpu, &self.config, world, each)
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.par_buffer_with(gpu, &self.config, world, &each)
                    }
                }
            }
        };
    }

    pub fn par_buffer(&mut self, world: &World, gpu: &Gpu, group_handles: &[GroupHandle]) {
        assert!(self.config.buffer != BufferOperation::Never);
        match &mut self.storage {
            ComponentTypeStorage::Single { .. } => self.buffer(world, gpu, group_handles),
            ComponentTypeStorage::Multiple(multiple) => {
                multiple.par_buffer(gpu, &self.config, world)
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                for group in group_handles {
                    if let Some(group) = groups.get_mut(group.0) {
                        group.par_buffer(gpu, &self.config, world)
                    }
                }
            }
        };
    }
}
