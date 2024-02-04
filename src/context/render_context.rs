use std::cell::Ref;
#[cfg(feature = "framebuffer")]
use crate::graphics::SpriteRenderTarget;

use crate::{
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityType, EntityTypeId, GroupedEntities,
        SingleEntity,
    },
    graphics::{
        CameraBuffer, CameraBuffer2D, DefaultResources, Instance, Instance2D, InstanceBuffer,
        InstanceIndices, Mesh2D, RenderGroupManager, RenderTarget, Renderer,
        SurfaceRenderTarget, WorldCamera3D,
    },
    prelude::Scene,
    system::SystemManager,
};

pub struct RenderContextEntities<'a>(&'a EntityManager);

impl<'a> RenderContextEntities<'a> {
    pub fn type_raw(&self, type_id: EntityTypeId) -> Ref<dyn EntityType> {
        self.0.type_raw_ref(type_id)
    }

    pub fn single<E: EntityIdentifier>(&self) -> Ref<SingleEntity<E>> {
        self.0.single_ref::<E>()
    }

    pub fn multiple<E: EntityIdentifier>(&self) -> Ref<Entities<E>> {
        self.0.multiple_ref::<E>()
    }

    pub fn group<ET: EntityType + Default>(&self) -> Ref<GroupedEntities<ET>> {
        self.0.group_ref::<ET>()
    }
}

pub struct RenderContext<'a> {
    pub entities: RenderContextEntities<'a>,
    #[cfg(feature = "framebuffer")]
    pub framebuffer_target: &'a SpriteRenderTarget,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_resources: &'a DefaultResources,
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
    pub centered_instance: &'a InstanceBuffer<Instance2D>,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        surface_target: &'a SurfaceRenderTarget,
        default_resources: &'a DefaultResources,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                entities: RenderContextEntities(&scene.entities),
                render_groups: &scene.render_groups,
                relative_camera: &default_resources.relative_camera.0,
                relative_bottom_left_camera: &default_resources.relative_bottom_left_camera.0,
                relative_bottom_right_camera: &default_resources.relative_bottom_right_camera.0,
                relative_top_left_camera: &default_resources.relative_top_left_camera.0,
                relative_top_right_camera: &default_resources.relative_top_right_camera.0,
                unit_camera: &default_resources.unit_camera.0,
                centered_instance: &default_resources.centered_instance,
                unit_mesh: &default_resources.unit_mesh,
                world_camera2d: &default_resources.world_camera2d,
                world_camera3d: &default_resources.world_camera3d,
                default_resources,
                surface_target,
                #[cfg(feature = "framebuffer")]
                framebuffer_target: &default_resources.framebuffer,
            },
        )
    }

    pub fn target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return self.framebuffer_target;

        #[cfg(not(feature = "framebuffer"))]
        return self.surface_target;
    }

    pub fn render<I: Instance>(
        &self,
        renderer: &mut Renderer<'a>,
        name: &'static str,
        all: impl Fn(&mut Renderer<'a>, &'a InstanceBuffer<I>, InstanceIndices),
    ) {
        let buffer = self
            .render_groups
            .get::<I>(name)
            .unwrap_or_else(|| panic!("Component {name} is not registered!"))
            .buffer();

        if buffer.instance_amount() != 0 {
            (all)(renderer, buffer, buffer.instances());
        }
    }
}
