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
/// Describes if the end method of the [ComponentController](crate::ComponentController) should be called
/// when the [Scene](crate::Scene) is destroyed, by either deleting it or the window closing.
pub enum EndOperation {
    /// No operation will be called.
    None,
    /// end method gets called of the [ComponentController](crate::ComponentController) with all components of this type in the [ComponentPath](crate::ComponentPath).
    AllComponents,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Describes when the [Matrix](crate::RenderMatrix) of the components should be bufferd.
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentConfig {
    /// Describes the order in which components are processed
    pub priority: i16,
    // pub camera: CameraUse,
    // pub postproccess: PostproccessOperation,
    pub update: UpdateOperation,
    pub render: RenderOperation,
    pub buffer: BufferOperation,
    pub end: EndOperation,
}

pub const DEFAULT_CONFIG: ComponentConfig = ComponentConfig {
    buffer: BufferOperation::EveryFrame,
    update: UpdateOperation::EveryFrame,
    render: RenderOperation::EveryFrame,
    end: EndOperation::None,
    // camera: CameraUse::World,
    // postproccess: PostproccessOperation::Never,
    priority: 16,
};

impl Default for ComponentConfig {
    fn default() -> Self {
        DEFAULT_CONFIG
    }
}
