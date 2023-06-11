use std::fmt::{Display, Formatter, Result};

use instant::Instant;

#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle, World, WorldChanges};

use crate::{
    data::arena::ArenaEntry, Arena, BoxedComponent, BufferOperation, ComponentConfig,
    ComponentController, ComponentDerive, ComponentHandle, ComponentIndex, Context, Gpu, Group,
    GroupHandle, InstanceBuffer, InstanceIndex, Matrix, RenderEncoder, TypeIndex,
};

#[derive(Clone, Copy)]
pub(crate) struct ComponentCallbacks {
    pub update: fn(ctx: &mut Context),
    pub render: fn(ctx: &Context, encoder: &mut RenderEncoder),
    #[cfg(feature = "physics")]
    pub collision: fn(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ),
}

impl ComponentCallbacks {
    pub fn new<C: ComponentController>() -> Self {
        return Self {
            update: C::update,
            #[cfg(feature = "physics")]
            collision: C::collision,
            render: C::render,
        };
    }
}

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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentTypeGroup {
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub components: Arena<BoxedComponent>,
    pub force_buffer: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    buffer: Option<InstanceBuffer>,
    last_len: usize,
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

    fn instances(&self, #[cfg(feature = "physics")] world: &mut World) -> Vec<Matrix> {
        self.components
            .iter()
            .map(|(_, component)| {
                component.base().matrix(
                    #[cfg(feature = "physics")]
                    world,
                )
            })
            .collect::<Vec<Matrix>>()
    }

    pub fn buffer(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        every_frame: bool,
        gpu: &Gpu,
    ) {
        let new_len = self.components.len();
        if new_len != self.last_len {
            // We have to resize the buffer
            let instances = self.instances(
                #[cfg(feature = "physics")]
                world,
            );
            self.last_len = new_len;
            self.buffer = Some(InstanceBuffer::new(gpu, &instances[..]));
        } else if every_frame || self.force_buffer {
            let instances = self.instances(
                #[cfg(feature = "physics")]
                world,
            );
            self.force_buffer = false;
            if let Some(buffer) = &mut self.buffer {
                buffer.write(gpu, &instances[..]);
            } else {
                self.buffer = Some(InstanceBuffer::new(gpu, &instances));
            }
        }
    }
}

pub(crate) struct CallableType {
    pub config: ComponentConfig,
    pub callbacks: ComponentCallbacks,
    pub last_update: Option<Instant>,
}

