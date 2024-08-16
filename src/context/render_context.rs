use std::{cell::Ref, sync::Arc};

use nalgebra::Isometry2;

use crate::{
    entity::{
        Entities, EntityId, EntityIdentifier, EntityManager, EntityType, GroupedEntities,
        SingleEntity,
    },
    graphics::{
        AssetManager, BaseVertex2D, DefaultAssets, Index, Instance, MeshBuilder, MeshBuilder2D,
        RenderTarget, SurfaceRenderTarget, Vertex,
    },
    physics::World,
    scene::Scene,
    system::SystemManager,
};

pub struct RenderContextEntityManager<'a>(&'a EntityManager);

impl<'a> RenderContextEntityManager<'a> {
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

    pub fn instances<E: EntityIdentifier, I: Instance>(
        &self,
        mut each: impl FnMut(&E, &mut Vec<I>),
    ) -> Vec<I> {
        let entities = self.0.get_dyn(E::IDENTIFIER);
        let iter = entities.dyn_iter();
        let size = iter.size_hint();
        let mut instances = Vec::with_capacity(size.1.unwrap_or(size.0)); // TODO: Cache this (also apply to other methods)

        for (_, entity) in iter {
            // TODO: Use different iterator (also apply to other methods)
            let entity = entity.downcast_ref().unwrap();
            each(entity, &mut instances);
        }

        return instances;
    }

    pub fn meshes<E: EntityIdentifier, V: Vertex, M: MeshBuilder<Vertex = V>>(
        &self,
        mut each: impl FnMut(&E) -> &M,
    ) -> (Vec<V>, Vec<Index>) {
        let entities = self.0.get_dyn(E::IDENTIFIER);
        let iter = entities.dyn_iter();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0;

        for (_, entity) in iter {
            let entity = entity.downcast_ref().unwrap();
            let mesh = each(entity);
            vertices.extend(mesh.vertices());
            indices.extend(mesh.indices().iter().map(|index| index + index_offset));
            index_offset += mesh.vertices().len() as u32;
        }

        return (vertices, indices);
    }

    pub fn meshes_with_offset<E: EntityIdentifier, V: BaseVertex2D>(
        &self,
        mut each: impl FnMut(&E) -> (&MeshBuilder2D<V>, Isometry2<f32>),
    ) -> (Vec<V>, Vec<Index>) {
        let entities = self.0.get_dyn(E::IDENTIFIER);
        let iter = entities.dyn_iter();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0;

        for (_, entity) in iter {
            let entity = entity.downcast_ref().unwrap();
            let (mesh, offset) = each(entity);
            let translation = offset.translation.vector;
            let rotation = offset.rotation;
            for v in mesh.vertices() {
                let mut pos = *v.pos();
                pos = rotation * pos;
                pos += translation;
                vertices.push(v.with_pos(pos));
            }
            indices.extend(mesh.indices().iter().map(|index| index + index_offset));
            index_offset += vertices.len() as u32;
        }

        return (vertices, indices);
    }

    // TODO: Component and recursive component
}

pub struct RenderContext<'a> {
    pub assets: Arc<AssetManager>,
    pub entities: RenderContextEntityManager<'a>,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_assets: &'a DefaultAssets,
    pub world: &'a World,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        assets: Arc<AssetManager>,
        surface_target: &'a SurfaceRenderTarget,
        default_assets: &'a DefaultAssets,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                assets,
                entities: RenderContextEntityManager(&scene.entities),
                default_assets,
                surface_target,
                world: &scene.world,
            },
        )
    }

    pub fn target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return &self.default_assets.framebuffer;

        #[cfg(not(feature = "framebuffer"))]
        return self.surface_target;
    }
}
