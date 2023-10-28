#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndReason {
    EndProgram,
    RemoveScene,
    Replaced,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentScope {
    Scene,
    Global,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Describes when the [Matrix](crate::Matrix) of the components should be bufferd.
pub enum BufferOperation {
    /// Manual buffering by calling [force_buffer](`crate::ComponentManager::force_buffer()`). This is used when you have component that dont
    /// change their position. If you add a new component, all components of this type from the group will be buffered.
    Manual,
    /// Automatically buffer all positions every time before rendering.
    EveryFrame,
    /// No Buffer is created for this component. When rendering an empty [InstanceBuffer](crate::InstanceBuffer) is passed to the [RenderConfig](crate::RenderConfig).
    Never,
}

/// Defines how to component gets stored and how many of it can exist
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentStorage {
    Single,
    Multiple,
    Groups,
}

/// The configuration of a component type
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentConfig {
    pub buffer: BufferOperation,
    pub storage: ComponentStorage,
    pub scope: ComponentScope,
}

impl ComponentConfig {
    pub const DEFAULT: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::EveryFrame,
        storage: ComponentStorage::Multiple,
        scope: ComponentScope::Scene,
    };
    pub const SINGLE: ComponentConfig = ComponentConfig {
        storage: ComponentStorage::Single,
        ..Self::DEFAULT
    };
    pub const RESOURCE: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::Never,
        storage: ComponentStorage::Single,
        ..Self::DEFAULT
    };
    pub const GLOBAL_RESOURCE: ComponentConfig = ComponentConfig {
        scope: ComponentScope::Global,
        ..Self::RESOURCE
    };
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}
