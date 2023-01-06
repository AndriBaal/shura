use core::hash::Hash;

#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle, World};
use crate::{
    ArenaIndex, ComponentSet, Context, DynamicScene, Instances, Matrix, Model, Renderer,
    Sprite,
};
use downcast_rs::*;

/// Dynamic component, that can be downcasted to any [ComponentController](crate::ComponentController)
/// using downcast_ref or downcast_mut.
pub type DynamicComponent = Box<dyn ComponentController>;

/// All [ComponentControllers](crate::ComponentController) need to derive from this trait, however
/// this is not done manually, but with the derive macro [Component](crate::Component).
///
/// # Example
/// ```
/// #[derive(Component)]
/// struct Bunny {
///     #[component] component: PositionComponent,
///     linvel: Vector<f32>,
/// }
/// ```
pub trait ComponentDerive {
    fn inner(&self) -> &dyn BaseComponent;
    fn inner_mut(&mut self) -> &mut dyn BaseComponent;
}

#[allow(unused_variables)]
/// shura has its own component system so every thing in the game is a component. Every struct
/// that implements this trait must have a [Component](crate::BaseComponent) field. This is usually
/// done with the [component derive macro](crate::Component)
///
///
/// A controller is used to add
/// data to a Component and define the behaviour of the componencomponents.len() as u32§t it controlls. Every component belongs to
/// one controller and every controller belongs to one component.
pub trait ComponentController: Downcast + _StaticAccess + ComponentDerive {
    /// This component gets updated if the component's [group](crate::ComponentGroup) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(&mut self, scene: &mut DynamicScene, ctx: &mut Context) {}
    /// This method gets called when this component gets destroyed.
    fn end(&mut self, scene: &mut DynamicScene, ctx: &mut Context) {}
    /// This component gets rendered if the component's [group](crate::ComponentGroup) is active.
    /// The render operation can be chosen through the [renderer](crate::Renderer) and the drawing
    /// can be completed with [renderer.commit()](crate::Renderer::commit())
    fn render<'a>(
        &'a self,
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
    }

    /// Grouped render of multiple components. This method gets called once for every group inwhich
    /// components of this type exist. This has massive performance advantes since many components
    /// can be rendered with the same operation, therefore it is mainly used for rendering
    /// components that have the exact same [model](crate::Model), [uniforms](crate::Uniform) or [sprites](crate::Sprite). 
    /// For this method to work the render operation of this component must be set to 
    /// [RenderOperation::Grouped](crate::RenderOperation::Grouped) in the [ComponentConfig](crate::ComponentConfig).
    fn render_grouped<'a>(
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        components: ComponentSet<DynamicComponent>,
        instances: Instances,
    ) where
        Self: Sized,
    {
    }

    #[cfg(feature = "physics")]
    /// Collision Event between 2 [PhysicsComponents](crate::physics::PhysicsComponent). It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [ComponentGroup](crate::ComponentGroup) is inactive or disabled.
    fn collision(
        &mut self,
        scene: &mut DynamicScene,
        ctx: &mut Context,
        other: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
    }

    /// Apply postprocessing after rendering all components of this Component. During rendering
    /// the relative camera is bound.
    fn postproccess<'a>(
        renderer: &mut Renderer<'a>,
        instance: Instances,
        screen_model: &'a Model,
        current_render: &'a Sprite,
    ) where
        Self: Sized,
    {
    }

    /// Configuration of all components from this type. See [ComponentConfig](crate::ComponentConfig)
    /// for more details.
    ///
    /// # Example
    /// ```
    /// fn config() -> &'static ComponentConfig {
    ///     static CONFIG: ComponentConfig = ComponentConfig {
    ///         priority: 1,
    ///         render: RenderOperation::Grouped,
    ///         ..ComponentConfig::default()
    ///     };
    ///     return &CONFIG;
    /// }
    /// ```
    fn config() -> &'static ComponentConfig
    where
        Self: Sized,
    {
        static CONFIG: ComponentConfig = ComponentConfig::default();
        &CONFIG
    }
}
impl_downcast!(ComponentController);

/// Grants access to the static members of the component type. This should never be overwritten,
/// since it is automatically implemented with generics.
pub trait _StaticAccess {
    fn get_config(&self) -> &'static ComponentConfig;
    fn get_grouped_render(
        &self,
    ) -> for<'a> fn(
        &'a DynamicScene,
        &mut Renderer<'a>,
        ComponentSet<DynamicComponent>,
        Instances,
    );
    fn get_postproccess(
        &self,
    ) -> for<'a> fn(&mut Renderer<'a>, Instances, &'a Model, &'a Sprite);
}

