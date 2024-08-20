use std::{cell::Ref, sync::Arc};

use crate::{
    component::{Component, ComponentIdentifier},
    entity::{
        ConstTypeId, Entities, Entity, EntityGroupManager, EntityHandle, EntityIdentifier,
        EntityManager, EntityType, GroupedEntities, SingleEntity,
    },
    graphics::{
        AssetKey, AssetManager, AssetWrapMut, DefaultAssets, Gpu, Instance, InstanceBuffer, Mesh,
        MeshBuilder, RenderTarget, SurfaceRenderTarget, Vertex,
    },
    physics::World,
    prelude::Index,
    scene::Scene,
    system::SystemManager,
};

pub struct RenderContextEntityManager<'a>(&'a EntityManager);

impl<'a> RenderContextEntityManager<'a> {
    pub fn get_dyn(&self, type_id: ConstTypeId) -> Ref<dyn EntityType> {
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

    pub fn components_each<C: ComponentIdentifier>(&self, each: impl FnMut(EntityHandle, &C)) {
        self.0.components_each(each)
    }
}

pub struct RenderContext<'a> {
    pub assets: Arc<AssetManager>,
    pub gpu: Arc<Gpu>,
    pub entities: RenderContextEntityManager<'a>,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_assets: &'a DefaultAssets,
    pub world: &'a World,
    pub groups: &'a EntityGroupManager,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        assets: Arc<AssetManager>,
        gpu: Arc<Gpu>,
        surface_target: &'a SurfaceRenderTarget,
        default_assets: &'a DefaultAssets,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                assets,
                gpu,
                entities: RenderContextEntityManager(&scene.entities),
                default_assets,
                surface_target,
                world: &scene.world,
                groups: &scene.groups,
            },
        )
    }

    pub fn target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return &self.default_assets.framebuffer;

        #[cfg(not(feature = "framebuffer"))]
        return self.surface_target;
    }

    pub fn write_instances<I: Instance>(
        &self,
        key: AssetKey,
        manual: bool,
        data: impl FnOnce(&mut Vec<I>),
    ) -> AssetWrapMut<InstanceBuffer<I>> {
        if !self.assets.exists(key) {
            self.assets.load_instance_buffer::<I>(key, &[]);
        }
        let mut instance_buffer = self.assets.get_mut::<InstanceBuffer<I>>(key);
        if manual && !instance_buffer.force_update {
            return instance_buffer;
        }
        instance_buffer.force_update = false;
        let instances = &mut instance_buffer.data;
        instances.clear();

        data(instances);

        // It is fine to replace with default value since Vec does not allocate
        let instances = std::mem::take(instances);
        instance_buffer.write(&self.gpu, &instances);
        instance_buffer.data = instances;

        instance_buffer
    }

    pub fn write_instance_entities_manual<E: Entity + EntityIdentifier, I: Instance>(
        &self,
        key: AssetKey,
        manual: bool,
        mut each: impl FnMut(&E, &mut Vec<I>),
    ) -> AssetWrapMut<InstanceBuffer<I>> {
        self.write_instances(key, manual, |data| {
            let entities = self.entities.get_dyn(E::IDENTIFIER);
            let iter = entities.iter_render(self.groups);
            for entity in iter {
                let entity = entity.downcast_ref().unwrap();
                each(entity, data);
            }
        })
    }

    pub fn write_instance_entities<E: Entity + EntityIdentifier, I: Instance>(
        &self,
        key: AssetKey,
        each: impl FnMut(&E, &mut Vec<I>),
    ) -> AssetWrapMut<InstanceBuffer<I>> {
        self.write_instance_entities_manual(key, false, each)
    }

    pub fn write_instance_components_manual<C: Component + ComponentIdentifier, I: Instance>(
        &self,
        key: AssetKey,
        manual: bool,
        mut each: impl FnMut(&C, &mut Vec<I>),
    ) -> AssetWrapMut<InstanceBuffer<I>> {
        self.write_instances(key, manual, |data| {
            self.entities
                .components_each(|_handle, component| each(component, data))
        })
    }

    pub fn write_instance_components<C: Component + ComponentIdentifier, I: Instance>(
        &self,
        key: AssetKey,
        each: impl FnMut(&C, &mut Vec<I>),
    ) -> AssetWrapMut<InstanceBuffer<I>> {
        self.write_instance_components_manual(key, false, each)
    }

    pub fn write_mesh<V: Vertex>(
        &self,
        key: AssetKey,
        manual: bool,
        update_indices_once: bool,
        data: impl FnOnce(&mut Vec<V>, Option<&mut Vec<Index>>),
    ) -> AssetWrapMut<Mesh<V>> {
        if !self.assets.exists(key) {
            self.assets.load(key, Mesh::<V>::empty(&self.gpu, 0, 0));
        }
        let mut mesh = self.assets.get_mut::<Mesh<V>>(key);

        if manual && !mesh.force_update {
            return mesh;
        }
        mesh.force_update = false;

        let update_indices = update_indices_once || mesh.write_indices;
        mesh.write_indices = false;

        let mesh_ref = &mut *mesh;
        let vertices = &mut mesh_ref.vertex_data;
        let indices = &mut mesh_ref.index_data;
        vertices.clear();
        indices.clear();

        data(vertices, if update_indices { Some(indices) } else { None });

        // It is fine to replace with default value since Vec does not allocate
        let indices = std::mem::take(indices);
        let vertices = std::mem::take(vertices);

        if update_indices {
            mesh.write_indices(&self.gpu, &indices);
        }
        mesh.write_vertices(&self.gpu, &vertices);

        mesh.vertex_data = vertices;
        mesh.index_data = indices;

        mesh
    }

    pub fn write_mesh_entities_manual<E: EntityIdentifier, V: Vertex, M: MeshBuilder<Vertex = V>>(
        &self,
        key: AssetKey,
        manual: bool,
        update_indices_once: bool,
        mut each: impl FnMut(&E) -> &M,
        mut each_vertex: Option<impl FnMut(&E, &V) -> V>,
    ) -> AssetWrapMut<Mesh<V>> {
        self.write_mesh(key, manual, update_indices_once, |vertices, mut indices| {
            let entities = self.entities.get_dyn(E::IDENTIFIER);
            let iter = entities.iter_render(self.groups);
            let mut index_offset = 0;
            for entity in iter {
                let entity = entity.downcast_ref().unwrap();
                let mesh = each(entity);
                if let Some(each_vertex) = each_vertex.as_mut() {
                    for v in mesh.vertices() {
                        let v = (each_vertex)(entity, v);
                        vertices.push(v);
                    }
                } else {
                    vertices.extend(mesh.vertices());
                }
                if let Some(ref mut indices) = indices {
                    indices.extend(mesh.indices().iter().map(|index| index + index_offset));
                    index_offset += mesh.vertices().len() as u32;
                }
            }
        })
    }

    pub fn write_mesh_components_manual<C: ComponentIdentifier, V: Vertex, M: MeshBuilder<Vertex = V>>(
        &self,
        key: AssetKey,
        manual: bool,
        update_indices_once: bool,
        mut each: impl FnMut(&C) -> &M,
        mut each_vertex: Option<impl FnMut(&C, &V) -> V>,
    ) -> AssetWrapMut<Mesh<V>> {
        self.write_mesh(key, manual, update_indices_once, |vertices, mut indices| {
            let mut index_offset = 0;
            self.entities.components_each(|_handle, component| {
                let mesh = each(component);
                if let Some(each_vertex) = each_vertex.as_mut() {
                    for v in mesh.vertices() {
                        let v = (each_vertex)(component, v);
                        vertices.push(v);
                    }
                } else {
                    vertices.extend(mesh.vertices());
                }
                if let Some(ref mut indices) = indices {
                    indices.extend(mesh.indices().iter().map(|index| index + index_offset));
                    index_offset += mesh.vertices().len() as u32;
                }
            })
        })
    }


    pub fn write_mesh_entities<E: EntityIdentifier, V: Vertex, M: MeshBuilder<Vertex = V>>(
        &self,
        key: AssetKey,
        each: impl FnMut(&E) -> &M,
        each_vertex: Option<impl FnMut(&E, &V) -> V>,
    ) -> AssetWrapMut<Mesh<V>> {
        self.write_mesh_entities_manual(key, false, false, each, each_vertex)
    }

    pub fn write_mesh_components<C: ComponentIdentifier, V: Vertex, M: MeshBuilder<Vertex = V>>(
        &self,
        key: AssetKey,
        each: impl FnMut(&C) -> &M,
        each_vertex: Option<impl FnMut(&C, &V) -> V>,
    ) -> AssetWrapMut<Mesh<V>> {
        self.write_mesh_components_manual(key, false, false, each, each_vertex)
    }
}
