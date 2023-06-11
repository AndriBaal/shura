use instant::Duration;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Desribes when a component gets rendered.
pub enum RenderOperation {
    /// Does not render at all and therefore does not create a Buffer on the GPU.
    Never,
    /// Draw all currrently relative components every frame.
    EveryFrame,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Describes when the [Matrix](crate::Matrix) of the components should be bufferd.
pub enum BufferOperation {
    /// Manual buffering by calling [force_buffer](`crate::Context::force_buffer()`). This is used when you have component that dont
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
/// Defines the postproccess operations
pub enum PostproccessOperation {
    /// No postprocessing is applied
    Never,
    /// Postprocessing is done on the same layer as every other render operation
    SameLayer,
    /// The Postprocessing gets applied to a seperate layer before rendering it on top of the others
    SeperateLayer,
}

/// The configuration of a component type. This configuration is used to statically define
/// behaviour of a component type for perfomance and utility reason.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentConfig {
    /// Describes the order in which components are processed
    pub priority: i16,
    // /// When this flag is set, the component is automatically registered when adding one
    // pub auto_register: bool,
    pub update: UpdateOperation,
    pub render: RenderOperation,
    pub buffer: BufferOperation,
}

impl ComponentConfig {
    pub const DEFAULT: ComponentConfig = ComponentConfig {
        buffer: BufferOperation::EveryFrame,
        update: UpdateOperation::EveryFrame,
        render: RenderOperation::EveryFrame,
        priority: 16,
    };
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}
