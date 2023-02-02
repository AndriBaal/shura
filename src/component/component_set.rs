use instant::Instant;

use crate::{
    ArenaIndex, ArenaIter, ArenaIterMut, ComponentConfig, ComponentController, ComponentType,
    DynamicComponent,
};
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ArenaPath {
    pub group_index: ArenaIndex,
    pub type_index: ArenaIndex,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentCluster {
    paths: Vec<ArenaPath>,
    config: ComponentConfig,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default))]
    last_update: Option<Instant>,
}

impl ComponentCluster {
    pub fn new(path: ArenaPath, config: ComponentConfig) -> Self {
        Self {
            paths: vec![path],
            last_update: match &config.update {
                crate::UpdateOperation::AfterDuration(_) => Some(Instant::now()),
                _ => None,
            },
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
    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    #[inline]
    pub fn set_last_update(&mut self, now: Instant) {
        self.last_update = Some(now);
    }

    #[inline]
    pub const fn config(&self) -> &ComponentConfig {
        &self.config
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
pub struct ComponentSet<'a, C: ComponentController> {
    pub(crate) types: Vec<&'a ComponentType>,
    pub(crate) len: usize,
    _type: PhantomData<C>,
}

impl<'a, C: ComponentController> ComponentSet<'a, C> {
    pub(crate) fn new(types: Vec<&'a ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            _type: PhantomData::<C>,
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
    pub fn iter(&self) -> ComponentIter<'a, C> {
        return ComponentIter::<'a, C>::new(&self.types, self.len);
    }

    pub fn test(&self) -> &'a C {
        self.types[0]
            .iter()
            .next()
            .unwrap()
            .1
            .downcast_ref::<C>()
            .unwrap()
    }
}

impl<'a, C> IntoIterator for &'a ComponentSet<'a, C>
where
    C: ComponentController,
{
    type Item = &'a C;
    type IntoIter = ComponentIter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        return self.iter();
    }
}

/// Iterator over a [ComponentSet], which holds components from multiple [ComponentGroups](crate::ComponentGroup).
pub struct ComponentIter<'a, C>
where
    C: ComponentController,
{
    iters: Vec<ArenaIter<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    _type: PhantomData<C>,
}

impl<'a, C> ComponentIter<'a, C>
where
    C: ComponentController,
{
    pub(crate) fn new(types: &Vec<&'a ComponentType>, len: usize) -> ComponentIter<'a, C> {
        let mut iters = Vec::with_capacity(types.len());
        for t in types {
            iters.push(t.iter());
        }
        ComponentIter {
            iters,
            iter_index: 0,
            len,
            _type: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIter<'a, C>
where
    C: ComponentController,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentIter<'a, C>
where
    C: ComponentController,
{
    type Item = &'a C;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            if let Some((_, c)) = iter.next() {
                return c.as_ref().downcast_ref::<C>();
            } else {
                self.iter_index += 1;
                return self.next();
            }
        }
        return None;
    }
}

impl<'a, C> DoubleEndedIterator for ComponentIter<'a, C>
where
    C: ComponentController,
{
    fn next_back(&mut self) -> Option<&'a C> {
        let len = self.iters.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            if let Some((_, c)) = iter.next_back() {
                return c.as_ref().downcast_ref::<C>();
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
pub struct ComponentSetMut<'a, C: ComponentController> {
    pub(crate) types: Vec<&'a mut ComponentType>,
    pub(crate) len: usize,
    _type: PhantomData<C>,
}

impl<'a, C: ComponentController> ComponentSetMut<'a, C> {
    pub(crate) fn new(types: Vec<&'a mut ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            _type: PhantomData::<C>,
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
    pub fn iter(&'a mut self) -> ComponentIterMut<'a, C> {
        return ComponentIterMut::<'a, C>::new(&mut self.types, self.len);
    }
}

impl<'a, C> IntoIterator for &'a mut ComponentSetMut<'a, C>
where
    C: ComponentController,
{
    type Item = &'a mut C;
    type IntoIter = ComponentIterMut<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        return self.iter();
    }
}

/// Iterator over a [ComponentSetMut], which holds components from multiple [ComponentGroups](crate::ComponentGroup).
pub struct ComponentIterMut<'a, C>
where
    C: ComponentController,
{
    iters: Vec<ArenaIterMut<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    _type: PhantomData<C>,
}

impl<'a, C> ComponentIterMut<'a, C>
where
    C: ComponentController,
{
    pub(crate) fn new(
        types: &'a mut Vec<&'a mut ComponentType>,
        len: usize,
    ) -> ComponentIterMut<'a, C> {
        let mut iters = Vec::with_capacity(types.len());
        for t in types {
            iters.push(t.iter_mut());
        }
        ComponentIterMut {
            iters,
            iter_index: 0,
            len,
            _type: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIterMut<'a, C>
where
    C: ComponentController,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentIterMut<'a, C>
where
    C: ComponentController,
{
    type Item = &'a mut C;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            if let Some((_, c)) = iter.next() {
                return c.as_mut().downcast_mut::<C>();
            } else {
                self.iter_index += 1;
                return self.next();
            }
        }
        return None;
    }
}

impl<'a, C> DoubleEndedIterator for ComponentIterMut<'a, C>
where
    C: ComponentController,
{
    fn next_back(&mut self) -> Option<&'a mut C> {
        let len = self.iters.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            if let Some((_, c)) = iter.next_back() {
                return c.as_mut().downcast_mut::<C>();
            } else {
                self.iter_index += 1;
                return self.next_back();
            }
        }
        return None;
    }
}
