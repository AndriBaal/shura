#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    data::arena::{ArenaEntry, ArenaIter, ArenaIterMut},
    Arena, ArenaIndex, ComponentConfig, ComponentHandle, DynamicComponent, Gpu, InstanceBuffer,
    Matrix, RenderOperation, ComponentController,
};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentType {
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default))]
    components: Arena<DynamicComponent>,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default))]
    buffer: Option<InstanceBuffer>,

    name: &'static str,
    last_len: usize,
    force_rewrite_buffer: bool,
    config: ComponentConfig,
}

impl ComponentType {
    pub fn new<C: ComponentController>(component: C) -> (ArenaIndex, Self) {
        let mut components: Arena<DynamicComponent> = Arena::new();
        let component_index = components.insert(Box::new(component));
        (component_index, Self {
            components,
            buffer: None,
            force_rewrite_buffer: false,
            last_len: 0,
            config: C::config(),
            name: C::name(),
        })
    }

    #[inline(always)]
    pub fn buffer_data(&mut self, gpu: &Gpu, #[cfg(feature = "physics")] world: &World) {
        if self.config.render == RenderOperation::None {
            return;
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
            .map(|(_, component)| {
                component.inner().matrix(
                    #[cfg(feature = "physics")]
                    world,
                )
            })
            .collect::<Vec<Matrix>>()
    }

    #[inline(always)]
    pub fn add<C: ComponentController>(&mut self, component: C) -> ArenaIndex {
        return self.components.insert(Box::new(component));
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
    pub const fn config(&self) -> &ComponentConfig {
        &self.config
    }

    #[inline]
    pub const fn name(&self) -> &'static str {
        self.name
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
