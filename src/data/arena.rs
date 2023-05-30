use core::cmp;
use core::iter::{self, Extend, FusedIterator};
use core::mem;
use core::ops;
use core::slice;

pub(crate) struct Arena<T> {
    pub(crate) items: Vec<ArenaEntry<T>>,
    pub(crate) generation: u32,
    pub(crate) free_list_head: Option<usize>,
    pub(crate) len: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum ArenaEntry<T> {
    Free { next_free: Option<usize> },
    Occupied { generation: u32, data: T },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArenaIndex {
    pub(super) index: usize,
    pub(super) generation: u32,
}

impl ArenaIndex {
    pub const INVALID: Self = Self {
        index: u32::MAX as usize,
        generation: u32::MAX,
    };

    pub const FIRST: Self = Self {
        index: 0,
        generation: 0,
    };

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

pub(crate) const DEFAULT_CAPACITY: usize = 4;

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn as_slice(&mut self) -> &mut [ArenaEntry<T>] {
        &mut self.items
    }

    pub fn with_capacity(n: usize) -> Arena<T> {
        let n = cmp::max(n, 1);
        let mut arena = Arena {
            items: Vec::new(),
            generation: 0,
            free_list_head: None,
            len: 0,
        };
        arena.reserve(n);
        arena
    }

    pub fn try_insert(&mut self, data: T) -> Result<ArenaIndex, T> {
        match self.try_alloc_next_index() {
            None => Err(data),
            Some(index) => {
                self.items[index.index as usize] = ArenaEntry::Occupied {
                    generation: self.generation,
                    data,
                };
                Ok(index)
            }
        }
    }

    #[inline]
    pub fn try_insert_with<F: FnOnce(ArenaIndex) -> T>(
        &mut self,
        create: F,
    ) -> Result<ArenaIndex, F> {
        match self.try_alloc_next_index() {
            None => Err(create),
            Some(index) => {
                self.items[index.index as usize] = ArenaEntry::Occupied {
                    generation: self.generation,
                    data: create(index),
                };
                Ok(index)
            }
        }
    }

    fn try_alloc_next_index(&mut self) -> Option<ArenaIndex> {
        match self.free_list_head {
            None => None,
            Some(i) => match self.items[i as usize] {
                ArenaEntry::Free { next_free } => {
                    self.free_list_head = next_free;
                    self.len += 1;
                    Some(ArenaIndex {
                        index: i,
                        generation: self.generation,
                    })
                }
                _ => panic!("corrupt free list"),
            },
        }
    }

    pub fn insert(&mut self, data: T) -> ArenaIndex {
        match self.try_insert(data) {
            Ok(i) => i,
            Err(data) => self.insert_slow_path(data),
        }
    }

    #[inline(never)]
    fn insert_slow_path(&mut self, data: T) -> ArenaIndex {
        let len = if self.capacity() == 0 {
            // `drain()` sets the capacity to 0 and if the capacity is 0, the
            // next `try_insert() `will refer to an out-of-range index because
            // the next `reserve()` does not add element, resulting in a panic.
            // So ensure that `self` have at least 1 capacity here.
            //
            // Ideally, this problem should be handled within `drain()`,but
            // this problem cannot be handled within `drain()` because `drain()`
            // returns an iterator that borrows `self` mutably.
            1
        } else {
            self.items.len()
        };
        self.reserve(len);
        self.try_insert(data)
            .map_err(|_| ())
            .expect("inserting will always succeed after reserving additional space")
    }

    #[inline]
    pub fn insert_with(&mut self, create: impl FnOnce(ArenaIndex) -> T) -> ArenaIndex {
        match self.try_insert_with(create) {
            Ok(i) => i,
            Err(create) => self.insert_with_slow_path(create),
        }
    }

    #[inline(never)]
    fn insert_with_slow_path(&mut self, create: impl FnOnce(ArenaIndex) -> T) -> ArenaIndex {
        let len = self.items.len();
        self.reserve(len);
        self.try_insert_with(create)
            .map_err(|_| ())
            .expect("inserting will always succeed after reserving additional space")
    }

    pub fn remove(&mut self, i: ArenaIndex) -> Option<T> {
        if i.index >= self.items.len() {
            return None;
        }

        match self.items[i.index as usize] {
            ArenaEntry::Occupied { generation, .. } if i.generation == generation => {
                let entry = mem::replace(
                    &mut self.items[i.index as usize],
                    ArenaEntry::Free {
                        next_free: self.free_list_head,
                    },
                );
                self.generation += 1;
                self.free_list_head = Some(i.index);
                self.len -= 1;

                match entry {
                    ArenaEntry::Occupied {
                        generation: _,
                        data,
                    } => Some(data),
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn contains(&self, i: ArenaIndex) -> bool {
        self.get(i).is_some()
    }

    pub fn get(&self, i: ArenaIndex) -> Option<&T> {
        match self.items.get(i.index as usize) {
            Some(ArenaEntry::Occupied { generation, data }) if *generation == i.generation => {
                Some(data)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, i: ArenaIndex) -> Option<&mut T> {
        match self.items.get_mut(i.index as usize) {
            Some(ArenaEntry::Occupied { generation, data }) if *generation == i.generation => {
                Some(data)
            }
            _ => None,
        }
    }

    pub fn get_unknown_gen(&self, i: usize) -> Option<&T> {
        match self.items.get(i) {
            Some(ArenaEntry::Occupied { data, .. }) => Some(data),
            _ => None,
        }
    }

    pub fn get_unknown_gen_mut(&mut self, i: usize) -> Option<&mut T> {
        match self.items.get_mut(i) {
            Some(ArenaEntry::Occupied { data, .. }) => Some(data),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.items.len()
    }

    pub fn reserve(&mut self, additional_capacity: usize) {
        let start = self.items.len();
        let end = self.items.len() + additional_capacity;
        let old_head = self.free_list_head;
        self.items.reserve_exact(additional_capacity);
        self.items.extend((start..end).map(|i| {
            if i == end - 1 {
                ArenaEntry::Free {
                    next_free: old_head,
                }
            } else {
                ArenaEntry::Free {
                    next_free: Some(i + 1),
                }
            }
        }));
        self.free_list_head = Some(start);
    }

    pub fn iter(&self) -> ArenaIter<T> {
        ArenaIter {
            len: self.len,
            base: self.items.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> ArenaIterMut<T> {
        ArenaIterMut {
            len: self.len,
            base: self.items.iter_mut().enumerate(),
        }
    }

    pub fn retain(&mut self, mut predicate: impl FnMut(ArenaIndex, &mut T) -> bool) {
        for i in 0..self.capacity() {
            let remove = match &mut self.items[i] {
                ArenaEntry::Occupied { generation, data } => {
                    let index = ArenaIndex {
                        index: i,
                        generation: *generation,
                    };
                    if predicate(index, data) {
                        None
                    } else {
                        Some(index)
                    }
                }

                _ => None,
            };
            if let Some(index) = remove {
                self.remove(index);
            }
        }
    }

    pub fn get2_mut(&mut self, i1: ArenaIndex, i2: ArenaIndex) -> (Option<&mut T>, Option<&mut T>) {
        let len = self.items.len();
        let i1_index = i1.index as usize;
        let i2_index = i2.index as usize;

        if i1_index == i2_index {
            assert!(i1.generation != i2.generation);

            if i1.generation > i2.generation {
                return (self.get_mut(i1), None);
            }
            return (None, self.get_mut(i2));
        }

        if i1_index >= len {
            return (None, self.get_mut(i2));
        } else if i2_index >= len {
            return (self.get_mut(i1), None);
        }

        let (raw_item1, raw_item2) = {
            let (xs, ys) = self.items.split_at_mut(cmp::max(i1_index, i2_index));
            if i1_index < i2_index {
                (&mut xs[i1_index], &mut ys[0])
            } else {
                (&mut ys[0], &mut xs[i2_index])
            }
        };

        let item1 = match raw_item1 {
            ArenaEntry::Occupied { generation, data } if *generation == i1.generation => Some(data),
            _ => None,
        };

        let item2 = match raw_item2 {
            ArenaEntry::Occupied { generation, data } if *generation == i2.generation => Some(data),
            _ => None,
        };

        (item1, item2)
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = (ArenaIndex, &'a T);
    type IntoIter = ArenaIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ArenaIter<'a, T> {
    len: usize,
    base: iter::Enumerate<slice::Iter<'a, ArenaEntry<T>>>,
}

impl<T> Clone for ArenaIter<'_, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            base: self.base.clone(),
        }
    }
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = (ArenaIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.base.next() {
                Some((
                    index,
                    &ArenaEntry::Occupied {
                        generation,
                        ref data,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex {
                        index: index,
                        generation,
                    };
                    return Some((idx, data));
                }
                Some((_, _)) => continue,
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for ArenaIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.base.next_back() {
                Some((
                    index,
                    &ArenaEntry::Occupied {
                        generation,
                        ref data,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex {
                        index: index,
                        generation,
                    };
                    return Some((idx, data));
                }
                Some((_, _)) => continue,
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ArenaIter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for ArenaIter<'a, T> {}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = (ArenaIndex, &'a mut T);
    type IntoIter = ArenaIterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct ArenaIterMut<'a, T> {
    len: usize,
    base: iter::Enumerate<slice::IterMut<'a, ArenaEntry<T>>>,
}

impl<'a, T> Iterator for ArenaIterMut<'a, T> {
    type Item = (ArenaIndex, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.base.next() {
                Some((
                    index,
                    &mut ArenaEntry::Occupied {
                        generation,
                        ref mut data,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex {
                        index: index,
                        generation,
                    };
                    return Some((idx, data));
                }
                Some((_, _)) => continue,
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for ArenaIterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.base.next_back() {
                Some((
                    index,
                    &mut ArenaEntry::Occupied {
                        generation,
                        ref mut data,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex {
                        index: index,
                        generation,
                    };
                    return Some((idx, data));
                }
                Some((_, _)) => continue,
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ArenaIterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for ArenaIterMut<'a, T> {}

impl<T> Extend<T> for Arena<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for t in iter {
            self.insert(t);
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArenaIntoIter<T> {
    len: usize,
    inner: std::vec::IntoIter<ArenaEntry<T>>,
}

impl<T> Iterator for ArenaIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(ArenaEntry::Free { .. }) => continue,
                Some(ArenaEntry::Occupied { data, .. }) => {
                    self.len -= 1;
                    return Some(data);
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> DoubleEndedIterator for ArenaIntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next_back() {
                Some(ArenaEntry::Free { .. }) => continue,
                Some(ArenaEntry::Occupied { data, .. }) => {
                    self.len -= 1;
                    return Some(data);
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<T> ExactSizeIterator for ArenaIntoIter<T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<T> FusedIterator for ArenaIntoIter<T> {}

impl<T> IntoIterator for Arena<T> {
    type Item = T;
    type IntoIter = ArenaIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        ArenaIntoIter {
            len: self.len,
            inner: self.items.into_iter(),
        }
    }
}

impl<T> FromIterator<T> for Arena<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let cap = upper.unwrap_or(lower);
        let cap = cmp::max(cap, 1);
        let mut arena = Arena::with_capacity(cap);
        arena.extend(iter);
        arena
    }
}

impl<T> ops::Index<ArenaIndex> for Arena<T> {
    type Output = T;

    fn index(&self, index: ArenaIndex) -> &Self::Output {
        self.get(index).expect("No element at index")
    }
}

impl<T> ops::IndexMut<ArenaIndex> for Arena<T> {
    fn index_mut(&mut self, index: ArenaIndex) -> &mut Self::Output {
        self.get_mut(index).expect("No element at index")
    }
}
