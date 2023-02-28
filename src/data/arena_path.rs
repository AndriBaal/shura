use crate::ArenaIndex;

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArenaPath {
    pub(crate) group_index: ArenaIndex,
    pub(crate) type_index: ArenaIndex,
}
