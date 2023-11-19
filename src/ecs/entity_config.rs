#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndReason {
    EndProgram,
    RemoveScene,
    Replaced,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityScope {
    Scene,
    Global,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferOperation {
    Manual,
    EveryFrame,
    Never,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityStorage {
    Single,
    Multiple,
    Groups,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityConfig {
    pub buffer: BufferOperation,
    pub storage: EntityStorage,
    pub scope: EntityScope,
}

impl EntityConfig {
    pub const DEFAULT: EntityConfig = EntityConfig {
        buffer: BufferOperation::EveryFrame,
        storage: EntityStorage::Multiple,
        scope: EntityScope::Scene,
    };
    pub const SINGLE: EntityConfig = EntityConfig {
        storage: EntityStorage::Single,
        ..Self::DEFAULT
    };
    pub const RESOURCE: EntityConfig = EntityConfig {
        buffer: BufferOperation::Never,
        storage: EntityStorage::Single,
        ..Self::DEFAULT
    };
    pub const GLOBAL_RESOURCE: EntityConfig = EntityConfig {
        scope: EntityScope::Global,
        ..Self::RESOURCE
    };
}

impl Default for EntityConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}
