use std::cell::Ref;

use crate::{
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityType, EntityTypeId, GroupedEntities,
        SingleEntity,
    },
    graphics::{
        CameraBuffer, CameraBuffer2D, DefaultResources, Instance, Instance2D, InstanceBuffer,
        InstanceIndices, Mesh2D, RenderGroupManager, RenderTarget, Renderer, SurfaceRenderTarget,
        WorldCamera3D,
    },
    prelude::Scene,
    system::SystemManager,
};

pub struct RenderContext<'a> {
    entities: &'a EntityManager,
    surface_target: &'a SurfaceRenderTarget,
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
                entities: &scene.entities,
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
            },
        )
    }

    pub fn type_raw(&self, type_id: EntityTypeId) -> Ref<dyn EntityType> {
        self.entities.type_raw_ref(type_id)
    }

    pub fn single<E: EntityIdentifier>(&self) -> Ref<SingleEntity<E>> {
        self.entities.single_ref::<E>()
    }

    pub fn multiple<E: EntityIdentifier>(&self) -> Ref<Entities<E>> {
        self.entities.multiple_ref::<E>()
    }

    pub fn group<ET: EntityType + Default>(&self) -> Ref<GroupedEntities<ET>> {
        self.entities.group_ref::<ET>()
    }

    pub fn surface_target(&self) -> &dyn RenderTarget {
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
