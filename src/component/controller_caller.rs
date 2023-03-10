#[cfg(feature = "physics")]
use crate::{
    physics::{CollideType, ColliderHandle},
    ComponentHandle,
};
use crate::{ArenaPath, ComponentController, ComponentPath, Context, RenderConfig, RenderEncoder};

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
    fn call_render<'a>(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    );
    fn call_end(paths: &[ArenaPath], ctx: &mut Context);
}

impl<C: ComponentController> ComponentControllerCaller for C {
    fn call_render<'a>(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ) {
        C::render(ComponentPath::new(paths), ctx, config, encoder);
    }

    fn call_update(paths: &[ArenaPath], ctx: &mut Context) {
        C::update(ComponentPath::new(paths), ctx)
    }

    fn call_end(paths: &[ArenaPath], ctx: &mut Context) {
        C::end(ComponentPath::new(paths), ctx)
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
    pub call_end: fn(paths: &[ArenaPath], ctx: &mut Context),
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
    pub call_render: for<'a> fn(
        paths: &[ArenaPath],
        ctx: &'a Context<'a>,
        config: RenderConfig<'a>,
        encoder: &mut RenderEncoder,
    ),
}

impl ComponentCallbacks {
    pub fn new<C: ComponentController>() -> Self {
        return Self {
            call_end: C::call_end,
            call_update: C::call_update,
            #[cfg(feature = "physics")]
            call_collision: C::call_collision,
            call_render: C::call_render,
        };
    }
}
