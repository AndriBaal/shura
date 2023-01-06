#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    data::arena::{ArenaEntry, ArenaIter, ArenaIterMut},
    Arena, ArenaIndex, ComponentConfig, ComponentHandle, DynamicComponent, Gpu, InstanceBuffer,
    Matrix, RenderOperation,
};
use std::any::TypeId;

pub(crate) struct ComponentType {
    components: Arena<DynamicComponent>,
    buffer: Option<InstanceBuffer>,
    last_len: usize,
    force_rewrite_buffer: bool,

    config: &'static ComponentConfig,
    type_id: TypeId,
}

impl ComponentType {
    pub fn new(type_id: TypeId, config: &'static ComponentConfig) -> Self {
        Self {
            components: Arena::new(),
            buffer: None,
            force_rewrite_buffer: true,
            last_len: usize::MAX, // Max value to force a rewrite on the first cycle when the buffer is uninitialized
            config: config,
            type_id: type_id,
        }
    }

    // #[inline(always)]
    // pub fn scale(&mut self, window_size: Dimension<u32>) {
    //     if self.config.does_move && self.config.relative_position != RelativeScale::None {
    //         for (_, component) in &mut self.components {
    //             component.scale_relative(self.config.relative_position, window_size);
    //         }
    //     }
    // }

    #[inline(always)]
    pub fn buffer_data(&mut self, gpu: &Gpu, #[cfg(feature = "physics")] world: &World) {
        match self.config.render {
            RenderOperation::None => return,
            _ => {}
        }

        let new_len = self.components.len();
        if new_len != self.last_len {
            // We have to resize the buffer
            let data = self.data(
                #[cfg(feature = "physics")]
                world,
            );
            self.last_len = new_len;
            self.buffer = Some(InstanceBuffer::new(gpu, &data[..]));
        } else if self.config.does_move || self.force_rewrite_buffer {
            let data = self.data(
                #[cfg(feature = "physics")]
                world,
            );
            self.force_rewrite_buffer = false;
            self.buffer.as_mut().unwrap().write(gpu, &data[..]);
        }
    }

    #[inline(always)]
    fn data(&mut self, #[cfg(feature = "physics")] world: &World) -> Vec<Matrix> {
        self.components
            .iter_mut()
            .map(|(_, controller)| {
                controller.inner().matrix(
                    #[cfg(feature = "physics")]
                    world,
                )
            })
            .collect::<Vec<Matrix>>()
    }

    #[inline(always)]
    pub fn add(&mut self, component: DynamicComponent) -> ArenaIndex {
        return self.components.insert(component);
    }

    #[inline(always)]
    pub fn remove(&mut self, handle: &ComponentHandle) -> Option<DynamicComponent> {
        self.components.remove(handle.component_index())
    }

    pub fn buffer(&self) -> &InstanceBuffer {
        self.buffer.as_ref().unwrap()
    }

    // Getters

    #[inline]
    pub const fn config(&self) -> &'static ComponentConfig {
        self.config
    }

    #[inline]
    pub const fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.components = Arena::new();
    }

    #[inline]
    pub fn component(&self, index: ArenaIndex) -> Option<&DynamicComponent> {
        self.components.get(index)
    }

    #[inline]
    pub fn component_mut(&mut self, index: ArenaIndex) -> Option<&mut DynamicComponent> {
        self.components.get_mut(index)
    }

    #[inline]
    pub fn iter(&self) -> ArenaIter<DynamicComponent> {
        return self.components.iter();
    }

    #[inline]
    pub fn iter_mut(&mut self) -> ArenaIterMut<DynamicComponent> {
        return self.components.iter_mut();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    #[inline]
    pub fn borrow_component(&mut self, index: usize) -> Option<ArenaEntry<DynamicComponent>> {
        return self.components.borrow_value(index);
    }

    #[inline]
    pub fn return_component(&mut self, index: usize, component: ArenaEntry<DynamicComponent>) {
        self.components.return_value(index, component);
    }

    #[inline]
    pub fn not_return_component(&mut self, index: usize) {
        self.components.not_return_value(index);
    }

    // Setters
    #[inline]
    pub fn set_force_rewrite_buffer(&mut self, force_rewrite_buffer: bool) {
        self.force_rewrite_buffer = force_rewrite_buffer;
    }
}

impl<'a> IntoIterator for &'a ComponentType {
    type Item = (ArenaIndex, &'a DynamicComponent);
    type IntoIter = ArenaIter<'a, DynamicComponent>;
    fn into_iter(self) -> Self::IntoIter {
        self.components.iter()
    }
}

impl<'a> IntoIterator for &'a mut ComponentType {
    type Item = (ArenaIndex, &'a mut DynamicComponent);
    type IntoIter = ArenaIterMut<'a, DynamicComponent>;
    fn into_iter(self) -> Self::IntoIter {
        self.components.iter_mut()
    }
}
