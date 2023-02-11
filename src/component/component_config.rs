use instant::Duration;


/// Desribes how  a component gets rendered
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderOperation {
    /// Does not render at all and therefore does not create a Buffer on the GPU.
    None,
    /// Render all components in the same method by calling `grouped_render`. A Set of all components of
    /// a group get provided. Use this if your components all draw the same graphics on the same model.
    Grouped,
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
    None,
    EveryFrame,
    EveryNFrame(u64),
    AfterDuration(Duration),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Defines the postproccess operations
pub enum PostproccessOperation {
    /// No postprocessing is applied
    None,
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
    /// The position, rotation and the scale of the component does not change. For Example a Tree
    /// or a Background Wall. This boosts performance by allot since not every frame the matrix of
    /// the component needs to be computed and written into the buffer. You always can call
    /// `force_matrix_update` on the `ComponentSet` of the type to manually force the update off the buffer.
    pub does_move: bool,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            does_move: true,
            update: UpdateOperation::EveryFrame,
            postproccess: PostproccessOperation::None,
            render: RenderOperation::Grouped,
            camera: CameraUse::World,
            priority: 16,
        }
    }
}

