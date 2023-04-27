#[cfg(feature = "physics")]
use crate::physics::RigidBodyHandle;
use bincode::{
    config::{AllowTrailing, FixintEncoding, WithOtherIntEncoding, WithOtherTrailing},
    de::read::SliceReader,
    DefaultOptions, Options,
};
use rustc_hash::FxHashMap;
#[cfg(feature = "physics")]
use rustc_hash::FxHashSet;
use serde::{de::Visitor, Deserializer, Serialize};
use std::{cmp, marker::PhantomData};

use crate::{
    Arena, ArenaEntry, BoxedComponent, ComponentController, ComponentFilter, ComponentGroup,
    ComponentManager, ComponentTypeId, Context, FieldNames, GlobalStateController, Scene,
    SceneCreator, SceneStateController, ShuraFields,
};

pub struct SceneSerializer<'a> {
    global_state: &'a Box<dyn GlobalStateController>,
    scene_state: &'a Box<dyn SceneStateController>,

    groups: Vec<Option<(&'a u32, &'a ComponentGroup)>>,
    ser_components: FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
    ser_scene_state: Option<Vec<u8>>,
    ser_global_state: Option<Vec<u8>>,

    #[cfg(feature = "physics")]
    body_handles: FxHashSet<RigidBodyHandle>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(
        component_manager: &'a ComponentManager,
        global_state: &'a Box<dyn GlobalStateController>,
        scene_state: &'a Box<dyn SceneStateController>,
        filter: ComponentFilter,
    ) -> Self {
        let groups = component_manager.serialize_groups(filter);
        Self {
            groups,
            global_state,
            scene_state,
            ser_components: Default::default(),
            ser_scene_state: None,
            ser_global_state: None,
            #[cfg(feature = "physics")]
            body_handles: Default::default(),
        }
    }

    #[cfg(feature = "physics")]
    pub(crate) fn finish(
        self,
    ) -> (
        Vec<Option<(&'a u32, &'a ComponentGroup)>>,
        FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        FxHashSet<RigidBodyHandle>,
    ) {
        (
            self.groups,
            self.ser_components,
            self.ser_scene_state,
            self.ser_global_state,
            self.body_handles,
        )
    }

    #[cfg(not(feature = "physics"))]
    pub(crate) fn finish(
        self,
    ) -> (
        Vec<Option<(&'a u32, &'a ComponentGroup)>>,
        FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
    ) {
        (
            self.groups,
            self.ser_components,
            self.ser_scene_state,
            self.ser_global_state,
        )
    }

    pub fn serialize_components<C: ComponentController + serde::Serialize>(&mut self) {
        let type_id = C::IDENTIFIER;
        self.ser_components.insert(type_id, vec![]);
        let ty = self.ser_components.get_mut(&type_id).unwrap();
        for group in &self.groups {
            if let Some((_, group)) = group {
                if let Some(type_index) = group.type_index(type_id) {
                    let type_ref = group.type_ref(*type_index).unwrap();
                    let ser_components = type_ref.serialize_components::<C>();
                    ty.push((group.id(), ser_components));
                    #[cfg(feature = "physics")]
                    for (_, component) in type_ref {
                        if let Some(body_handle) = component.base().try_body_handle() {
                            self.body_handles.insert(body_handle);
                        }
                    }
                }
            }
        }
    }

    pub fn serialize_global_state<G: GlobalStateController + Serialize>(&mut self) {
        self.ser_global_state =
            bincode::serialize(self.global_state.downcast_ref::<G>().unwrap()).ok();
    }

    pub fn serialize_scene_state<S: SceneStateController + Serialize>(&mut self) {
        self.ser_scene_state =
            bincode::serialize(self.scene_state.downcast_ref::<S>().unwrap()).ok();
    }
}

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
    fn id(&self) -> u32 {
        self.id
    }

    fn create(mut self, shura: ShuraFields) -> Scene {
        let (mut scene, groups, ser_components, ser_scene_state, ser_global_state): (
            Scene,
            Arena<ComponentGroup>,
            FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
        ) = bincode::deserialize(&self.scene).unwrap();
        scene.component_manager.deserialize_groups(groups);
        let mut de = SceneDeserializer::new(ser_components, ser_scene_state, ser_global_state);
        let mut ctx = Context::from_fields(shura, &mut scene);
        (self.init)(&mut ctx, &mut de);
        return scene;
    }
}

