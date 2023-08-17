use super::arena::{Arena, ArenaEntry, ArenaIndex, DEFAULT_CAPACITY};
use crate::BoxedComponent;
use crate::Component;
use core::cmp;
use core::fmt;
use core::iter;
use core::marker::PhantomData;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::Serializer;
use serde::Serialize;

impl Serialize for ArenaIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.index, self.generation).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArenaIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (index, generation) = Deserialize::deserialize(deserializer)?;
        Ok(ArenaIndex { index, generation })
    }
}

impl<T> Serialize for Arena<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Note: do not change the serialization format, or it may break
        // forward and backward compatibility of serialized data!
        serializer.collect_seq(self.items.iter().map(|entry| match entry {
            ArenaEntry::Occupied { generation, data } => Some((generation, data)),
            _ => None,
        }))
    }
}

impl Arena<BoxedComponent> {
    pub fn serialize_components<C: Component + Serialize>(
        &self,
    ) -> Vec<Option<(u32, Vec<u8>)>> {
        let e = self
            .items
            .iter()
            .map(|entry| match entry {
                ArenaEntry::Occupied { generation, data } => Some((
                    *generation,
                    bincode::serialize(data.downcast_ref::<C>().unwrap()).unwrap(),
                )),
                ArenaEntry::Free { .. } => None,
            })
            .collect();
        return e;
    }
}

// impl Arena<Group> {
//     pub fn serialize_groups(
//         &self,
//         ids: FxHashSet<GroupId>,
//     ) -> Vec<Option<(&u32, &Group)>> {
//         let e = self
//             .items
//             .iter()
//             .map(|entry| match entry {
//                 ArenaEntry::Occupied { generation, data } => {
//                     if ids.contains(&data.id()) {
//                         Some((generation, data))
//                     } else {
//                         None
//                     }
//                 }
//                 _ => None,
//             })
//             .collect();
//         return e;
//     }
// }

impl<'de, T> Deserialize<'de> for Arena<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ArenaVisitor::new())
    }
}

struct ArenaVisitor<T> {
    marker: PhantomData<fn() -> Arena<T>>,
}

impl<T> ArenaVisitor<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> Arena<T> {
    pub fn from_items(mut items: Vec<ArenaEntry<T>>, generation: u32) -> Arena<T> {
        // items.len() must be same as item.capacity(), so fill the unused elements with Free.
        if items.len() + 1 < items.capacity() {
            let add_cap = items.capacity() - (items.len() + 1);
            items.reserve_exact(add_cap);
            items.extend(iter::repeat_with(|| ArenaEntry::Free { next_free: None }).take(add_cap));
            assert_eq!(items.len(), items.capacity());
        }

        let mut free_list_head = None;
        let mut len = items.len();
        // Iterates `arena.items` in reverse order so that free_list concatenates
        // indices in ascending order.
        for (idx, entry) in items.iter_mut().enumerate().rev() {
            if let ArenaEntry::Free { next_free } = entry {
                *next_free = free_list_head;
                free_list_head = Some(idx);
                len -= 1;
            }
        }

        Arena {
            items,
            generation,
            free_list_head,
            len,
        }
    }
}

impl<'de, T> Visitor<'de> for ArenaVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Arena<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a generational arena")
    }

    fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: SeqAccess<'de>,
    {
        let init_cap = access.size_hint().unwrap_or(DEFAULT_CAPACITY);
        let mut items = Vec::with_capacity(init_cap);

        let mut generation = 0;
        while let Some(element) = access.next_element::<Option<(u32, T)>>()? {
            let item = match element {
                Some((gen, data)) => {
                    generation = cmp::max(generation, gen);
                    ArenaEntry::Occupied {
                        generation: gen,
                        data,
                    }
                }
                None => ArenaEntry::Free { next_free: None },
            };
            items.push(item);
        }

        return Ok(Arena::from_items(items, generation));
    }
}
