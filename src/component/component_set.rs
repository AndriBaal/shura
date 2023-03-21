use instant::Instant;

use crate::{
    ArenaIter, ArenaIterMut, ArenaPath, ComponentCallbacks, ComponentConfig, ComponentDerive,
    DynamicComponent, InstanceBuffer, InstanceIndex,
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
pub struct ComponentSet<'a, C>
where
    C: ComponentDerive,
{
    iters: Vec<ArenaIter<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<C: ComponentDerive> Clone for ComponentSet<'_, C> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iters: self.iters.clone(),
            marker: PhantomData,
            iter_index: self.iter_index,
            len: self.len,
        }
    }
}

impl<'a, C> ComponentSet<'a, C>
where
    C: ComponentDerive,
{
    pub(crate) fn new(
        iters: Vec<ArenaIter<'a, DynamicComponent>>,
        len: usize,
    ) -> ComponentSet<'a, C> {
        ComponentSet {
            iters,
            iter_index: 0,
            len,
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentSet<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentSet<'a, C>
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

impl<'a, C> DoubleEndedIterator for ComponentSet<'a, C>
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
pub struct ComponentSetMut<'a, C>
where
    C: ComponentDerive,
{
    iters: Vec<ArenaIterMut<'a, DynamicComponent>>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<'a, C> ComponentSetMut<'a, C>
where
    C: ComponentDerive,
{
    pub(crate) fn new(
        iters: Vec<ArenaIterMut<'a, DynamicComponent>>,
        len: usize,
    ) -> ComponentSetMut<'a, C> {
        ComponentSetMut {
            iters,
            iter_index: 0,
            len,
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentSetMut<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentSetMut<'a, C>
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

impl<'a, C> DoubleEndedIterator for ComponentSetMut<'a, C>
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

pub struct ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    iters: Vec<(&'a InstanceBuffer, ComponentIterRender<'a, C>)>,
    iter_index: usize,
    len: usize,
    marker: PhantomData<C>,
}

impl<C: ComponentDerive> Clone for ComponentSetRender<'_, C> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iters: self.iters.clone(),
            marker: PhantomData,
            iter_index: self.iter_index,
            len: self.len,
        }
    }
}

impl<'a, C> ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    pub(crate) fn new(
        iters: Vec<(&'a InstanceBuffer, ComponentIterRender<'a, C>)>,
        len: usize,
    ) -> ComponentSetRender<'a, C> {
        ComponentSetRender {
            iters,
            iter_index: 0,
            len,
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, C> Iterator for ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    type Item = (&'a InstanceBuffer, ComponentIterRender<'a, C>);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iters.get_mut(self.iter_index) {
            self.iter_index += 1;
            return Some(iter.clone());
        }
        return None;
    }
}

impl<'a, C> DoubleEndedIterator for ComponentSetRender<'a, C>
where
    C: ComponentDerive,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let len = self.len();
        if let Some(iter) = self.iters.get_mut(len - 1 - self.iter_index) {
            self.iter_index += 1;
            return Some(iter.clone());
        }
        return None;
    }
}

pub struct ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    iter: Enumerate<ArenaIter<'a, DynamicComponent>>,
    marker: PhantomData<C>,
}

impl<C: ComponentDerive> Clone for ComponentIterRender<'_, C> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            marker: PhantomData,
        }
    }
}

impl<'a, C> ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    pub(crate) fn new(
        iter: Enumerate<ArenaIter<'a, DynamicComponent>>,
    ) -> ComponentIterRender<'a, C> {
        ComponentIterRender {
            iter,
            marker: PhantomData::<C>,
        }
    }
}

impl<'a, C> ExactSizeIterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, C> Iterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    type Item = (InstanceIndex, &'a C);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, entry)) = self.iter.next() {
            let i = i as u32;
            return Some((
                InstanceIndex { index: i },
                entry.1.downcast_ref::<C>().unwrap(),
            ));
        }
        return None;
    }
}

impl<'a, C> DoubleEndedIterator for ComponentIterRender<'a, C>
where
    C: ComponentDerive,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some((i, entry)) = self.iter.next_back() {
            let i = i as u32;
            return Some((
                InstanceIndex { index: i - 1 },
                entry.1.downcast_ref::<C>().unwrap(),
            ));
        }
        return None;
    }
}
