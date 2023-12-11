use rustc_hash::FxHashMap;
#[allow(private_interfaces)]
use std::{cell::RefCell, sync::Arc};

use crate::{
    app::{App, GLOBAL_GPU},
    context::Context,
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityScope, EntityType, EntityTypeId,
        GroupedEntities,
    },
    graphics::{ComponentBufferImpl, Gpu},
    scene::{Scene, SceneCreator},
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

    pub fn serialize<ET: EntityType + serde::Serialize>(&mut self) {
        let ser = self.entities.serialize::<ET>();
        self.ser_entities.insert(ET::Entity::IDENTIFIER, ser);
    }
}

#[non_exhaustive]
pub struct SerializedScene {
    pub id: u32,
    pub scene: Scene,
    systems: Vec<System>,
    entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
    ser_entities: FxHashMap<EntityTypeId, Vec<u8>>,
    component_buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl SerializedScene {
    pub fn new(id: u32, scene: &[u8]) -> SerializedScene {
        let (scene, ser_entities): (Scene, FxHashMap<EntityTypeId, Vec<u8>>) =
            bincode::deserialize(scene).unwrap();
        Self {
            id,
            scene,
            ser_entities,
            systems: Default::default(),
            entities: Default::default(),
            component_buffers: Default::default(),
        }
    }

    pub fn deserialize<ET: EntityType + serde::de::DeserializeOwned>(
        mut self,
        scope: EntityScope,
    ) -> Self {
        let type_id = ET::Entity::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            self.entities.push((
                scope,
                Box::new(RefCell::new(bincode::deserialize::<ET>(&data).unwrap())),
            ));
        }
        self
    }

    pub fn deserialize_multiple<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize::<Entities<E>>(scope)
    }

    pub fn deserialize_single<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize::<Entities<E>>(scope)
    }

    pub fn deserialize_groups<E: EntityIdentifier + serde::de::DeserializeOwned>(
        self,
        scope: EntityScope,
    ) -> Self {
        self.deserialize::<GroupedEntities<Entities<E>>>(scope)
    }
}

#[allow(private_interfaces)]
impl SceneCreator for SerializedScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn systems(&mut self) -> &mut Vec<System> {
        &mut self.systems
    }

    fn entities(&mut self) -> &mut Vec<(EntityScope, Box<RefCell<dyn EntityType>>)> {
        &mut self.entities
    }

    fn components(&mut self) -> &mut FxHashMap<&'static str, Box<dyn ComponentBufferImpl>> {
        &mut self.component_buffers
    }

    fn create(mut self: Box<Self>, app: &mut App) -> Scene {
        self.scene.component_buffers.init(self.component_buffers);
        self.scene.entities.init(&app.globals, self.entities);
        self.scene.systems.init(&self.systems);
        let (_, mut ctx) = Context::new(&self.id, app, &mut self.scene);
        for system in &self.systems {
            match system {
                System::Setup(setup) => {
                    (setup)(&mut ctx);
                }
                _ => (),
            }
        }
        self.scene
    }
}