impl<T: ComponentController> _StaticAccess for T {
    fn get_config(&self) -> &'static ComponentConfig {
        T::config()
    }

    fn get_grouped_render(
        &self,
    ) -> for<'a> fn(
        &'a DynamicScene,
        &mut Renderer<'a>,
        ComponentSet<DynamicComponent>,
        Instances,
    ) {
        T::render_grouped
    }
    fn get_postproccess(
        &self,
    ) -> for<'a> fn(&mut Renderer<'a>, Instances, &'a Model, &'a Sprite) {
        T::postproccess
    }
}

/// Handle for a component. Through these handles components can be easily be fetches every frame
/// with a specific type through the [component](crate::Context::component) or
/// [component_mut](crate::Context::component_mut) method or without a specific type through the
/// [component_dynamic](crate::Context::component_dynamic) or
/// [component_dynamic_mut](crate::Context::component_dynamic_mut) method from the [context](crate::Context)
#[derive(Copy, Clone, Default, Debug)]
pub struct ComponentHandle {
    component_index: ArenaIndex,
    type_index: ArenaIndex,
    group_index: ArenaIndex,
    start: u64,
    id: u32,
}

impl Hash for ComponentHandle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Eq for ComponentHandle {}
impl PartialEq for ComponentHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl ComponentHandle {
    pub const UNINITIALIZED_ID: u32 = 0;

    #[inline]
    pub(crate) const fn new(
        component_index: ArenaIndex,
        type_index: ArenaIndex,
        group_index: ArenaIndex,
        start: u64,
        id: u32,
    ) -> Self {
        Self {
            id,
            start,
            component_index,
            type_index,
            group_index,
        }
    }

    #[inline]
    pub(crate) fn type_index(&self) -> ArenaIndex {
        self.type_index
    }

    #[inline]
    pub(crate) fn group_index(&self) -> ArenaIndex {
        self.group_index
    }

    #[inline]
    pub(crate) fn component_index(&self) -> ArenaIndex {
        self.component_index
    }

    /// Unique if of the handle and its component
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn start(&self) -> u64 {
        self.start
    }
}

#[allow(unused_variables)]
/// Every component like [PositionComponent](crate::PositionComponent) or
/// [PhysicsComponent](crate::physics::PhysicsComponent) implement this trait. This can be
/// used to create your own component.
pub trait BaseComponent: Downcast {
    fn init(&mut self, #[cfg(feature = "physics")] world: &mut World, handle: ComponentHandle);
    fn handle(&self) -> &ComponentHandle;
    fn matrix(&self, #[cfg(feature = "physics")] world: &World) -> Matrix;
}
impl_downcast!(BaseComponent);

/// Desribes how  a component gets rendered
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RenderOperation {
    /// Does not render at all and therefore does not create a Buffer on the GPU.
    None,
    /// Render all components in the same method by calling `grouped_render`. A Set of all components of
    /// a group get provided. Use this if your components all draw the same graphics on the same model.
    Grouped,
    /// Render all components individually by call `render` on each of them.
    Solo,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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
/// Defines the update of a component
pub enum UpdateOperation {
    None,
    EveryFrame,
    EveryNFrame(u64),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
/// Defines the postproccess operations
pub enum PostproccessOperation {
    /// No postprocessing is applied
    None,
    /// Postprocessing is done on the same layer as every other render operation
    SameLayer,
    /// The Postprocessing gets applied to a seperate layer before rendering it on top of the others
    SeperateLayer,
}

/// Default configuration for a component.
pub static DEFAULT_CONFIG: ComponentConfig = ComponentConfig::default();

/// The configuration of a component type. This configuration is used to statically define
/// behaviour of a component type for perfomance and utility reason.
#[derive(Debug)]
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
        Self::default()
    }
}

impl ComponentConfig {
    pub const fn default() -> ComponentConfig {
        Self {
            does_move: true,
            update: UpdateOperation::EveryFrame,
            postproccess: PostproccessOperation::None,
            render: RenderOperation::Solo,
            camera: CameraUse::World,
            priority: 10,
        }
    }
}

impl<T: ComponentController + ?Sized> ComponentDerive for Box<T> {
    fn inner(&self) -> &dyn BaseComponent {
        (**self).inner()
    }

    fn inner_mut(&mut self) -> &mut dyn BaseComponent {
        (**self).inner_mut()
    }
}

impl<T: ComponentController + ?Sized> ComponentController for Box<T> {
    fn update(&mut self, scene: &mut DynamicScene, ctx: &mut Context) {
        (**self).update(scene, ctx)
    }
    fn end(&mut self, scene: &mut DynamicScene, ctx: &mut Context) {
        (**self).end(scene, ctx)
    }
    fn render<'a>(
        &'a self,
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        (**self).render(scene, renderer, instance)
    }

    #[cfg(feature = "physics")]
    fn collision(
        &mut self,
        scene: &mut DynamicScene,
        ctx: &mut Context,
        other: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
        (**self).collision(
            scene,
            ctx,
            other,
            self_collider,
            other_collider,
            collide_type,
        )
    }
}
