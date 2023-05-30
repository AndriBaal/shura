use bincode::{
    config::{AllowTrailing, FixintEncoding, WithOtherIntEncoding, WithOtherTrailing},
    de::read::SliceReader,
    DefaultOptions, Options,
};
use rustc_hash::FxHashMap;
use serde::{de::Visitor, Deserializer, Serialize};
use std::{cmp, marker::PhantomData};

use crate::{
    Arena, ArenaEntry, BoxedComponent, ComponentController, ComponentManager, ComponentTypeId,
    Context, FieldNames, GlobalStateController, GlobalStateManager, GroupHandle, Scene,
    SceneCreator, SceneStateController, SceneStateManager, Shura, StateIdentifier, StateTypeId,
};

/// Helper to serialize [Components](crate::Component) and [States](crate::State) of a [Scene]
pub struct SceneSerializer<'a> {
    components: &'a ComponentManager,
    global_states: &'a GlobalStateManager,
    scene_states: &'a SceneStateManager,

    ser_components: FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
    ser_scene_states: FxHashMap<StateTypeId, Vec<u8>>,
    ser_global_states: FxHashMap<StateTypeId, Vec<u8>>
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(
        components: &'a ComponentManager,
        global_states: &'a GlobalStateManager,
        scene_states: &'a SceneStateManager,
    ) -> Self {
        Self {
            components,
            global_states,
            scene_states,
            ser_components: Default::default(),
            ser_scene_states: Default::default(),
            ser_global_states: Default::default()
        }
    }

    pub(crate) fn finish(
        self,
    ) -> (
        FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
        FxHashMap<StateTypeId, Vec<u8>>,
        FxHashMap<StateTypeId, Vec<u8>>,
    ) {
        (
            self.ser_components,
            self.ser_scene_states,
            self.ser_global_states,
        )
    }


    pub fn serialize_components<C: ComponentController + serde::Serialize>(&mut self) {
        let ty = self.components.type_ref::<C>();
        let mut group_data = vec![];
        for (group_handle, group) in &ty.groups {
            let ser_components = group.components.serialize_components::<C>();
            group_data.push((GroupHandle(group_handle), ser_components));
        }
        self.ser_components.insert(C::IDENTIFIER, group_data);
    }

    pub fn serialize_global_state<G: GlobalStateController + StateIdentifier + Serialize>(
        &mut self,
    ) {
        if let Some(state) = self.global_states.try_get::<G>() {
            self.ser_global_states
                .insert(G::IDENTIFIER, bincode::serialize(state).unwrap());
        }
    }

    pub fn serialize_scene_state<S: SceneStateController + StateIdentifier + Serialize>(&mut self) {
        if let Some(state) = self.scene_states.try_get::<S>() {
            self.ser_scene_states
                .insert(S::IDENTIFIER, bincode::serialize(state).unwrap());
        }
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
        let (mut scene, ser_components, ser_scene_state, ser_global_state): (
            Scene,
            FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
            FxHashMap<StateTypeId, Vec<u8>>,
            FxHashMap<StateTypeId, Vec<u8>>,
        ) = bincode::deserialize(&self.scene).unwrap();
        scene.id = self.id;
        let mut de = SceneDeserializer::new(ser_components, ser_scene_state, ser_global_state);
        let mut ctx = Context::new(shura, &mut scene);
        (self.init)(&mut ctx, &mut de);
        return scene;
    }
}

#[derive(serde::Deserialize)]
/// Helper to deserialize [Components](crate::Component) and [States](crate::State) of a serialized [Scene]
pub struct SceneDeserializer {
    ser_components: FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
    ser_scene_states: FxHashMap<StateTypeId, Vec<u8>>,
    ser_global_states: FxHashMap<StateTypeId, Vec<u8>>,
}

impl SceneDeserializer {
    pub(crate) fn new(
        ser_components: FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
        ser_scene_states: FxHashMap<StateTypeId, Vec<u8>>,
        ser_global_states: FxHashMap<StateTypeId, Vec<u8>>,
    ) -> Self {
        Self {
            ser_components,
            ser_scene_states,
            ser_global_states,
        }
    }

