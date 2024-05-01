#[cfg(feature = "framebuffer")]
use crate::graphics::SpriteRenderTarget;
use std::cell::Ref;

use crate::{
    entity::{
        Entities, EntityId, EntityIdentifier, EntityManager, EntityType, GroupedEntities,
        SingleEntity,
    },
    graphics::{
        CameraBuffer, CameraBuffer2D, DefaultAssets, Instance, Instance2D, InstanceBuffer,
        Mesh2D, RenderGroupManager, RenderTarget, SurfaceRenderTarget,
        WorldCamera3D,
    },
    prelude::Scene,
    system::SystemManager,
};

pub struct RenderContextEntities<'a>(&'a EntityManager);

impl<'a> RenderContextEntities<'a> {
    pub fn get_dyn(&self, type_id: EntityId) -> Ref<dyn EntityType> {
        self.0.get_dyn(type_id)
    }

    pub fn single<E: EntityIdentifier>(&self) -> Ref<SingleEntity<E>> {
        self.0.single::<E>()
    }

    pub fn get<E: EntityIdentifier>(&self) -> Ref<Entities<E>> {
        self.0.get::<E>()
    }

    pub fn group<ET: EntityType + Default>(&self) -> Ref<GroupedEntities<ET>> {
        self.0.group::<ET>()
    }
}

pub struct RenderContext<'a> {
    pub entities: RenderContextEntities<'a>,
    #[cfg(feature = "framebuffer")]
    pub framebuffer_target: &'a SpriteRenderTarget,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_assets: &'a DefaultAssets,
    pub render_groups: &'a RenderGroupManager,

    pub world_camera2d: &'a CameraBuffer2D,
    pub world_camera3d: &'a CameraBuffer<WorldCamera3D>,
    pub relative_camera: &'a CameraBuffer2D,
    pub relative_bottom_left_camera: &'a CameraBuffer2D,
    pub relative_bottom_right_camera: &'a CameraBuffer2D,
    pub relative_top_left_camera: &'a CameraBuffer2D,
    pub relative_top_right_camera: &'a CameraBuffer2D,
    pub unit_camera: &'a CameraBuffer2D,
    pub unit_mesh: &'a Mesh2D,
    /// Single Instance positioned at (0, 0)
    pub single_instance: &'a InstanceBuffer<Instance2D>,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        surface_target: &'a SurfaceRenderTarget,
        default_assets: &'a DefaultAssets,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                entities: RenderContextEntities(&scene.entities),
                render_groups: &scene.render_groups,
                relative_camera: &default_assets.relative_camera.0,
                relative_bottom_left_camera: &default_assets.relative_bottom_left_camera.0,
                relative_bottom_right_camera: &default_assets.relative_bottom_right_camera.0,
                relative_top_left_camera: &default_assets.relative_top_left_camera.0,
                relative_top_right_camera: &default_assets.relative_top_right_camera.0,
                unit_camera: &default_assets.unit_camera.0,
                single_instance: &default_assets.single_instance,
                unit_mesh: &default_assets.unit_mesh,
                world_camera2d: &default_assets.world_camera2d,
                world_camera3d: &default_assets.world_camera3d,
                default_assets,
                surface_target,
                #[cfg(feature = "framebuffer")]
                framebuffer_target: &default_assets.framebuffer,
            },
        )
    }

    pub fn target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return self.framebuffer_target;

        #[cfg(not(feature = "framebuffer"))]
        return self.surface_target;
    }

    pub fn group<I: Instance>(
        &self,
        name: &'static str,
        mut all: impl FnMut(&'a InstanceBuffer<I>),
    ) {
        let buffer = self
            .render_groups
            .get::<I>(name)
            .unwrap_or_else(|| panic!("Render group {name} is not registered!"))
            .buffer();

        if buffer.instance_amount() != 0 {
            (all)(buffer);
        }
    }
}
