use instant::Instant;

use crate::{
    data::arena::ArenaEntry, Arena, ArenaIndex, BoxedComponent, BufferOperation,
    ComponentCallbacks, ComponentConfig, ComponentController, ComponentDerive, ComponentGroup,
    ComponentHandle, ComponentIndex, Gpu, GroupHandle, InstanceBuffer, Matrix, TypeIndex,
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
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

/// Trait to identify a struct that derives from  the [Component](crate::Component) macro using
/// a [ComponentTypeId]
pub trait ComponentIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ComponentTypeId;
}

struct ComponentTypeGroup {
    pub components: Arena<BoxedComponent>,
    pub force_buffer: bool,
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

    fn data(&mut self) -> Vec<Matrix> {
        self.components
            .iter_mut()
            .map(|(_, component)| component.base().matrix())
            .collect::<Vec<Matrix>>()
    }

    pub fn buffer(&mut self, every_frame: bool, gpu: &Gpu) {
        let new_len = self.components.len();
        if new_len != self.last_len {
            // We have to resize the buffer
            let data = self.data();
            self.last_len = new_len;
            self.buffer = Some(InstanceBuffer::new(gpu, &data[..]));
        } else if every_frame || self.force_buffer {
            let data = self.data();
            self.force_buffer = false;
            if let Some(buffer) = &mut self.buffer {
                buffer.write(gpu, &data[..]);
            } else {
                self.buffer = Some(InstanceBuffer::new(gpu, &data));
            }
        }
    }
}

pub(crate) struct CallableType {
    pub config: ComponentConfig,
    pub callbacks: ComponentCallbacks,
    pub last_update: Option<Instant>
}

impl CallableType {
    pub fn new(ty: &ComponentType) -> CallableType {
        Self {
            last_update: match &ty.config.update {
                crate::UpdateOperation::AfterDuration(_) => Some(Instant::now()),
                _ => None,
            },
            callbacks: ty.callbacks,
            config: ty.config,
        }    
    }
}

// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]group
pub(crate) struct ComponentType {
    groups: Arena<ComponentTypeGroup>,
    index: TypeIndex,
    type_id: ComponentTypeId,
    config: ComponentConfig,
    callbacks: ComponentCallbacks
}

