use crate::{
    data::arena::{ArenaIter, ArenaIterMut},
    Arena, ArenaIndex, BoxedComponent, BufferOperation, ComponentConfig, ComponentController,
    ComponentDerive, ComponentHandle, Gpu, InstanceBuffer, Matrix,
};

#[cfg(feature = "physics")]
use crate::physics::RcWorld;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentTypeId {
    id: u32,
}

impl ComponentTypeId {
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

pub trait ComponentIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ComponentTypeId;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentType {
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    components: Arena<BoxedComponent>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    buffer: Option<InstanceBuffer>,

    type_id: ComponentTypeId,
    last_len: usize,
    force_buffer: bool,
    config: ComponentConfig,
}

impl ComponentType {
    pub fn new<C: ComponentController>() -> Self {
        return Self::from_arena::<C>(Arena::new());
    }

    pub fn from_arena<C: ComponentController>(components: Arena<BoxedComponent>) -> Self {
        Self {
            components,
            buffer: None,
            force_buffer: false,
            last_len: 0,
            config: C::CONFIG,
            type_id: C::IDENTIFIER,
        }
    }

    pub fn buffer_data(&mut self, gpu: &Gpu) {
        if self.config.buffer == BufferOperation::Never {
            return;
        }

        let new_len = self.components.len();
        if new_len != self.last_len {
            // We have to resize the buffer
            let data = self.data();
            self.last_len = new_len;
            self.buffer = Some(InstanceBuffer::new(gpu, &data[..]));
        } else if self.config.buffer == BufferOperation::EveryFrame || self.force_buffer {
            let data = self.data();
            self.force_buffer = false;
            if let Some(buffer) = &mut self.buffer {
                buffer.write(gpu, &data[..]);
            } else {
                self.buffer = Some(InstanceBuffer::new(gpu, &data));
            }
        }
    }

    fn data(&mut self) -> Vec<Matrix> {
        self.components
            .iter_mut()
            .map(|(_, component)| component.base().matrix())
            .collect::<Vec<Matrix>>()
    }

    pub fn add<C: ComponentDerive + ComponentController>(
        &mut self,
        mut handle: ComponentHandle,
        #[cfg(feature = "physics")]
        world: RcWorld,
        mut component: C,
    ) -> ComponentHandle {
        self.components.insert_with(|idx| {
            handle.component_index = idx;
            component.base_mut().init(handle);
            #[cfg(feature = "physics")]
            if component.base().is_body() {
                component
                    .base_mut()
                    .add_to_world(C::IDENTIFIER, world)
            }
            Box::new(component)
        });
        return handle;
    }

    #[cfg(feature = "serde")]
    pub fn serialize_components<C: ComponentDerive + serde::Serialize>(
        &self,
    ) -> Vec<Option<(u32, Vec<u8>)>> {
        return self.components.serialize_components::<C>();
    }

    pub fn remove(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
        self.components.remove(handle.component_index())
    }

    pub fn buffer(&self) -> Option<&InstanceBuffer> {
        self.buffer.as_ref()
    }

    pub const fn config(&self) -> &ComponentConfig {
        &self.config
    }

    pub const fn type_id(&self) -> ComponentTypeId {
        self.type_id
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn index(&self, index: usize) -> Option<&BoxedComponent> {
        self.components.get_unknown_gen(index)
    }

    pub fn index_mut(&mut self, index: usize) -> Option<&mut BoxedComponent> {
        self.components.get_unknown_gen_mut(index)
    }

    pub fn component(&self, index: ArenaIndex) -> Option<&BoxedComponent> {
        self.components.get(index)
    }

    pub fn component_mut(&mut self, index: ArenaIndex) -> Option<&mut BoxedComponent> {
        self.components.get_mut(index)
    }

    pub fn iter(&self) -> ArenaIter<BoxedComponent> {
        return self.components.iter();
    }

    pub fn iter_mut(&mut self) -> ArenaIterMut<BoxedComponent> {
        return self.components.iter_mut();
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn set_force_buffer(&mut self, force_buffer: bool) {
        self.force_buffer = force_buffer;
    }
}

impl<'a> IntoIterator for &'a ComponentType {
    type Item = (ArenaIndex, &'a BoxedComponent);
    type IntoIter = ArenaIter<'a, BoxedComponent>;
    fn into_iter(self) -> Self::IntoIter {
        self.components.iter()
    }
}

impl<'a> IntoIterator for &'a mut ComponentType {
    type Item = (ArenaIndex, &'a mut BoxedComponent);
    type IntoIter = ArenaIterMut<'a, BoxedComponent>;
    fn into_iter(self) -> Self::IntoIter {
        self.components.iter_mut()
    }
}
