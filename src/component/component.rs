use core::hash::Hash;

#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle, World};
use crate::{
    data::arena::ArenaIter, ArenaIndex, Context, Instances, Matrix, Model, RenderIter, Renderer,
    Sprite,
};
use downcast_rs::*;
use instant::Duration;

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
    fn base(&self) -> &dyn BaseComponent;
    fn base_mut(&mut self) -> &mut dyn BaseComponent;
}

#[allow(unused_variables)]
/// shura has its own component system so every thing in the game is a component. Every struct
/// that implements this trait must have a [Component](crate::BaseComponent) field. This is usually
/// done with the [component derive macro](crate::Component)
///
/// A controller is used to add
/// data to a Component and define the behaviour of the componencomponents.len() as u32§t it controlls. Every component belongs to
/// one controller and every controller belongs to one component.
pub trait ComponentController: Downcast + _StaticAccess + ComponentDerive {
    /// This component gets updated if the component's [group](crate::ComponentGroup) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(&mut self, ctx: &mut Context) {}

    #[cfg(feature = "physics")]
    /// Collision Event between 2 [PhysicsComponents](crate::physics::PhysicsComponent). It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [ComponentGroup](crate::ComponentGroup) is inactive or disabled.
    fn collision(
        &mut self,
        ctx: &mut Context,
        other: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
    }

    /// Grouped render of multiple components. This method gets called once for every group inwhich
    /// components of this type exist. This has massive performance advantes since many components
    /// can be rendered with the same operation, therefore it is mainly used for rendering
    /// components that have the exact same [model](crate::Model), [uniforms](crate::Uniform) or [sprites](crate::Sprite).
    /// For this method to work the render operation of this component must be set to
    /// [RenderOperation::Grouped](crate::RenderOperation::Grouped) in the [ComponentConfig](crate::ComponentConfig).
    fn render<'a>(
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        components: RenderIter<'a, Self>,
        instances: Instances,
    ) where
        Self: Sized,
    {
    }

    /// Apply postprocessing after rendering all components of this Component. During rendering
    /// the relative camera is bound.
    fn postproccess<'a>(
        ctx: &Context,
        renderer: &mut Renderer<'a>,
        instance: Instances,
        screen_model: &'a Model,
        current_render: &'a Sprite,
    ) where
        Self: Sized,
    {
    }

    fn config() -> ComponentConfig
    where
        Self: Sized,
    {
        return DEFAULT_CONFIG;
    }
}
impl_downcast!(ComponentController);

/// Handle for a component. Through these handles components can be easily be fetches every frame
/// with a specific type through the [component](crate::Context::component) or
/// [component_mut](crate::Context::component_mut) method or without a specific type through the
/// [component_dynamic](crate::Context::component_dynamic) or
/// [component_dynamic_mut](crate::Context::component_dynamic_mut) method from the [context](crate::Context)
#[derive(Copy, Clone, Default, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentHandle {
    component_index: ArenaIndex,
    type_index: ArenaIndex,
    group_index: ArenaIndex,
    id: u32,
}

impl Hash for ComponentHandle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // The id is unique per ComponentHandle, so hashing only the id is faster
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
}

#[allow(unused_variables)]
/// Every component like [PositionComponent](crate::PositionComponent) or
/// [PhysicsComponent](crate::physics::PhysicsComponent) implement this trait. This can be
/// used to create your own component.
pub trait BaseComponent: Downcast {
    fn init(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        type_id: ComponentTypeId,
        handle: ComponentHandle,
    );
    fn handle(&self) -> &ComponentHandle;
    fn matrix(&self, #[cfg(feature = "physics")] world: &World) -> Matrix;
}
impl_downcast!(BaseComponent);

/// Desribes how  a component gets rendered
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderOperation {
    /// Does not render at all and therefore does not create a Buffer on the GPU.
    None,
    /// Render all components in the same method by calling `grouped_render`. A Set of all components of
    /// a group get provided. Use this if your components all draw the same graphics on the same model.
    Grouped,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
/// Defines the update of a component
pub enum UpdateOperation {
    None,
    EveryFrame,
    EveryNFrame(u64),
    AfterDuration(Duration),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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
pub const DEFAULT_CONFIG: ComponentConfig = ComponentConfig::default();

/// The configuration of a component type. This configuration is used to statically define
/// behaviour of a component type for perfomance and utility reason.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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
    pub const DEFAULT: Self = Self {
        does_move: true,
        update: UpdateOperation::EveryFrame,
        postproccess: PostproccessOperation::None,
        render: RenderOperation::Grouped,
        camera: CameraUse::World,
        priority: 16,
    };

    pub const fn default() -> ComponentConfig {
        Self::DEFAULT
    }
}

impl<C: ComponentController + ?Sized> ComponentDerive for Box<C> {
    fn base(&self) -> &dyn BaseComponent {
        (**self).base()
    }

    fn base_mut(&mut self) -> &mut dyn BaseComponent {
        (**self).base_mut()
    }
}

impl<C: ComponentController + ?Sized> ComponentController for Box<C> {
    fn update(&mut self, ctx: &mut Context) {
        (**self).update(ctx)
    }
    #[cfg(feature = "physics")]
    fn collision(
        &mut self,
        ctx: &mut Context,
        other: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
        (**self).collision(ctx, other, self_collider, other_collider, collide_type)
    }
}

/// Grants access to the static members of the component type. This should never be overwritten,
/// since it is automatically implemented with generics.
pub trait _StaticAccess {
    fn call_grouped_render<'a>(
        &self,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        iter: ArenaIter<'a, DynamicComponent>,
        instances: Instances,
    );
    fn call_postproccess<'a>(
        &self,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        instances: Instances,
        model: &'a Model,
        sprite: &'a Sprite,
    );
}

impl<C: ComponentController> _StaticAccess for C {
    fn call_grouped_render<'a>(
        &self,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        iter: ArenaIter<'a, DynamicComponent>,
        instances: Instances,
    ) {
        C::render(ctx, renderer, RenderIter::new(iter), instances);
    }
    fn call_postproccess<'a>(
        &self,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        instances: Instances,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        C::postproccess(ctx, renderer, instances, model, sprite);
    }
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentTypeId {
    id: u32,
}

pub trait ComponentIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ComponentTypeId = ComponentTypeId {
        id: const_fnv1a_hash::fnv1a_hash_str_32(Self::TYPE_NAME),
    };
}
