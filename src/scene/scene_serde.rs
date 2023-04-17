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
    Arena, ArenaEntry, BoxedComponent, ComponentController, ComponentDerive, ComponentGroup,
    ComponentManager, ComponentTypeId, Context, FieldNames, GlobalState, GroupFilter, Scene,
    SceneCreator, SceneState, ShuraFields,
};

pub(crate) type SerializedGroups = FxHashMap<
    u16,
    (
        Vec<u8>,
        FxHashMap<ComponentTypeId, Vec<Option<(u32, Vec<u8>)>>>,
    ),
>;

pub struct SceneSerializer<'a> {
    global_state: &'a Box<dyn GlobalState>,
    scene_state: &'a Box<dyn SceneState>,
    groups: Vec<&'a ComponentGroup>,

    ser_groups: SerializedGroups,
    ser_scene_state: Option<Vec<u8>>,
    ser_global_state: Option<Vec<u8>>,

    #[cfg(feature = "physics")]
    body_handles: FxHashSet<RigidBodyHandle>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(
        component_manager: &'a ComponentManager,
        global_state: &'a Box<dyn GlobalState>,
        scene_state: &'a Box<dyn SceneState>,
        group_filter: GroupFilter<'a>,
    ) -> Self {
        let mut ser_groups: SerializedGroups = Default::default();
        let mut groups = vec![];
        match group_filter {
            GroupFilter::All => {
                for group_id in component_manager.group_ids() {
                    if let Some(group) = component_manager.group_by_id(*group_id) {
                        ser_groups.insert(
                            *group_id,
                            (bincode::serialize(group).unwrap(), Default::default()),
                        );
                        groups.push(group);
                    }
                }
            }
            GroupFilter::Active => {
                for group_id in component_manager.active_group_ids() {
                    if let Some(group) = component_manager.group_by_id(*group_id) {
                        ser_groups.insert(
                            *group_id,
                            (bincode::serialize(group).unwrap(), Default::default()),
                        );
                        groups.push(group);
                    }
                }
            }
            GroupFilter::Specific(group_ids) => {
                for group_id in group_ids {
                    if let Some(group) = component_manager.group_by_id(*group_id) {
                        ser_groups.insert(
                            *group_id,
                            (bincode::serialize(group).unwrap(), Default::default()),
                        );
                        groups.push(group);
                    }
                }
            }
        }
        Self {
            groups,
            global_state,
            scene_state,
            ser_groups,
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
        SerializedGroups,
        FxHashSet<RigidBodyHandle>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
    ) {
        (
            self.ser_groups,
            self.body_handles,
            self.ser_scene_state,
            self.ser_global_state,
        )
    }

    #[cfg(not(feature = "physics"))]
    pub(crate) fn finish(self) -> SerializedGroups {
        self.ser_groups
    }

    pub fn serialize_components<C: ComponentController + serde::Serialize>(&mut self) {
        let type_id = C::IDENTIFIER;
        for group in &self.groups {
            let group_ser = self.ser_groups.get_mut(&group.id()).unwrap();
            if let Some(type_index) = group.type_index(type_id) {
                let type_ref = group.type_ref(*type_index).unwrap();
                let ser_components = type_ref.serialize_components::<C>();
                group_ser.1.insert(type_id, ser_components);
                #[cfg(feature = "physics")]
                for (_, component) in type_ref {
                    if let Some(body_handle) = component.base().rigid_body_handle() {
                        self.body_handles.insert(body_handle);
                    }
                }
            }
        }
    }

    pub fn serialize_global_state<G: GlobalState + Serialize>(&mut self) {
        self.ser_global_state =
            bincode::serialize(self.global_state.downcast_ref::<G>().unwrap()).ok();
    }

    pub fn serialize_scene_state<S: SceneState + Serialize>(&mut self) {
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
        let (mut scene, groups, scene_state, global_state): (
            Scene,
            SerializedGroups,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
        ) = bincode::deserialize(&self.scene).unwrap();
        let mut de = SceneDeserializer::new(
            &mut scene.component_manager,
            groups,
            scene_state,
            global_state,
        );
        let mut ctx = Context::from_fields(shura, &mut scene);
        (self.init)(&mut ctx, &mut de);
        de.finish();
        return scene;
    }
}

#[derive(serde::Deserialize)]
pub struct SceneDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>>,
    scene_state: Option<Vec<u8>>,
    global_state: Option<Vec<u8>>,
}

impl SceneDeserializer {
    pub(crate) fn new(
        component_manager: &mut ComponentManager,
        groups: SerializedGroups,
        scene_state: Option<Vec<u8>>,
        global_state: Option<Vec<u8>>,
    ) -> Self {
        let mut components: FxHashMap<ComponentTypeId, Vec<(u16, Vec<Option<(u32, Vec<u8>)>>)>> =
            Default::default();
        for (group_id, (ser_group, component_types)) in groups {
            let group: ComponentGroup = bincode::deserialize(&ser_group).unwrap();
            component_manager.add_group(group);
            for (type_id, c) in component_types {
                let t = components.entry(type_id).or_insert(vec![]);
                t.push((group_id, c));
            }
        }
        Self {
            components,
            scene_state,
            global_state,
        }
    }

    pub(crate) fn finish(&self) {
        assert!(
            self.components.is_empty(),
            "All components need to be deserialized!"
        );
    }

    pub fn deserialize_components<C: serde::de::DeserializeOwned + ComponentController>(
        &mut self,
        ctx: &mut Context,
    ) {
        let type_id = C::IDENTIFIER;
        ctx.component_manager.register_callbacks::<C>();
        let components = self.components.remove(&type_id).unwrap();
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
                            if component.base().is_rigid_body() {
                                component
                                    .base_mut()
                                    .init_rigid_body(ctx.component_manager.world.clone());
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
        let components = self.components.remove(&type_id).unwrap();

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
                        if component.base().is_rigid_body() {
                            component
                                .base_mut()
                                .init_rigid_body(ctx.component_manager.world.clone());
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

    pub fn deserialize_global_state<G: GlobalState + serde::de::DeserializeOwned>(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(global_state) = self.global_state.take() {
            let de: G = bincode::deserialize(&global_state).unwrap();
            ctx.set_global_state(de);
        }
    }

    pub fn deserialize_scene_state<S: SceneState + serde::de::DeserializeOwned>(
        &mut self,
        ctx: &mut Context,
    ) {
        if let Some(scene_state) = self.scene_state.take() {
            let de: S = bincode::deserialize(&scene_state).unwrap();
            ctx.set_scene_state(de);
        }
    }

    pub fn deserialize_global_state_with<G: GlobalState + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, G>, &'de Context<'de>) -> G,
    ) {
        if let Some(data) = self.global_state.take() {
            let wrapper = DeserializeWrapper::new(&data);
            let state: G = (de)(wrapper, ctx);
            ctx.set_global_state(state);
        }
    }
    pub fn deserialize_scene_state_with<S: SceneState + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, S>, &'de Context<'de>) -> S,
    ) {
        if let Some(data) = self.scene_state.take() {
            let wrapper = DeserializeWrapper::new(&data);
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
