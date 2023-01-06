use crate::{
    ArenaIndex, ArenaIter, ArenaIterMut, ComponentConfig, ComponentController, ComponentType,
    DynamicComponent,
};
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArenaPath {
    pub group_index: ArenaIndex,
    pub type_index: ArenaIndex,
}

#[derive(Debug, Clone)]
pub(crate) struct ComponentCluster {
    paths: Vec<ArenaPath>,
    config: &'static ComponentConfig,
}

impl ComponentCluster {
    pub fn new(path: ArenaPath, config: &'static ComponentConfig) -> Self {
        Self {
            paths: vec![path],
            config: config,
        }
    }

    pub fn clear(&mut self) {
        self.paths.clear();
    }

    pub fn add(&mut self, path: ArenaPath) {
        self.paths.push(path);
    }

    // Getters
    #[inline]
    pub const fn config(&self) -> &'static ComponentConfig {
        self.config
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &Vec<ArenaPath> {
        &self.paths
    }
}

/// A set of components that includes all components of a specific type from a variety of
/// [ComponentGroups](crate::ComponentGroup).
/// A [ComponentSet] can be retrieved from the [Context](crate::Context) with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut).
pub struct ComponentSet<'a, T: ComponentController> {
    pub(crate) types: Vec<&'a ComponentType>,
    pub(crate) len: usize,
    _type: PhantomData<T>,
}

impl<'a, T: ComponentController> ComponentSet<'a, T> {
    pub(crate) fn new(types: Vec<&'a ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            _type: PhantomData::<T>,
        }
    }

    /// Get the amount of components in the set.
    pub fn len(&self) -> usize {
        return self.len;
    }

    /// Check if this set is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate over this set
    pub fn iter(&'a self) -> ComponentIter<'a, T> {
        return ComponentIter::new(self);
    }

    // Getters

    /// Get the config from the type of the controller.
    #[inline]
    pub fn config(&self) -> &'static ComponentConfig {
        T::config()
    }
}

impl<'a, T> IntoIterator for &'a ComponentSet<'a, T>
where
    T: ComponentController,
{
    type Item = &'a T;
    type IntoIter = ComponentIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        return self.iter();
    }
}

/// Iterator over a [ComponentSet], which holds components from multiple [ComponentGroups](crate::ComponentGroup).
pub struct ComponentIter<'a, T>
where
    T: ComponentController,
{
    iters: Vec<ArenaIter<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    _type: PhantomData<T>,
}

impl<'a, T> ComponentIter<'a, T>
where
    T: ComponentController,
{
    pub(crate) fn new(set: &'a ComponentSet<T>) -> ComponentIter<'a, T> {
        let mut iters = Vec::with_capacity(set.types.len());
        for t in &set.types {
            iters.push(t.iter());
        }
        ComponentIter {
            iters,
            iter_index: 0,
            len: set.len(),
            _type: PhantomData::<T>,
        }
    }
}

impl<'a, T> ExactSizeIterator for ComponentIter<'a, T>
where
    T: ComponentController,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> Iterator for ComponentIter<'a, T>
where
    T: ComponentController,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            if let Some((_, c)) = iter.next() {
                return c.as_ref().downcast_ref::<T>();
            } else {
                self.iter_index += 1;
                return self.next();
            }
        }
        return None;
    }
}

impl<'a, T> DoubleEndedIterator for ComponentIter<'a, T>
where
    T: ComponentController,
{
    fn next_back(&mut self) -> Option<&'a T> {
        let len = self.iters.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            if let Some((_, c)) = iter.next_back() {
                return c.as_ref().downcast_ref::<T>();
            } else {
                self.iter_index += 1;
                return self.next_back();
            }
        }
        return None;
    }
}

/// A set of components that includes all components of a specific type from a variety of
/// [ComponentGroups](crate::ComponentGroup).
/// A [ComponentSet] can be retrieved from the [Context](crate::Context) with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut).
pub struct ComponentSetMut<'a, T: ComponentController> {
    pub(crate) types: Vec<&'a mut ComponentType>,
    pub(crate) len: usize,
    _type: PhantomData<T>,
}

impl<'a, T: ComponentController> ComponentSetMut<'a, T> {
    pub(crate) fn new(types: Vec<&'a mut ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            _type: PhantomData::<T>,
        }
    }

    /// Get the amount of components in the set.
    pub fn len(&self) -> usize {
        return self.len;
    }

    /// Check if this set is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate over this set
    pub fn iter(&'a mut self) -> ComponentIterMut<'a, T> {
        return ComponentIterMut::new(self);
    }

    // Getters

    /// Get the config from the type of the controller.
    #[inline]
    pub fn config(&self) -> &'static ComponentConfig {
        T::config()
    }
}

impl<'a, T> IntoIterator for &'a mut ComponentSetMut<'a, T>
where
    T: ComponentController,
{
    type Item = &'a mut T;
    type IntoIter = ComponentIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        return self.iter();
    }
}

/// Iterator over a [ComponentSetMut], which holds components from multiple [ComponentGroups](crate::ComponentGroup).
pub struct ComponentIterMut<'a, T>
where
    T: ComponentController,
{
    iters: Vec<ArenaIterMut<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    _type: PhantomData<T>,
}

impl<'a, T> ComponentIterMut<'a, T>
where
    T: ComponentController,
{
    pub(crate) fn new(set: &'a mut ComponentSetMut<T>) -> ComponentIterMut<'a, T> {
        let mut iters = Vec::with_capacity(set.types.len());
        let len = set.len();
        for t in &mut set.types {
            iters.push(t.iter_mut());
        }
        ComponentIterMut {
            iters,
            iter_index: 0,
            len,
            _type: PhantomData::<T>,
        }
    }
}

impl<'a, T> ExactSizeIterator for ComponentIterMut<'a, T>
where
    T: ComponentController,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> Iterator for ComponentIterMut<'a, T>
where
    T: ComponentController,
{
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            if let Some((_, c)) = iter.next() {
                return c.as_mut().downcast_mut::<T>();
            } else {
                self.iter_index += 1;
                return self.next();
            }
        }
        return None;
    }
}

impl<'a, T> DoubleEndedIterator for ComponentIterMut<'a, T>
where
    T: ComponentController,
{
    fn next_back(&mut self) -> Option<&'a mut T> {
        let len = self.iters.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            if let Some((_, c)) = iter.next_back() {
                return c.as_mut().downcast_mut::<T>();
            } else {
                self.iter_index += 1;
                return self.next_back();
            }
        }
        return None;
    }
}