#[derive(serde::Deserialize)]
pub struct SceneDeserializer {
    ser_components: FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
    ser_scene_state: Option<Vec<u8>>,
    ser_global_state: Option<Vec<u8>>,
}

impl SceneDeserializer {
    pub(crate) fn new(
        ser_components: FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
        ser_scene_state: Option<Vec<u8>>,
        ser_global_state: Option<Vec<u8>>,
    ) -> Self {
        Self {
            ser_components,
            ser_scene_state,
            ser_global_state,
        }
    }

    // pub(crate) fn finish(&self) {
    //     assert!(
    //         self.ser_components.is_empty(),
    //         "All components need to be deserialized!"
    //     );
    // }

    pub fn deserialize_components<C: serde::de::DeserializeOwned + ComponentController>(
        &mut self,
        ctx: &mut Context,
    ) {
        let type_id = C::IDENTIFIER;
        ctx.component_manager.register_callbacks::<C>();
        let components = self.ser_components.remove(&type_id).unwrap();
        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<BoxedComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        #[allow(unused_mut)]
                        let mut component: BoxedComponent =
                            Box::new(bincode::deserialize::<C>(&data).unwrap());

                        #[cfg(feature = "physics")]
                        {
                            if component.base().is_body() {
                                component
                                    .base_mut()
                                    .init_body(ctx.component_manager.world.clone());
                            }
                        }
                        ArenaEntry::Occupied {
                            generation: gen,
                            data: component,
                        }
                    }
                    None => ArenaEntry::Free { next_free: None },
                };
                items.push(item);
            }

            let group = ctx.group_mut(group_id).unwrap();
            let components = Arena::from_items(items, generation);
            group.deserialize_type::<C>(components);
        }
    }

    pub fn deserialize_components_with<C: ComponentController + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, C>, &'de Context<'de>) -> C,
    ) {
        let type_id = C::IDENTIFIER;
        ctx.component_manager.register_callbacks::<C>();
        let components = self.ser_components.remove(&type_id).unwrap();

        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<BoxedComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        let wrapper = DeserializeWrapper::new(&data);
                        #[allow(unused_mut)]
                        let mut component: BoxedComponent = Box::new((de)(wrapper, ctx));
                        #[cfg(feature = "physics")]
                        if component.base().is_body() {
                            component
                                .base_mut()
                                .init_body(ctx.component_manager.world.clone());
                        }
                        ArenaEntry::Occupied {
                            generation: gen,
                            data: component,
                        }
                    }
                    None => ArenaEntry::Free { next_free: None },
                };
                items.push(item);
            }

            let group = ctx.group_mut(group_id).unwrap();
            let components = Arena::from_items(items, generation);
            group.deserialize_type::<C>(components);
        }
    }

    pub fn deserialize_global_state<G: GlobalStateController + serde::de::DeserializeOwned>(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(ser_global_state) = self.ser_global_state.take() {
            let de: G = bincode::deserialize(&ser_global_state).unwrap();
            ctx.set_global_state(de);
        }
    }

    pub fn deserialize_scene_state<S: SceneStateController + serde::de::DeserializeOwned>(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(ser_scene_state) = self.ser_scene_state.take() {
            let de: S = bincode::deserialize(&ser_scene_state).unwrap();
            ctx.set_scene_state(de);
        }
    }

    pub fn deserialize_global_state_with<G: GlobalStateController + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, G>, &'de Context<'de>) -> G,
    ) {
        if let Some(ser_global_state) = self.ser_global_state.take() {
            let wrapper = DeserializeWrapper::new(&ser_global_state);
            let state: G = (de)(wrapper, ctx);
            ctx.set_global_state(state);
        }
    }
    pub fn deserialize_scene_state_with<S: SceneStateController + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, S>, &'de Context<'de>) -> S,
    ) {
        if let Some(ser_scene_state) = self.ser_scene_state.take() {
            let wrapper = DeserializeWrapper::new(&ser_scene_state);
            let state: S = (de)(wrapper, ctx);
            ctx.set_scene_state(state);
        }
    }
}

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
