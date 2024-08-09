use std::{cell::Ref, sync::Arc};

use crate::{
    entity::{
        Entities, EntityId, EntityIdentifier, EntityManager, EntityType, GroupedEntities,
        SingleEntity,
    },
    graphics::{AssetManager, DefaultAssets, RenderTarget, SurfaceRenderTarget},
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
}

pub struct RenderContext<'a> {
    pub assets: Arc<AssetManager>,
    pub entities: RenderContextEntityManager<'a>,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_assets: &'a DefaultAssets,
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
