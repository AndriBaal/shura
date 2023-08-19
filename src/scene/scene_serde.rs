use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    Component, ComponentManager, ComponentTypeId, Context, ContextUse, Gpu, Scene, SceneCreator,
    Shura, GLOBAL_GPU,
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
pub struct SerializedScene<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> {
    pub id: u32,
    pub scene: Vec<u8>,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> SerializedScene<N> {
    pub fn new(id: u32, scene: Vec<u8>, init: N) -> SerializedScene<N> {
        Self { id, scene, init }
    }
}

impl<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> SceneCreator for SerializedScene<N> {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, shura: &mut Shura) -> Scene {
        let (mut scene, ser_components): (Scene, FxHashMap<ComponentTypeId, Vec<u8>>) =
            bincode::deserialize(&self.scene).unwrap();
        scene.id = self.id;
        let mut de = SceneDeserializer::new(ser_components);
        let mut ctx = Context::new(shura, &mut scene, ContextUse::Update);
        (self.init)(&mut ctx, &mut de);
        return scene;
    }
}

#[derive(serde::Deserialize)]
/// Helper to deserialize [Components](crate::Component) and [States](crate::State) of a serialized [Scene]
pub struct SceneDeserializer {
    ser_components: FxHashMap<ComponentTypeId, Vec<u8>>,
}

impl SceneDeserializer {
    pub(crate) fn new(ser_components: FxHashMap<ComponentTypeId, Vec<u8>>) -> Self {
        Self { ser_components }
    }

    pub fn deserialize<C: serde::de::DeserializeOwned + Component>(&mut self, ctx: &mut Context) {
        let type_id = C::IDENTIFIER;
        if let Some(storage) = self.ser_components.remove(&type_id) {
            ctx.components.deserialize::<C>(storage)
        }
    }
}
