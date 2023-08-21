use crate::Duration;

pub enum EndReason {
    EndProgram,
    RemoveScene,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentScope {
    Scene,
    Global
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Defines which camera should be used for rendering
pub enum CameraUse {
    /// Use the camera of the world
    World,
    /// The position, rotation and the scale is always relative to the screen. On the top right is
    /// always (1.0, 1.0) and on the bottom left (-1.0, -1.0). This only has affects on
    /// `PositionComponent`
    Relative,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Defines the update of a component
pub enum UpdateOperation {
    Never,
    EveryFrame,
    EveryNFrame(u64),
    AfterDuration(Duration),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderOperation {
    Never,
    EveryFrame,
}

/// Defines how to component gets stored. It is either a signle, multiple of it can be
/// stored or it has multiple groups
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentStorage {
    Single,
    Multiple,
    Groups,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndOperation {
    Never,
    Always,
}

/// The configuration of a component type. This configuration is used to statically define
/// behaviour of a component type for perfomance and utility reason.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentConfig {
    /// Describes the order in which components are updated and ended
    pub update_priority: i16,
    /// Describes the order in which components are rendered
    pub render_priority: i16,
    pub end_priority: i16,
    pub update: UpdateOperation,
    pub render: RenderOperation,
    pub buffer: BufferOperation,
    pub end: EndOperation,
    pub storage: ComponentStorage,
    pub scope: ComponentScope,
    // Update even when the game is paused
    pub force_update_level: i16,
}

impl ComponentConfig {
    pub const DEFAULT_PRIORITY: i16 = 16;
    pub const DEFAULT_FORCE_UPDATE_LEVEL: i16 = 0;
    pub const DEFAULT: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::EveryFrame,
        render: RenderOperation::EveryFrame,
        update: UpdateOperation::EveryFrame,
        end: EndOperation::Never,
        storage: ComponentStorage::Multiple,
        update_priority: Self::DEFAULT_PRIORITY,
        render_priority: Self::DEFAULT_PRIORITY,
        end_priority: Self::DEFAULT_PRIORITY,
        force_update_level: Self::DEFAULT_FORCE_UPDATE_LEVEL,
        scope: ComponentScope::Scene
    };
    pub const RESOURCE: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::Never,
        render: RenderOperation::Never,
        update: UpdateOperation::Never,
        end: EndOperation::Never,
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