impl CallableType {
    pub fn new<C: ComponentController>() -> CallableType {
        Self {
            last_update: match &C::CONFIG.update {
                crate::UpdateOperation::AfterDuration(_) => Some(Instant::now()),
                _ => None,
            },
            callbacks: ComponentCallbacks::new::<C>(),
            config: C::CONFIG,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentType {
    index: TypeIndex,
    type_id: ComponentTypeId,
    config: ComponentConfig,
    pub groups: Arena<ComponentTypeGroup>,
    #[cfg(feature = "physics")]
    pub world_changes: WorldChanges,
}

impl ComponentType {
    pub(crate) fn with_config<C: ComponentController>(
        config: ComponentConfig,
        index: TypeIndex,
        group_structure: &Arena<Group>,
    ) -> Self {
        let groups = Arena {
            items: group_structure
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
            generation: group_structure.generation,
            free_list_head: group_structure.free_list_head,
            len: group_structure.len(),
        };
        Self {
            index,
            groups,
            config,
            type_id: C::IDENTIFIER,
            #[cfg(feature = "physics")]
            world_changes: WorldChanges::new(),
        }
    }

    pub(crate) fn buffer(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        active: &[GroupHandle],
        gpu: &Gpu,
    ) {
        if self.config.buffer == BufferOperation::Never {
            return;
        }

        let every_frame = self.config.buffer == BufferOperation::EveryFrame;
        for index in active {
            let group = &mut self.groups[index.0];
            group.buffer(
                #[cfg(feature = "physics")]
                world,
                every_frame,
                gpu,
            );
        }
    }

    pub(crate) fn add_group(&mut self) -> GroupHandle {
        let index = self.groups.insert(ComponentTypeGroup::new());
        return GroupHandle(index);
    }

    pub(crate) fn remove_group(&mut self, handle: GroupHandle) {
        let _group = self.groups.remove(handle.0).unwrap();
        #[cfg(feature = "physics")]
        for component in _group.components {
            self.world_changes.register_remove(&component);
        }
    }

    pub fn component_type_id(&self) -> ComponentTypeId {
        self.type_id
    }

    #[cfg(feature = "physics")]
    pub fn apply_world_mapping(&mut self, world: &mut World) {
        self.world_changes.apply(world)
    }

    pub fn each<C: ComponentController>(&self, groups: &[GroupHandle], mut each: impl FnMut(&C)) {
        for group in groups {
            if let Some(group) = self.groups.get(group.0) {
                for (_, component) in &group.components {
                    (each)(component.downcast_ref::<C>().unwrap());
                }
            }
        }
    }

    pub fn each_mut<C: ComponentController>(
        &mut self,
        groups: &[GroupHandle],
        mut each: impl FnMut(&mut C),
    ) {
        for group in groups {
            if let Some(group) = self.groups.get_mut(group.0) {
                for (_, component) in &mut group.components {
                    (each)(component.downcast_mut::<C>().unwrap());
                }
            }
        }
    }

    pub fn retain<C: ComponentController>(
        &mut self,
        groups: &[GroupHandle],
        mut keep: impl FnMut(&mut C) -> bool,
    ) {
        for group in groups {
            if let Some(group) = self.groups.get_mut(group.0) {
                group.components.retain(|_, component| {
                    let component = component.downcast_mut::<C>().unwrap();
                    if keep(component) {
                        true
                    } else {
                        #[cfg(feature = "physics")]
                        self.world_changes.register_remove(component);
                        false
                    }
                });
            }
        }
    }

    pub fn index<C: ComponentController>(&self, group: GroupHandle, index: usize) -> Option<&C> {
        if let Some(group) = self.groups.get(group.0) {
            if let Some(component) = group.components.get_unknown_gen(index) {
                return component.downcast_ref::<C>();
            }
        }
        return None;
    }

    pub fn index_mut<C: ComponentController>(
        &mut self,
        group: GroupHandle,
        index: usize,
    ) -> Option<&mut C> {
        if let Some(group) = self.groups.get_mut(group.0) {
            if let Some(component) = group.components.get_unknown_gen_mut(index) {
                return component.downcast_mut::<C>();
            }
        }
        return None;
    }

    pub fn get<C: ComponentController>(&self, handle: ComponentHandle) -> Option<&C> {
        if let Some(group) = self.groups.get(handle.group_index().0) {
            if let Some(component) = group.components.get(handle.component_index().0) {
                return component.downcast_ref::<C>();
            }
        }
        return None;
    }

    pub fn get_mut<C: ComponentController>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
        if let Some(group) = self.groups.get_mut(handle.group_index().0) {
            if let Some(component) = group.components.get_mut(handle.component_index().0) {
                return component.downcast_mut::<C>();
            }
        }
        return None;
    }

    pub fn get2_mut<C1: ComponentController, C2: ComponentController>(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C1>, Option<&mut C2>) {
        let mut c1 = None;
        let mut c2 = None;
        if handle1.group_index() == handle2.group_index() {
            if let Some(group) = self.groups.get_mut(handle1.group_index().0) {
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
            let (group1, group2) = self
                .groups
                .get2_mut(handle1.group_index().0, handle2.group_index().0);
            if let Some(group) = group1 {
                if let Some(component) = group.components.get_mut(handle1.component_index().0) {
                    c1 = component.downcast_mut::<C1>();
                }
            }

            if let Some(group) = group2 {
                if let Some(component) = group.components.get_mut(handle2.component_index().0) {
                    c2 = component.downcast_mut::<C2>();
                }
            }
        }
        return (c1, c2);
    }

    pub fn get2_mut_boxed(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut BoxedComponent>, Option<&mut BoxedComponent>) {
        let mut c1 = None;
        let mut c2 = None;
        if handle1.group_index() == handle2.group_index() {
            if let Some(group) = self.groups.get_mut(handle1.group_index().0) {
                (c1, c2) = group
                    .components
                    .get2_mut(handle1.component_index().0, handle2.component_index().0);
            }
        } else {
            let (group1, group2) = self
                .groups
                .get2_mut(handle1.group_index().0, handle2.group_index().0);
            if let Some(group) = group1 {
                c1 = group.components.get_mut(handle1.component_index().0);
            }

            if let Some(group) = group2 {
                c2 = group.components.get_mut(handle2.component_index().0);
            }
        }
        return (c1, c2);
    }

    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        if let Some(group) = self.groups.get(handle.group_index().0) {
            return group.components.get(handle.component_index().0);
        }
        return None;
    }

    pub fn get_boxed_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index().0) {
            return group.components.get_mut(handle.component_index().0);
        }
        return None;
    }

