#[allow(private_interfaces)]

use std::{any::Any, cell::RefCell, sync::Arc};
use rustc_hash::FxHashMap;

use crate::{
    App, ComponentBufferImpl, Context, Entity, EntityManager,
    EntityType, EntityTypeGroup, EntityTypeId, EntityTypeImplementation, EntityTypeStorage, Gpu,
    Group, GroupHandle, GroupManager, Scene, SceneCreator, System, World, GLOBAL_GPU,
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

    pub fn serialize<E: Entity + serde::Serialize>(&mut self) {
        let ser = self.entities.serialize::<E>();
        self.ser_entities.insert(E::IDENTIFIER, ser);
    }
}

#[non_exhaustive]
pub struct SerializedScene {
    pub id: u32,
    pub scene: Scene,
    systems: Vec<System>,
    entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
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

    pub fn deserialize<E: serde::de::DeserializeOwned + Entity>(mut self) -> Self {
        let type_id = E::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            self.entities.push(Box::new(RefCell::new(
                bincode::deserialize::<EntityType<E>>(&data).unwrap(),
            )));
        }
        self
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

    fn entities(&mut self) -> &mut Vec<Box<RefCell<dyn EntityTypeImplementation>>> {
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

pub struct GroupSerializer {
    entities: FxHashMap<EntityTypeId, Box<dyn Any>>,
    ser_entities: FxHashMap<EntityTypeId, Vec<u8>>,
    group: Group,
}

impl GroupSerializer {
    pub fn new(
        world: &mut World,
        groups: &mut GroupManager,
        entities: &mut EntityManager,
        group: GroupHandle,
    ) -> Option<Self> {
        if let Some((group, entities)) = groups.remove_serialize(entities, world, group) {
            return Some(Self {
                group,
                entities,
                ser_entities: Default::default(),
            });
        }
        None
    }

    pub fn remove_serialize<E: Entity + Clone + serde::Serialize>(&mut self) {
        if let Some(data) = self.entities.remove(&E::IDENTIFIER) {
            let entities = data.downcast_ref::<EntityTypeGroup<E>>().unwrap();
            let data = bincode::serialize(entities).unwrap();
            self.ser_entities.insert(E::IDENTIFIER, data);
        }
    }

    pub fn finish(self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(&(self.group, self.ser_entities))
    }
}

pub struct GroupDeserializer {
    group: Group,
    ser_entities: FxHashMap<EntityTypeId, Vec<u8>>,
    pub(crate) entities: FxHashMap<EntityTypeId, Box<dyn Any>>,
    pub(crate) init_callbacks:
        Vec<Box<dyn FnOnce(&mut FxHashMap<EntityTypeId, Box<dyn Any>>, &mut Context)>>,
}

impl GroupDeserializer {
    pub fn new(data: &[u8]) -> Self {
        let (group, ser_entities): (Group, FxHashMap<EntityTypeId, Vec<u8>>) =
            bincode::deserialize(data).unwrap();
        Self {
            group,
            ser_entities,
            entities: Default::default(),
            init_callbacks: Default::default(),
        }
    }

    pub fn deserialize<E: serde::de::DeserializeOwned + Entity>(&mut self) {
        let type_id = E::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            let deserialized: EntityTypeStorage<E> = bincode::deserialize(&data).unwrap();
            self.entities.insert(type_id, Box::new(deserialized));
            self.init_callbacks.push(Box::new(|des, ctx| {
                if let Some(data) = des.remove(&E::IDENTIFIER) {
                    let storage = *data.downcast::<EntityTypeGroup<E>>().ok().unwrap();
                    ctx.entities.deserialize_group(storage, ctx.world);
                }
            }));
        }
    }

    pub(crate) fn finish(mut self, ctx: &mut Context) -> GroupHandle {
        let handle = ctx.groups.add(ctx.entities, self.group.clone());
        for call in self.init_callbacks.drain(..) {
            call(&mut self.entities, ctx);
        }
        handle
    }
}
