#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle,
};
use crate::{
    BaseComponent, ComponentConfig, ComponentIdentifier, ComponentTypeId, Context, RenderEncoder,
    DEFAULT_CONFIG,
};
use downcast_rs::*;

/// Fields names of a struct used for deserialization and serialization
pub trait FieldNames {
    const FIELDS: &'static [&'static str];
}

/// Boxed component, that can be downcasted to any [Component](crate::Component)
/// using downcast_ref or downcast_mut.
pub type BoxedComponent = Box<dyn ComponentDerive>;

/// All components need to implement from this trait. This is not done manually, but with the derive macro [Component](crate::Component).
///
/// # Example
/// ```
/// #[derive(Component)]
/// struct Bunny {
///     #[component] component: BaseComponent,
///     linvel: Vector<f32>,
/// }
/// ```
pub trait ComponentDerive: Downcast {
    fn base(&self) -> &BaseComponent;
    fn base_mut(&mut self) -> &mut BaseComponent;
}
impl_downcast!(ComponentDerive);

#[allow(unused_variables)]
/// A controller is used to define the behaviour of a component, by the given config and callbacks. The
/// currently relevant components get passed through the [ActiveComponents](crate::ActiveComponents).
pub trait ComponentController: ComponentDerive + ComponentIdentifier
where
    Self: Sized,
{
    const CONFIG: ComponentConfig = DEFAULT_CONFIG;
    /// This component gets updated if the component's [group](crate::ComponentGroup) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(ctx: &mut Context) {}

    #[cfg(feature = "physics")]
    /// Collision Event between 2 components. It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [ComponentGroup](crate::ComponentGroup) is inactive or disabled.
    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        other_type: ComponentTypeId,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
    }

    /// This method gets called once for every group inwhich components of this type exist.
    /// This has massive performance advantes since many components
    /// can be rendered with the same operation, therefore it is mainly used for rendering
    /// components that have the exact same [model](crate::Model), [uniforms](crate::Uniform) or [sprites](crate::Sprite).
    /// For this method to work the render operation of this component must be set to
    /// [RenderOperation::EveryFrame](crate::RenderOperation::EveryFrame) in the [ComponentConfig](crate::ComponentConfig).
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {}
}

impl<C: ComponentDerive + ?Sized> ComponentDerive for Box<C> {
    fn base(&self) -> &BaseComponent {
        (**self).base()
    }

    fn base_mut(&mut self) -> &mut BaseComponent {
        (**self).base_mut()
    }
}

impl ComponentDerive for BaseComponent {
    fn base(&self) -> &BaseComponent {
        self
    }

    fn base_mut(&mut self) -> &mut BaseComponent {
        self
    }
}
