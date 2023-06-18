use instant::Duration;

pub enum EndReason {
    EndProgram,
    RemoveScene,
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

/// Defines how to component gets stored. It is either a signle, multiple of it can be
/// stored or it has multiple groups
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentStorage {
    Single,
    Multiple,
    Groups,
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
    // /// When this flag is set, the component is automatically registered when adding one
    // pub auto_register: bool,
    pub update: UpdateOperation,
    pub buffer: BufferOperation,
    pub storage: ComponentStorage,
}

impl ComponentConfig {
    pub const DEFAULT_PRIORITY: i16 = 16;
    pub const DEFAULT: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::EveryFrame,
        update: UpdateOperation::EveryFrame,
        storage: ComponentStorage::Multiple,
        update_priority: Self::DEFAULT_PRIORITY,
        render_priority: Self::DEFAULT_PRIORITY,
    };
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}