impl ComponentType {
    pub fn new<C: ComponentController>(
        index: TypeIndex,
        group_structure: &Arena<ComponentGroup>,
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
            config: C::CONFIG,
            type_id: C::IDENTIFIER,
            callbacks: ComponentCallbacks::new::<C>(),
        }
    }

    pub fn buffer(&mut self, active: &[GroupHandle], gpu: &Gpu) {
        if self.config.buffer == BufferOperation::Never {
            return;
        }

        let every_frame = self.config.buffer == BufferOperation::EveryFrame;
        for index in active {
            let group = &mut self.groups[index.0];
            group.buffer(self.config.buffer == BufferOperation::EveryFrame, gpu);
        }
    }

    pub fn set_force_buffer(&mut self, force_buffer: bool) {
        for (_, group) in &mut self.groups {
            group.force_buffer = force_buffer;
        }
    }

    pub fn add_group(&mut self) -> GroupHandle {
        let index = self.groups.insert(ComponentTypeGroup::new());
        return GroupHandle(index);
    }

    pub fn remove_group(&mut self, handle: GroupHandle) {
        self.groups.remove(handle.0);
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
                    keep(component)
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
    pub fn remove_component<C: ComponentController>(
        &mut self,
        handle: ComponentHandle,
    ) -> Option<Box<C>> {
        if let Some(group) = self.groups.get_mut(handle.group_index().0) {
            if let Some(component) = group.components.remove(handle.component_index().0) {
                return component.downcast::<C>().ok();
            }
        }
        return None;
    }
    pub fn add_component<C: ComponentDerive + ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        assert_eq!(C::IDENTIFIER, self.type_id);
        let group = &mut self.groups[group_handle.0];
        let mut handle;
        group.components.insert_with(|idx| {
            handle = ComponentHandle::new(ComponentIndex(idx), self.index, group_handle);
            component.base_mut().init(handle);
            Box::new(component)
        });
        return handle;
    }
    pub fn add_components<I, C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        components: impl Iterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let mut handles = Vec::with_capacity(components.size_hint().0);
        if let Some(group) = self.groups.get_mut(group_handle.0) {
            for component in components {
                group.components.insert_with(|idx| {
                    let handle =
                        ComponentHandle::new(ComponentIndex(idx), self.index, group_handle);
                    component.base_mut().init(handle);
                    handles.push(handle);
                    Box::new(component)
                });
            }
        }
        return handles;
    }

    pub fn force_buffer(&mut self, groups: &[GroupHandle]) {
        for group in groups {
            if let Some(group) = self.groups.get(group.0) {
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

    // pub fn get(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
    //     if let Some(group) = self.groups.get(handle.group_index()) {
    //         return group.components.get(handle.component_index());
    //     }
    //     return None;
    // }

    // pub fn get_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
    //     if let Some(group) = self.groups.get(handle.group_index()) {
    //         return group.components.get_mut(handle.component_index());
    //     }
    //     return None;
    // }

    // pub fn add<C: ComponentDerive + ComponentController>(
    //     &mut self,
    //     mut handle: ComponentHandle,
    //     mut component: C,
    //     #[cfg(feature = "physics")] world: RcWorld,
    // ) -> ComponentHandle {
    //     assert_eq!(C::IDENTIFIER, self.type_id);
    //     self.components.insert_with(|idx| {
    //         handle.component_index = idx;
    //         component.base_mut().init(handle);
    //         #[cfg(feature = "physics")]
    //         if component.base().is_body() {
    //             component.base_mut().add_to_world(C::IDENTIFIER, world)
    //         }
    //         Box::new(component)
    //     });
    //     return handle;
    // }

    // pub fn add<C: ComponentDerive + ComponentController>(
    //     &mut self,
    //     mut handle: ComponentHandle,
    //     #[cfg(feature = "physics")] world: RcWorld,
    //     mut component: C,
    // ) -> ComponentHandle {
    //     self.components.insert_with(|idx| {
    //         handle.component_index = idx;
    //         component.base_mut().init(handle);
    //         #[cfg(feature = "physics")]
    //         if component.base().is_body() {
    //             component.base_mut().add_to_world(C::IDENTIFIER, world)
    //         }
    //         Box::new(component)
    //     });
    //     return handle;
    // }

    // #[cfg(feature = "serde")]
    // pub fn serialize_components<C: ComponentDerive + serde::Serialize>(
    //     &self,
    // ) -> Vec<Option<(u32, Vec<u8>)>> {
    //     return self.components.serialize_components::<C>();
    // }

    // pub fn remove(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
    //     self.components.remove(handle.component_index())
    // }

    // pub fn buffer(&self) -> Option<&InstanceBuffer> {
    //     self.buffer.as_ref()
    // }

    // pub const fn config(&self) -> &ComponentConfig {
    //     &self.config
    // }

    // pub const fn type_id(&self) -> ComponentTypeId {
    //     self.type_id
    // }

    // pub fn index(&self, index: usize) -> Option<&BoxedComponent> {
    //     self.components.get_unknown_gen(index)
    // }

    // pub fn index_mut(&mut self, index: usize) -> Option<&mut BoxedComponent> {
    //     self.components.get_unknown_gen_mut(index)
    // }

    // pub fn component(&self, index: ArenaIndex) -> Option<&BoxedComponent> {
    //     self.components.get(index)
    // }

    // pub fn component_mut(&mut self, index: ArenaIndex) -> Option<&mut BoxedComponent> {
    //     self.components.get_mut(index)
    // }

    // pub fn iter(&self) -> ArenaIter<BoxedComponent> {
    //     return self.components.iter();
    // }

    // pub fn iter_mut(&mut self) -> ArenaIterMut<BoxedComponent> {
    //     return self.components.iter_mut();
    // }

    // pub fn len(&self) -> usize {
    //     self.components.len()
    // }

    // pub fn set_force_buffer(&mut self, force_buffer: bool) {
    //     self.force_buffer = force_buffer;
    // }
}
