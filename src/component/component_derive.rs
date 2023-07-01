#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle, World},
    ComponentHandle,
};
use crate::{
    Color, ComponentConfig, ComponentIdentifier, ComponentTypeId, Context, EndReason, Gpu,
    InstanceBuffer, InstanceData, RenderTarget, Renderer,
};
use downcast_rs::*;

/// Fields names of a struct used for deserialization and serialization
pub trait FieldNames {
    const FIELDS: &'static [&'static str];
}

/// Boxed component, that can be downcasted to any [Component](crate::Component)
/// using downcast_ref or downcast_mut.
pub type BoxedComponent = Box<dyn ComponentDerive>;

/// Base of every component. Provides a method to generate a 2D Matrix, so the component can be rendered
/// to the screen.
pub trait BaseComponent: Downcast {
    fn instance(&self, #[cfg(feature = "physics")] world: &World) -> InstanceData;
}
impl_downcast!(BaseComponent);

/// All components need to implement from this trait. This is not done manually, but with the derive macro [Component](crate::Component).
pub trait ComponentDerive: Downcast {
    fn base(&self) -> &dyn BaseComponent;
    fn base_mut(&mut self) -> &mut dyn BaseComponent;
    fn component_type_id(&self) -> ComponentTypeId;
}
impl_downcast!(ComponentDerive);

#[allow(unused_variables)]
/// A controller is used to define the behaviour of a component, by the given config and callbacks.
pub trait ComponentController: ComponentDerive + ComponentIdentifier + ComponentBuffer
where
    Self: Sized,
{
    const CONFIG: ComponentConfig = ComponentConfig::DEFAULT;
    /// This component gets updated if the component's [group](crate::Group) is active and enabled.
    /// Through the [context](crate::Context) you have access to all other scenes, groups,
    /// components with the matching controller and all data from the engine.
    fn update(ctx: &mut Context) {}

    #[cfg(feature = "physics")]
    /// Collision Event between 2 components. It requires that
    /// this component has the [ActiveEvents::COLLISION_EVENTS](crate::physics::ActiveEvents::COLLISION_EVENTS)
    /// flag set on its [RigidBody](crate::physics::RigidBody). Collisions still get processed even if
    /// the [Group](crate::Group) is inactive or disabled.
    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
    }

    /// This method gets called once for every group inwhich components of this type exist.
    /// This has massive performance advantes since many components
    /// can be rendered with the same operation, therefore it is mainly used for rendering
    /// components that have the exact same [model](crate::Model), [uniforms](crate::Uniform) or [sprites](crate::Sprite).
    /// Rendering is mainly done with [render_each](crate::ComponentManager::render_each) and [render_all](crate::ComponentManager::render_all).
    fn render<'a>(ctx: &'a Context, renderer: &mut Renderer<'a>) {}

    /// Method called when the game is closed or the scene gets removed
    fn end(ctx: &mut Context, reason: EndReason) {}

    fn render_target<'a>(ctx: &'a Context) -> (Option<Color>, &'a RenderTarget) {
        return (None, &ctx.defaults.world_target);
    }
}

impl<C: ComponentDerive + ?Sized> ComponentDerive for Box<C> {
    fn base(&self) -> &dyn BaseComponent {
        (**self).base()
    }

    fn base_mut(&mut self) -> &mut dyn BaseComponent {
        (**self).base_mut()
    }

    fn component_type_id(&self) -> ComponentTypeId {
        (**self).component_type_id()
    }
}

pub trait ComponentBuffer {
    const INSTANCE_SIZE: u64 = InstanceData::SIZE;
    fn buffer(
        buffer: &mut InstanceBuffer,
        #[cfg(feature = "physics")] world: &World,
        gpu: &Gpu,
        components: &mut dyn Iterator<Item = &BoxedComponent>,
    ) {
        let instances = components
            .into_iter()
            .map(|component| {
                component.base().instance(
                    #[cfg(feature = "physics")]
                    world,
                )
            })
            .collect::<Vec<InstanceData>>();
        buffer.write(gpu, bytemuck::cast_slice(&instances));
    }
}
