#[cfg(feature = "physics")]
use crate::physics::{CollideType, ColliderHandle, ComponentHandle};
use crate::{
    data::arena::ArenaIter, BaseComponent, ComponentConfig, Context, Instances, Model, RenderIter,
    Renderer, Sprite,
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
    fn base(&self) -> &BaseComponent;
    fn base_mut(&mut self) -> &mut BaseComponent;
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
        return ComponentConfig::default();
    }
}
impl_downcast!(ComponentController);

impl<C: ComponentController + ?Sized> ComponentDerive for Box<C> {
    fn base(&self) -> &BaseComponent {
        (**self).base()
    }

    fn base_mut(&mut self) -> &mut BaseComponent {
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
