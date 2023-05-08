#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle,
};
use crate::{ActiveComponents, ArenaPath, ComponentController, Context, RenderEncoder};

/// Grants access to the static members of the component type. This should never be overwritten,
/// since it is automatically implemented with generics.
pub trait ComponentControllerCaller
where
    Self: Sized,
{
    fn call_update(paths: &[ArenaPath], ctx: &mut Context);
    #[cfg(feature = "physics")]
    fn call_collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    );
    fn call_render(paths: &[ArenaPath], ctx: &Context, encoder: &mut RenderEncoder);
}

impl<C: ComponentController> ComponentControllerCaller for C {
    fn call_render(paths: &[ArenaPath], ctx: &Context, encoder: &mut RenderEncoder) {
        C::render(&ActiveComponents::new(paths), ctx, encoder);
    }

    fn call_update(paths: &[ArenaPath], ctx: &mut Context) {
        C::update(&ActiveComponents::new(paths), ctx)
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
    #[cfg(feature = "physics")]
    pub call_collision: fn(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        other_handle: ComponentHandle,
        self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collision_type: CollideType,
    ),
    pub call_render: fn(paths: &[ArenaPath], ctx: &Context, encoder: &mut RenderEncoder),
}

impl ComponentCallbacks {
    pub fn new<C: ComponentController>() -> Self {
        return Self {
            call_update: C::call_update,
            #[cfg(feature = "physics")]
            call_collision: C::call_collision,
            call_render: C::call_render,
        };
    }
}