    pub fn remove<C: ComponentController>(&mut self, handle: ComponentHandle) -> Option<C> {
        if let Some(group) = self.groups.get_mut(handle.group_index().0) {
            if let Some(component) = group.components.remove(handle.component_index().0) {
                #[cfg(feature = "physics")]
                self.world_changes.register_remove(&component);
                return component.downcast::<C>().ok().and_then(|b| Some(*b));
            }
        }
        return None;
    }

    pub fn remove_boxed(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index().0) {
            if let Some(component) = group.components.remove(handle.component_index().0) {
                #[cfg(feature = "physics")]
                self.world_changes.register_remove(&component);
                return Some(component);
            }
        }
        return None;
    }

    pub fn remove_all<C: ComponentController>(
        &mut self,
        groups: &[GroupHandle],
    ) -> Vec<(GroupHandle, Vec<C>)> {
        let mut result = Vec::with_capacity(groups.len());
        for group_handle in groups {
            if let Some(group) = self.groups.get_mut(group_handle.0) {
                let components = std::mem::replace(&mut group.components, Default::default());
                let mut casted = Vec::with_capacity(components.len());
                for component in components {
                    #[cfg(feature = "physics")]
                    self.world_changes.register_remove(&component);
                    casted.push(*component.downcast::<C>().ok().unwrap())
                }
                result.push((*group_handle, casted));
            }
        }
        return result;
    }

    pub fn add<C: ComponentDerive + ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        assert_eq!(C::IDENTIFIER, self.type_id);
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.components.insert_with(|idx| {
            handle = ComponentHandle::new(ComponentIndex(idx), self.index, group_handle);
            #[cfg(feature = "physics")]
            self.world_changes.register_add(handle, &component);
            Box::new(component)
        });
        return handle;
    }

    pub fn add_many<C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let components = components.into_iter();
        let mut handles = Vec::with_capacity(components.size_hint().0);
        if let Some(group) = self.groups.get_mut(group_handle.0) {
            for component in components {
                group.components.insert_with(|idx| {
                    let handle =
                        ComponentHandle::new(ComponentIndex(idx), self.index, group_handle);
                    #[cfg(feature = "physics")]
                    self.world_changes.register_add(handle, &component);
                    handles.push(handle);
                    Box::new(component)
                });
            }
        }
        return handles;
    }

    pub fn add_with<C: ComponentDerive + ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        assert_eq!(C::IDENTIFIER, self.type_id);
        let group = &mut self.groups[group_handle.0];
        let mut handle = Default::default();
        group.components.insert_with(|idx| {
            handle = ComponentHandle::new(ComponentIndex(idx), self.index, group_handle);
            let component = create(handle);
            #[cfg(feature = "physics")]
            self.world_changes.register_add(handle, &component);
            Box::new(component)
        });
        return handle;
    }

    pub fn force_buffer(&mut self, groups: &[GroupHandle]) {
        for group in groups {
            if let Some(group) = self.groups.get_mut(group.0) {
                group.force_buffer = true;
            }
        }
    }

    pub fn len(&self, groups: &[GroupHandle]) -> usize {
        let mut len = 0;
        for group in groups {
            if let Some(group) = self.groups.get(group.0) {
                len += group.components.len();
            }
        }
        return len;
    }

    pub fn iter<'a, C: ComponentController>(
        &'a self,
        groups: &[GroupHandle],
    ) -> impl DoubleEndedIterator<Item = &'a C> {
        let mut iters = Vec::with_capacity(groups.len());
        for group in groups {
            if let Some(group) = self.groups.get(group.0) {
                if !group.components.is_empty() {
                    iters.push(
                        group
                            .components
                            .iter()
                            .map(|(_, c)| c.downcast_ref::<C>().unwrap()),
                    );
                }
            }
        }
        return iters.into_iter().flatten();
    }

    pub fn iter_with_handles<'a, C: ComponentController>(
        &'a self,
        groups: &'a [GroupHandle],
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
        let mut iters = Vec::with_capacity(groups.len());
        for group_handle in groups {
            if let Some(group) = self.groups.get(group_handle.0) {
                if !group.components.is_empty() {
                    iters.push(group.components.iter().map(|(idx, c)| {
                        (
                            ComponentHandle::new(ComponentIndex(idx), self.index, *group_handle),
                            c.downcast_ref::<C>().unwrap(),
                        )
                    }));
                }
            }
        }
        return iters.into_iter().flatten();
    }

    pub fn iter_mut_with_handles<'a, C: ComponentController>(
        &'a mut self,
        groups: &'a [GroupHandle],
        check: bool,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
        let mut iters = Vec::with_capacity(groups.len());
        let ptr: *mut Arena<ComponentTypeGroup> = &mut self.groups as *mut _;
        if check && groups.len() > 1 {
            for (index, value) in groups.iter().enumerate() {
                for other in groups.iter().skip(index + 1) {
                    assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                }
            }
        }
        unsafe {
            for group_handle in groups {
                if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                    let type_index = &self.index;

                    iters.push(group.components.iter_mut().map(move |(idx, c)| {
                        (
                            ComponentHandle::new(ComponentIndex(idx), *type_index, *group_handle),
                            c.downcast_mut::<C>().unwrap(),
                        )
                    }));
                };
            }
        }

        return iters.into_iter().flatten();
    }

    pub fn iter_mut<'a, C: ComponentController>(
        &'a mut self,
        groups: &[GroupHandle],
        check: bool,
    ) -> impl DoubleEndedIterator<Item = &'a mut C> {
        if check && groups.len() > 1 {
            for (index, value) in groups.iter().enumerate() {
                for other in groups.iter().skip(index + 1) {
                    assert_ne!(value.0.index(), other.0.index(), "Duplicate GroupHandle!");
                }
            }
        }
        let mut iters = Vec::with_capacity(groups.len());
        let ptr: *mut Arena<ComponentTypeGroup> = &mut self.groups as *mut _;
        unsafe {
            for group_handle in groups {
                if let Some(group) = (&mut *ptr).get_mut(group_handle.0) {
                    iters.push(
                        group
                            .components
                            .iter_mut()
                            .map(|(_, c)| c.downcast_mut::<C>().unwrap()),
                    );
                };
            }
        }

        return iters.into_iter().flatten();
    }

    pub fn iter_render<C: ComponentController>(
        &self,
        groups: &[GroupHandle],
    ) -> impl DoubleEndedIterator<
        Item = (
            &InstanceBuffer,
            impl DoubleEndedIterator<Item = (InstanceIndex, &C)> + Clone,
        ),
    > {
        let mut iters = Vec::with_capacity(groups.len());
        for group in groups {
            if let Some(group) = self.groups.get(group.0) {
                if !group.components.is_empty() {
                    iters.push((
                        group.buffer.as_ref().expect(
                            "This component's buffer is either not initialized or disabled.",
                        ),
                        group.components.iter().enumerate().map(|(i, (_, c))| {
                            (InstanceIndex::new(i as u32), c.downcast_ref::<C>().unwrap())
                        }),
                    ));
                }
            }
        }
        return iters.into_iter();
    }
}
