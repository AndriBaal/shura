use std::cell::Ref;

use crate::{entity::{EntityManager, EntityIdentifier, EntityType, SingleEntity, Entities, GroupedEntities}, graphics::{ComponentBufferManager, CameraBuffer2D, WorldCamera3D, CameraBuffer, Instance2D, InstanceBuffer, Mesh2D, DefaultResources, Instance, Renderer, InstanceIndices}, system::SystemManager, prelude::Scene};


pub struct RenderContext<'a> {
    entities: &'a EntityManager,
    pub component_buffers: &'a ComponentBufferManager,

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
                component_buffers: &scene.component_buffers,
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

    pub fn type_raw<E: EntityIdentifier>(&self) -> Ref<dyn EntityType> {
        self.entities.type_raw_ref::<E>()
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

    pub fn render_all<I: Instance>(
        &self,
        renderer: &mut Renderer<'a>,
        name: &'static str,
        all: impl Fn(&mut Renderer<'a>, &'a InstanceBuffer<I>, InstanceIndices),
    ) {
        let buffer = self
            .component_buffers
            .get::<I>(name)
            .unwrap_or_else(|| panic!("Component {name} is not registered!"))
            .buffer();

        if buffer.instance_amount() != 0 {
            (all)(renderer, buffer, buffer.instances());
        }
    }
}
