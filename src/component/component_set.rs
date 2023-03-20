use instant::Instant;

use crate::{
    ArenaIter, ArenaIterMut, ArenaPath, ComponentCallbacks, ComponentConfig,
    ComponentType, DynamicComponent, InstanceIndex, ComponentDerive,
};
use std::{iter::Enumerate, marker::PhantomData};

#[derive(Clone)]
pub(crate) struct ComponentCluster {
    paths: Vec<ArenaPath>,
    config: ComponentConfig,
    last_update: Option<Instant>,
    callbacks: ComponentCallbacks,
}

impl ComponentCluster {
    pub fn new(
        path: ArenaPath,
        callbacks: ComponentCallbacks,
        config: ComponentConfig,
        now: Instant,
    ) -> Self {
        Self {
            paths: vec![path],
            last_update: match &config.update {
                crate::UpdateOperation::AfterDuration(_) => Some(now),
                _ => None,
            },
            config: config,
            callbacks,
        }
    }

    pub fn sort(&mut self) {
        self.paths
            .sort_by(|a, b| a.group_index.index().cmp(&b.group_index.index()));
    }

    pub fn clear(&mut self) {
        self.paths.clear();
    }

    pub fn add(&mut self, path: ArenaPath) {
        self.paths.push(path);
    }

    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    pub fn update_time(&mut self, now: Instant) {
        match &mut self.config.update {
            crate::UpdateOperation::AfterDuration(dur) => {
                if now > self.last_update.unwrap() + *dur {
                    self.last_update = Some(now);
                }
            }
            _ => {}
        };
    }

    pub const fn config(&self) -> &ComponentConfig {
        &self.config
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &Vec<ArenaPath> {
        &self.paths
    }

    pub fn callbacks(&self) -> &ComponentCallbacks {
        &self.callbacks
    }
}

pub struct ComponentPath<'a, C: ComponentDerive> {
    paths: &'a [ArenaPath],
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ComponentPath<'a, C> {
    pub(crate) fn new(paths: &'a [ArenaPath]) -> Self {
        Self {
            paths,
            marker: PhantomData,
        }
    }

    pub(crate) fn paths(&self) -> &[ArenaPath] {
        self.paths
    }

    pub fn amount_of_groups(&self) -> usize {
        self.paths.len()
    }
}

/// A set of components that includes all components of a specific type from a variety of
/// [ComponentGroups](crate::ComponentGroup).
/// A [ComponentSet] can be retrieved from the [Context](crate::Context) with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut).
pub struct ComponentSet<'a, C: ComponentDerive> {
    pub(crate) types: Vec<&'a ComponentType>,
    pub(crate) len: usize,
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ComponentSet<'a, C> {
    pub(crate) fn new(types: Vec<&'a ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            marker: PhantomData::<C>,
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
}

impl<'a, C> IntoIterator for &ComponentSet<'a, C>
where
    C: ComponentDerive,
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
    C: ComponentDerive,
{
    iters: Vec<ArenaIter<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<'a, C> ComponentIter<'a, C>
where
    C: ComponentDerive,
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
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIter<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentIter<'a, C>
where
    C: ComponentDerive,
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
    C: ComponentDerive,
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
pub struct ComponentSetMut<'a, C: ComponentDerive> {
    pub(crate) types: Vec<&'a mut ComponentType>,
    pub(crate) len: usize,
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ComponentSetMut<'a, C> {
    pub(crate) fn new(types: Vec<&'a mut ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            marker: PhantomData::<C>,
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
    C: ComponentDerive,
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
    C: ComponentDerive,
{
    iters: Vec<ArenaIterMut<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<'a, C> ComponentIterMut<'a, C>
where
    C: ComponentDerive,
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
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIterMut<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentIterMut<'a, C>
where
    C: ComponentDerive,
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
    C: ComponentDerive,
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

pub struct ComponentSetRender<'a, C: ComponentDerive> {
    pub(crate) types: Vec<&'a ComponentType>,
    pub(crate) len: usize,
    marker: PhantomData<C>,
}

impl<'a, C: ComponentDerive> ComponentSetRender<'a, C> {
    pub(crate) fn new(types: Vec<&'a ComponentType>, len: usize) -> Self {
        Self {
            types,
            len,
            marker: PhantomData::<C>,
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
    pub fn iter(&self) -> ComponentIterRender<'a, C> {
        return ComponentIterRender::<'a, C>::new(&self.types, self.len);
    }
}

impl<'a, C> IntoIterator for &ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    type Item = (InstanceIndex, &'a C);
    type IntoIter = ComponentIterRender<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        return self.iter();
    }
}

pub struct ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    iters: Vec<Enumerate<ArenaIter<'a, DynamicComponent>>>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<'a, C> ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    pub(crate) fn new(types: &Vec<&'a ComponentType>, len: usize) -> ComponentIterRender<'a, C> {
        let mut iters = Vec::with_capacity(types.len());
        for t in types {
            iters.push(t.iter().enumerate());
        }
        ComponentIterRender {
            iters,
            iter_index: 0,
            len,
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    type Item = (InstanceIndex, &'a C);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            if let Some((i, entry)) = iter.next() {
                let i = i as u32;
                return Some((
                    InstanceIndex { index: i },
                    entry.1.downcast_ref::<C>().unwrap(),
                ));
            }
            return None;
        }
        return None;
    }
}

impl<'a, C> DoubleEndedIterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let len = self.iters.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            if let Some((i, entry)) = iter.next_back() {
                let i = i as u32;
                return Some((
                    InstanceIndex { index: i - 1 },
                    entry.1.downcast_ref::<C>().unwrap(),
                ));
            }
            return None;
        }
        return None;
    }
}
