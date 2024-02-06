use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::{
    entity::{
        Entities, EntityId, EntityIdentifier, EntityManager, EntityScope, EntityStorage,
        EntityType, GroupedEntities, SingleEntity,
    },
    graphics::{Gpu, GLOBAL_GPU},
    scene::{Scene, SceneCreator},
};

pub fn gpu() -> Arc<Gpu> {
    GLOBAL_GPU.get().unwrap().clone()
}

pub struct SceneSerializer<'a> {
    entities: &'a EntityManager,
    ser_entities: FxHashMap<EntityId, Vec<u8>>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(entities: &'a EntityManager) -> Self {
        Self {
            entities,
            ser_entities: Default::default(),
        }
    }

    pub(crate) fn finish(self) -> FxHashMap<EntityId, Vec<u8>> {
        self.ser_entities
    }

    pub fn serialize_custom_entity<ET: EntityStorage + serde::Serialize>(mut self) -> Self {
        let ser = self.entities.serialize::<ET>();
        self.ser_entities.insert(ET::Entity::IDENTIFIER, ser);
        self
    }

    pub fn serialize_entity<E: EntityIdentifier + serde::Serialize>(
        self,
        storage: EntityType,
    ) -> Self
    where
        Self: Sized,
    {
        match storage {
            EntityType::Single => self.serialize_custom_entity::<SingleEntity<E>>(),
            EntityType::Multiple => self.serialize_custom_entity::<Entities<E>>(),
            EntityType::Groups => self.serialize_custom_entity::<GroupedEntities<Entities<E>>>(),
        }
    }
}

#[non_exhaustive]
pub struct SerializedScene {
    pub id: u32,
    pub scene: Scene,
    ser_entities: FxHashMap<EntityId, Vec<u8>>,
}

impl SerializedScene {
    pub fn new(id: u32, scene: &[u8]) -> SerializedScene {
        let (scene, ser_entities): (Scene, FxHashMap<EntityId, Vec<u8>>) =
            bincode::deserialize(scene).unwrap();
        Self {
            id,
            scene,
            ser_entities,
        }
    }

    pub fn deserialize_custom_entity<ET: EntityStorage + serde::de::DeserializeOwned>(
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

    pub fn deserialize_entity<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        storage: EntityType,
        scope: EntityScope,
    ) -> Self
    where
        Self: Sized,
    {
        match storage {
            EntityType::Single => self.deserialize_custom_entity::<SingleEntity<E>>(scope),
            EntityType::Multiple => self.deserialize_custom_entity::<Entities<E>>(scope),
            EntityType::Groups => {
                self.deserialize_custom_entity::<GroupedEntities<Entities<E>>>(scope)
            }
        }
    }

    pub fn finish(self) -> Scene {
        assert!(
            self.ser_entities.is_empty(),
            "All components that were serialized should also be deserialized!"
        );
        self.scene
    }
}

impl From<SerializedScene> for Scene {
    fn from(ser: SerializedScene) -> Self {
        ser.scene
    }
}

impl SceneCreator for SerializedScene {
    fn scene(&mut self) -> &mut Scene {
        &mut self.scene
    }
}
