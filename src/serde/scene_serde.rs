use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::{
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityScope, EntityType, EntityTypeId,
        GroupedEntities, SingleEntity,
    },
    graphics::{Gpu, Instance, Instance2D, Instance3D, RenderGroupConfig, GLOBAL_GPU},
    scene::Scene,
    system::System,
};

pub fn gpu() -> Arc<Gpu> {
    GLOBAL_GPU.get().unwrap().clone()
}

pub struct SceneSerializer<'a> {
    entities: &'a EntityManager,
    ser_entities: FxHashMap<EntityTypeId, Vec<u8>>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(entities: &'a EntityManager) -> Self {
        Self {
            entities,
            ser_entities: Default::default(),
        }
    }

    pub(crate) fn finish(self) -> FxHashMap<EntityTypeId, Vec<u8>> {
        self.ser_entities
    }

    pub fn serialize_entity<ET: EntityType + serde::Serialize>(mut self) -> Self {
        let ser = self.entities.serialize::<ET>();
        self.ser_entities.insert(ET::Entity::IDENTIFIER, ser);
        self
    }

    pub fn serialize_entities<E: EntityIdentifier + serde::Serialize>(self) -> Self {
        self.serialize_entity::<Entities<E>>()
    }

    pub fn serialize_single_entity<E: EntityIdentifier + serde::Serialize>(self) -> Self {
        self.serialize_entity::<SingleEntity<E>>()
    }

    pub fn serialize_grouped_entity<E: EntityIdentifier + serde::Serialize>(self) -> Self {
        self.serialize_entity::<GroupedEntities<Entities<E>>>()
    }
}

#[non_exhaustive]
pub struct SerializedScene {
    pub id: u32,
    pub scene: Scene,
    ser_entities: FxHashMap<EntityTypeId, Vec<u8>>,
}

impl SerializedScene {
    pub fn new(id: u32, scene: &[u8]) -> SerializedScene {
        let (scene, ser_entities): (Scene, FxHashMap<EntityTypeId, Vec<u8>>) =
            bincode::deserialize(scene).unwrap();
        Self {
            id,
            scene,
            ser_entities,
        }
    }

    pub fn deserialize_entity<ET: EntityType + serde::de::DeserializeOwned>(
        mut self,
        scope: EntityScope,
    ) -> Self {
        let type_id = ET::Entity::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            self.scene
                .entities
                .register_entity(scope, bincode::deserialize::<ET>(&data).unwrap());
        }
        self
    }

    pub fn deserialize_entities<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize_entity::<Entities<E>>(scope)
    }

    pub fn deserialize_single_entity<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize_entity::<SingleEntity<E>>(scope)
    }

    pub fn deserialize_grouped_entity<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize_entity::<GroupedEntities<Entities<E>>>(scope)
    }

    pub fn finish(self) -> Scene {
        assert!(
            self.ser_entities.is_empty(),
            "All components that were serialized should also be deserialized!"
        );
        self.scene
    }

    pub fn render_group<I: Instance>(
        mut self,
        name: &'static str,
        config: RenderGroupConfig,
    ) -> Self
    where
        Self: Sized,
    {
        self.scene = self.scene.render_group::<I>(name, config);
        self
    }
    pub fn render_group2d(self, name: &'static str, config: RenderGroupConfig) -> Self
    where
        Self: Sized,
    {
        self.render_group::<Instance2D>(name, config)
    }

    pub fn render_group3d(self, name: &'static str, config: RenderGroupConfig) -> Self
    where
        Self: Sized,
    {
        self.render_group::<Instance3D>(name, config)
    }

    pub fn single_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(SingleEntity::<E>::default(), scope)
    }

    pub fn entities<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(Entities::<E>::default(), scope)
    }

    pub fn grouped_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(GroupedEntities::<Entities<E>>::default(), scope)
    }

    pub fn entity<ET: EntityType>(mut self, ty: ET, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.scene = self.scene.entity(ty, scope);
        self
    }

    pub fn system(mut self, system: System) -> Self
    where
        Self: Sized,
    {
        self.scene = self.scene.system(system);
        self
    }
}
