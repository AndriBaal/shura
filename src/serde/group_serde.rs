use rustc_hash::FxHashMap;

use crate::{
    context::Context,
    entity::{
        Entities, EntityGroup, EntityGroupHandle, EntityGroupManager, EntityId, EntityIdentifier,
        EntityManager, EntityType, GroupedEntities, SingleEntity,
    },
    physics::World,
};

pub struct EntityGroupSerializer {
    entities: FxHashMap<EntityId, Box<dyn EntityType>>,
    ser_entities: FxHashMap<EntityId, Vec<u8>>,
    group: EntityGroup,
}

impl EntityGroupSerializer {
    pub fn new(
        world: &mut World,
        groups: &mut EntityGroupManager,
        entities: &mut EntityManager,
        group: EntityGroupHandle,
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

    pub fn serialize<ET: EntityType + serde::Serialize>(&mut self) {
        if let Some(data) = self.entities.remove(&ET::Entity::IDENTIFIER) {
            let entities = data.downcast_ref::<ET>().unwrap();
            let data = bincode::serialize(entities).unwrap();
            self.ser_entities.insert(ET::Entity::IDENTIFIER, data);
        }
    }

    pub fn serialize_single<E: EntityIdentifier + serde::Serialize>(&mut self) {
        self.serialize::<SingleEntity<E>>()
    }

    pub fn serialize_multiple<E: EntityIdentifier + serde::Serialize>(&mut self) {
        self.serialize::<Entities<E>>()
    }

    pub fn serialize_groups<E: EntityIdentifier + serde::Serialize>(&mut self) {
        self.serialize::<GroupedEntities<Entities<E>>>()
    }

    pub fn finish(self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(&(self.group, self.ser_entities))
    }
}

pub struct EntityGroupDeserializer {
    group: EntityGroup,
    ser_entities: FxHashMap<EntityId, Vec<u8>>,
    pub(crate) init_callbacks: Vec<Box<dyn FnOnce(EntityGroupHandle, &mut Context)>>,
}

impl EntityGroupDeserializer {
    pub fn new(data: &[u8]) -> Self {
        let (group, ser_entities): (EntityGroup, FxHashMap<EntityId, Vec<u8>>) =
            bincode::deserialize(data).unwrap();
        Self {
            group,
            ser_entities,
            init_callbacks: Default::default(),
        }
    }

    pub fn deserialize<
        ET: EntityType<Entity = E> + serde::de::DeserializeOwned + Default,
        E: serde::de::DeserializeOwned + EntityIdentifier,
    >(
        &mut self,
    ) {
        let type_id = E::IDENTIFIER;
        if let Some(data) = self.ser_entities.remove(&type_id) {
            let deserialized: ET = bincode::deserialize(&data).unwrap();
            self.init_callbacks.push(Box::new(|group, ctx| {
                ctx.entities
                    .deserialize_group(group, deserialized, ctx.world);
            }));
        }
    }

    pub(crate) fn finish(mut self, ctx: &mut Context) -> EntityGroupHandle {
        let handle = ctx.groups.add(ctx.entities, self.group.clone());
        for call in self.init_callbacks.drain(..) {
            call(handle, ctx);
        }
        handle
    }
}
