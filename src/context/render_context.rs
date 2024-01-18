use std::cell::Ref;

use crate::{
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityType, EntityTypeId, GroupedEntities,
        SingleEntity,
    },
    graphics::{
        CameraBuffer, CameraBuffer2D, DefaultResources, Instance, Instance2D, InstanceBuffer,
        InstanceIndices, Mesh2D, RenderGroupManager, Renderer, WorldCamera3D,
    },
    prelude::Scene,
    system::SystemManager,
};

pub struct RenderContext<'a> {
    entities: &'a EntityManager,
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
        defaults: &'a DefaultResources,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                entities: &scene.entities,
                render_groups: &scene.render_groups,
                relative_camera: &defaults.relative_camera.0,
                relative_bottom_left_camera: &defaults.relative_bottom_left_camera.0,
                relative_bottom_right_camera: &defaults.relative_bottom_right_camera.0,
                relative_top_left_camera: &defaults.relative_top_left_camera.0,
                relative_top_right_camera: &defaults.relative_top_right_camera.0,
                unit_camera: &defaults.unit_camera.0,
                centered_instance: &defaults.centered_instance,
                unit_mesh: &defaults.unit_mesh,
                world_camera2d: &defaults.world_camera2d,
                world_camera3d: &defaults.world_camera3d,
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
