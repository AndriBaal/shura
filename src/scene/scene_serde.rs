use std::{any::Any, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    Component, ComponentConfig, ComponentManager, ComponentTypeGroup, ComponentTypeId,
    ComponentTypeStorage, Context, ContextUse, Gpu, Group, GroupHandle, GroupManager, Scene,
    SceneCreator, App, World, GLOBAL_GPU,
};

pub fn gpu() -> Arc<Gpu> {
    GLOBAL_GPU.get().unwrap().clone()
}

/// Helper to serialize [Components](crate::Component) and [States](crate::State) of a [Scene]
pub struct SceneSerializer<'a> {
    components: &'a ComponentManager,
    ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(components: &'a ComponentManager) -> Self {
        Self {
            components,
            ser_components: Default::default(),
        }
    }

    pub(crate) fn finish(self) -> FxHashMap<ComponentTypeId, Vec<u8>> {
        self.ser_components
    }

    pub fn serialize<C: Component + serde::Serialize>(&mut self) {
        let ser = self.components.serialize::<C>();
        self.ser_components.insert(C::IDENTIFIER, ser);
    }
}

/// Reload a [Scene] from its serialized state
#[non_exhaustive]
pub struct SerializedScene<N: 'static + FnMut(&mut Context)> {
    pub id: u32,
    pub scene: Scene,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context)> SerializedScene<N> {
    pub fn new(
        id: u32,
        scene: &[u8],
        deserialize: impl FnOnce(&mut SceneDeserializer),
        init: N,
    ) -> SerializedScene<N> {
        let (mut scene, ser_components): (Scene, FxHashMap<ComponentTypeId, Vec<u8>>) =
            bincode::deserialize(&scene).unwrap();
        scene.id = id;
        let mut de = SceneDeserializer::new(&mut scene, ser_components);
        (deserialize)(&mut de);
        Self { id, scene, init }
    }
}

impl<N: 'static + FnMut(&mut Context)> SceneCreator for SerializedScene<N> {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, app: &mut App) -> Scene {
        let mut ctx = Context::new(app, &mut self.scene, ContextUse::Update);
        (self.init)(&mut ctx);
        return self.scene;
    }
}

/// Helper to deserialize [Components](crate::Component) and [States](crate::State) of a serialized [Scene]
pub struct SceneDeserializer<'a> {
    ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
    pub scene: &'a mut Scene,
}

impl<'a> SceneDeserializer<'a> {
    pub(crate) fn new(
        scene: &'a mut Scene,
        ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
    ) -> Self {
        Self {
            scene,
            ser_components,
        }
    }

    pub fn register<C: Component>(&mut self) {
        self.register_with_config::<C>(C::CONFIG);
    }

    pub fn register_with_config<C: Component>(&mut self, config: ComponentConfig) {
        self.scene
            .components
            .register_with_config::<C>(&self.scene.groups, config);
    }

    pub fn deserialize<C: serde::de::DeserializeOwned + Component>(&mut self) {
        let type_id = C::IDENTIFIER;
        if let Some(data) = self.ser_components.remove(&type_id) {
            self.scene.components.deserialize::<C>(data)
        }
    }
}

pub struct GroupSerializer {
    components: FxHashMap<ComponentTypeId, Box<dyn Any>>,
    ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
    group: Group,
}

impl GroupSerializer {
    pub fn new(
        world: &mut World,
        groups: &mut GroupManager,
        components: &mut ComponentManager,
        group: GroupHandle,
    ) -> Option<Self> {
        if let Some((group, components)) = groups.remove_serialize(components, world, group) {
            return Some(Self {
                group,
                components,
                ser_components: Default::default(),
            });
        }
        return None;
    }

    pub fn remove_serialize<C: Component + Clone + serde::Serialize>(&mut self) {
        if let Some(data) = self.components.remove(&C::IDENTIFIER) {
            let components = data.downcast_ref::<ComponentTypeGroup<C>>().unwrap();
            let data = bincode::serialize(components).unwrap();
            self.ser_components.insert(C::IDENTIFIER, data);
        }
    }

    pub fn finish(self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        return bincode::serialize(&(self.group, self.ser_components));
    }
}

pub struct GroupDeserializer {
    group: Group,
    ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
    pub(crate) components: FxHashMap<ComponentTypeId, Box<dyn Any>>,
    pub(crate) init_callbacks:
        Vec<Box<dyn FnOnce(&mut FxHashMap<ComponentTypeId, Box<dyn Any>>, &mut Context)>>,
}

impl GroupDeserializer {
    pub fn new(data: &[u8]) -> Self {
        let (group, ser_components): (Group, FxHashMap<ComponentTypeId, Vec<u8>>) =
            bincode::deserialize(&data).unwrap();
        Self {
            group,
            ser_components,
            components: Default::default(),
            init_callbacks: Default::default(),
        }
    }

    pub fn deserialize<C: serde::de::DeserializeOwned + Component>(&mut self) {
        let type_id = C::IDENTIFIER;
        if let Some(data) = self.ser_components.remove(&type_id) {
            let deserialized: ComponentTypeStorage<C> = bincode::deserialize(&data).unwrap();
            self.components.insert(type_id, Box::new(deserialized));
            self.init_callbacks.push(Box::new(|des, ctx| {
                if let Some(data) = des.remove(&C::IDENTIFIER) {
                    let storage = *data.downcast::<ComponentTypeGroup<C>>().ok().unwrap();
                    ctx.components.deserialize_group(storage, ctx.world);
                }
            }));
        }
    }

    pub(crate) fn finish(mut self, ctx: &mut Context) -> GroupHandle {
        let handle = ctx.groups.add(ctx.components, self.group.clone());
        for call in self.init_callbacks.drain(..) {
            call(&mut self.components, ctx);
        }
        return handle;
    }
}
