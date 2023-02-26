use instant::Duration;

/// Desribes how  a component gets rendered
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderOperation {
    /// Does not render at all and therefore does not create a Buffer on the GPU.
    Never,
    /// Render all components in the same method by calling `grouped_render`. A Set of all components of
    /// a group get provided. Use this if your components all draw the same graphics on the same model.
    EveryFrame,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndOperation {
    None,
    AllComponents,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferOperation {
    Manual,
    EveryFrame,
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
    /// Describes which camera should be used for rendering
    pub camera: CameraUse,
    /// Describes the priority the update and render methods gets called.
    pub priority: i16,
    /// Indicates if the controller should be updated.
    pub update: UpdateOperation,
    /// Indicates if after rendering the component, postproccessing should be applied to the frame
    pub postproccess: PostproccessOperation,
    /// Defines how rendering is handled for the component
    pub render: RenderOperation,
    /// Defines when the position of the component should be buffered
    pub buffer: BufferOperation,
    /// Defines if the end method should be called upon ending the scene by either removing it or the window being closed
    pub end: EndOperation
}

pub const DEFAULT_CONFIG: ComponentConfig = ComponentConfig {
    buffer: BufferOperation::EveryFrame,
    update: UpdateOperation::EveryFrame,
    postproccess: PostproccessOperation::Never,
    render: RenderOperation::EveryFrame,
    end: EndOperation::None,
    camera: CameraUse::World,
    priority: 16,
};

impl Default for ComponentConfig {
    fn default() -> Self {
        DEFAULT_CONFIG
    }
}
