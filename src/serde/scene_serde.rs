use rustc_hash::FxHashMap;
use std::{ops::Deref, sync::Arc};

use crate::{
    entity::{
        Entities, ConstTypeId, EntityIdentifier, EntityManager, EntityScope, EntityType,
        GroupedEntities, SingleEntity, ConstIdentifier
    },
    graphics::{Gpu, GLOBAL_GPU},
    scene::{Scene, SceneCreator},
    system::System,
};

pub fn gpu() -> Arc<Gpu> {
    GLOBAL_GPU.get().unwrap().clone()
}

pub struct SceneSerializer<'a> {
    entities: &'a EntityManager,
    ser_entities: FxHashMap<ConstTypeId, Vec<u8>>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(entities: &'a EntityManager) -> Self {
        Self {
            entities,
            ser_entities: Default::default(),
        }
    }

    pub(crate) fn finish(self) -> FxHashMap<ConstTypeId, Vec<u8>> {
        self.ser_entities
    }

    pub fn serialize_entity_custom<ET: EntityType + serde::Serialize>(mut self) -> Self {
        let ser = self.entities.serialize::<ET>();
        self.ser_entities.insert(ET::Entity::IDENTIFIER, ser);
        self
    }

    pub fn serialize_entity_single<E: EntityIdentifier + serde::Serialize>(self) -> Self
    where
        Self: Sized,
    {
        self.serialize_entity_custom::<SingleEntity<E>>()
    }

    pub fn serialize_entity<E: EntityIdentifier + serde::Serialize>(self) -> Self
    where
        Self: Sized,
    {
        self.serialize_entity_custom::<Entities<E>>()
    }

    pub fn serialize_entity_grouped<E: EntityIdentifier + serde::Serialize>(self) -> Self
    where
        Self: Sized,
    {
        self.serialize_entity_custom::<GroupedEntities<Entities<E>>>()
    }

    pub fn serialize_entity_grouped_single<E: EntityIdentifier + serde::Serialize>(self) -> Self
    where
        Self: Sized,
    {
        self.serialize_entity_custom::<GroupedEntities<SingleEntity<E>>>()
    }
}

pub struct SerializedScene {
    pub id: u32,
    pub scene: Scene,
    once: bool,
    ser_entities: FxHashMap<ConstTypeId, Vec<u8>>,
}

impl SerializedScene {
    pub fn new<A: Deref<Target = [u8]>>(id: u32, scene: Option<A>) -> SerializedScene {
        let once = scene.is_none();
        let (scene, ser_entities): (Scene, FxHashMap<ConstTypeId, Vec<u8>>) =
            if let Some(scene) = scene {
                bincode::deserialize(&scene).unwrap()
            } else {
                (Scene::new(), Default::default())
            };
        Self {
            once,
            id,
            scene,
            ser_entities,
        }
    }

    pub fn deserialize_entity_custom<ET: EntityType + Default + serde::de::DeserializeOwned>(
        mut self,
        scope: EntityScope,
    ) -> Self {
        let type_id = ET::Entity::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            self.scene
                .entities
                .register_entity(scope, bincode::deserialize::<ET>(&data).unwrap());
        } else {
            return self.entity_custom(ET::default(), scope);
        }
        self
    }

    pub fn deserialize_entity<E: EntityIdentifier + serde::de::DeserializeOwned>(self) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<Entities<E>>(EntityScope::Scene)
    }

    pub fn deserialize_entity_single<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
    ) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<SingleEntity<E>>(EntityScope::Scene)
    }

    pub fn deserialize_entity_global<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
    ) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<Entities<E>>(EntityScope::Global)
    }

    pub fn deserialize_entity_single_global<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
    ) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<SingleEntity<E>>(EntityScope::Global)
    }

    pub fn deserialize_entity_grouped<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<GroupedEntities<Entities<E>>>(scope)
    }
    pub fn deserialize_entity_grouped_single<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self
    where
        Self: Sized,
    {
        self.deserialize_entity_custom::<GroupedEntities<SingleEntity<E>>>(scope)
    }

    pub fn system_once(self, system: System) -> Self {
        if self.once {
            return self.system(system);
        }
        self
    }

    pub fn once(&self) -> bool {
        self.once
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