    pub fn deserialize_components<C: serde::de::DeserializeOwned + ComponentController>(
        &mut self,
        ctx: &mut Context,
    ) {
        ctx.components.reregister::<C>();
        let type_id = C::IDENTIFIER;
        let ty = ctx.components.type_mut::<C>();
        if let Some(components) = self.ser_components.remove(&type_id) {
            for (group_id, components) in components {
                let mut items: Vec<ArenaEntry<BoxedComponent>> =
                    Vec::with_capacity(components.capacity());
                let mut generation = 0;
                for component in components {
                    let item = match component {
                        Some((gen, data)) => {
                            generation = cmp::max(generation, gen);
                            let component: BoxedComponent =
                                Box::new(bincode::deserialize::<C>(&data).unwrap());

                            ArenaEntry::Occupied {
                                generation: gen,
                                data: component,
                            }
                        }
                        None => ArenaEntry::Free { next_free: None },
                    };
                    items.push(item);
                }

                let components = Arena::from_items(items, generation);
                ty.groups[group_id.0].components = components;
            }
        }
    }

    pub fn deserialize_components_with<C: ComponentController + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, C>, &'de Context<'de>) -> C,
    ) {
        ctx.components.reregister::<C>();
        let type_id = C::IDENTIFIER;
        if let Some(components) = self.ser_components.remove(&type_id) {
            for (group_id, components) in components {
                let mut items: Vec<ArenaEntry<BoxedComponent>> =
                    Vec::with_capacity(components.capacity());
                let mut generation = 0;
                for component in components {
                    let item = match component {
                        Some((gen, data)) => {
                            generation = cmp::max(generation, gen);
                            let wrapper = DeserializeWrapper::new(&data);
                            let component: BoxedComponent = Box::new((de)(wrapper, ctx));
                            // #[cfg(feature = "physics")]
                            // if component.base().is_body() {
                            //     component.base_mut().init_body(ctx.components.world.clone());
                            // }
                            ArenaEntry::Occupied {
                                generation: gen,
                                data: component,
                            }
                        }
                        None => ArenaEntry::Free { next_free: None },
                    };
                    items.push(item);
                }

                let ty = ctx.components.type_mut::<C>();
                let components = Arena::from_items(items, generation);
                ty.groups[group_id.0].components = components;
            }
        }
    }

    pub fn deserialize_global_state<
        G: GlobalStateController + StateIdentifier + serde::de::DeserializeOwned,
    >(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(ser_global_state) = self.ser_global_states.get(&G::IDENTIFIER) {
            let de: G = bincode::deserialize(&ser_global_state).unwrap();
            ctx.global_states.insert(de);
        }
    }

    pub fn deserialize_scene_state<
        S: SceneStateController + StateIdentifier + serde::de::DeserializeOwned,
    >(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(ser_scene_state) = self.ser_scene_states.get(&S::IDENTIFIER) {
            let de: S = bincode::deserialize(&ser_scene_state).unwrap();
            ctx.scene_states.insert(de);
        }
    }

    pub fn deserialize_global_state_with<
        G: GlobalStateController + StateIdentifier + FieldNames,
    >(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, G>, &'de Context<'de>) -> G,
    ) {
        if let Some(ser_global_state) = self.ser_global_states.get(&G::IDENTIFIER) {
            let wrapper = DeserializeWrapper::new(&ser_global_state);
            let state: G = (de)(wrapper, ctx);
            ctx.global_states.insert(state);
        }
    }
    pub fn deserialize_scene_state_with<S: SceneStateController + StateIdentifier + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, S>, &'de Context<'de>) -> S,
    ) {
        if let Some(ser_scene_state) = self.ser_scene_states.get(&S::IDENTIFIER) {
            let wrapper = DeserializeWrapper::new(&ser_scene_state);
            let state: S = (de)(wrapper, ctx);
            ctx.scene_states.insert(state);
        }
    }
}

/// Wrapper to deserialize structs deriving from [component](crate::Component) or [state](crate::StateIdentifier)
pub struct DeserializeWrapper<'de, F: FieldNames> {
    de: bincode::Deserializer<
        SliceReader<'de>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    >,
    _marker: PhantomData<F>,
}

impl<'de, F: FieldNames> DeserializeWrapper<'de, F> {
    pub(crate) fn new(data: &'de [u8]) -> Self {
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let de = bincode::Deserializer::from_slice(&data, options);
        Self {
            de,
            _marker: PhantomData::<F>,
        }
    }

    pub fn deserialize(mut self, visitor: impl Visitor<'de, Value = F>) -> F {
        self.de.deserialize_struct("", F::FIELDS, visitor).unwrap()
    }
}
