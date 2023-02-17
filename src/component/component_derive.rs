#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle,
};
use crate::{
    ActiveComponents, ArenaPath, BaseComponent, ComponentConfig, Context, Instances, Model,
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
pub trait ComponentController: Downcast + ComponentControllerCaller + ComponentDerive {
    /// This component gets updated if the component's [group](crate::ComponentGroup) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(set: ActiveComponents<Self>, ctx: &mut Context)
    where
        Self: Sized,
    {
    }

    #[cfg(feature = "physics")]
    /// Collision Event between 2 [PhysicsComponents](crate::physics::PhysicsComponent). It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [ComponentGroup](crate::ComponentGroup) is inactive or disabled.
    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) where
        Self: Sized,
    {
    }

    /// Grouped render of multiple components. This method gets called once for every group inwhich
    /// components of this type exist. This has massive performance advantes since many components
    /// can be rendered with the same operation, therefore it is mainly used for rendering
    /// components that have the exact same [model](crate::Model), [uniforms](crate::Uniform) or [sprites](crate::Sprite).
    /// For this method to work the render operation of this component must be set to
    /// [RenderOperation::Grouped](crate::RenderOperation::Grouped) in the [ComponentConfig](crate::ComponentConfig).
    fn render<'a>(
        set: ActiveComponents<Self>,
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
    ) where
        Self: Sized,
    {
    }

    /// Apply postprocessing after rendering all components of this Component. During rendering
    /// the relative camera is bound.
    fn postproccess<'a>(
        ctx: &Context,
        renderer: &mut Renderer<'a>,
        // components: RenderIter<'a, Self>,
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

/// Grants access to the static members of the component type. This should never be overwritten,
/// since it is automatically implemented with generics.
pub trait ComponentControllerCaller {
    fn call_update(paths: &[ArenaPath], ctx: &mut Context)
    where
        Self: Sized;
    #[cfg(feature = "physics")]
    fn call_collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) where
        Self: Sized;
    fn call_render<'a>(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
    ) where
        Self: Sized;
    fn call_postproccess<'a>(
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
        model: &'a Model,
        sprite: &'a Sprite,
    ) where
        Self: Sized;
}

impl<C: ComponentController> ComponentControllerCaller for C {
    fn call_render<'a>(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
    ) {
        C::render(ActiveComponents::new(paths), ctx, renderer, all_instances);
    }
    fn call_postproccess<'a>(
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        C::postproccess(ctx, renderer, all_instances, model, sprite);
    }

    fn call_update(paths: &[ArenaPath], ctx: &mut Context) {
        C::update(ActiveComponents::new(paths), ctx)
    }

    #[cfg(feature = "physics")]
    fn call_collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        C::collision(
            ctx,
            self_handle,
            other_handle,
            self_collider,
            other_collider,
            collision_type,
        )
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ComponentCallbacks {
    pub call_update: fn(paths: &[ArenaPath], ctx: &mut Context),
    pub call_postproccess: for<'a> fn(
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
        model: &'a Model,
        sprite: &'a Sprite,
    ),
    #[cfg(feature = "physics")]
    pub call_collision: fn(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ),
    pub call_render: for<'a> fn(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        renderer: &mut Renderer<'a>,
        all_instances: Instances,
    ),
}

impl ComponentCallbacks {
    pub fn new<C: ComponentController>() -> Self {
        return Self {
            call_update: C::call_update,
            call_postproccess: C::call_postproccess,
            #[cfg(feature = "physics")]
            call_collision: C::call_collision,
            call_render: C::call_render,
        };
    }
}
