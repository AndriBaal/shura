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
    Arena, ArenaEntry, ComponentController, ComponentManager, ComponentTypeId, Context,
    DynamicComponent, FieldNames, GlobalState, GroupFilter, Scene, SceneCreator, SceneState,
    ShuraFields, Vector,
};

pub struct SceneSerializer<'a> {
    component_manager: &'a ComponentManager,
    global_state: &'a Box<dyn GlobalState>,
    scene_state: &'a Box<dyn SceneState>,
    organized_components:
        FxHashMap<ComponentTypeId, Vec<(u32 /* Group id */, Vec<Option<(u32, Vec<u8>)>>)>>,
    #[cfg(feature = "physics")]
    body_handles: FxHashSet<RigidBodyHandle>,
    ser_scene_state: Option<Vec<u8>>,
    ser_global_state: Option<Vec<u8>>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(
        component_manager: &'a ComponentManager,
        global_state: &'a Box<dyn GlobalState>,
        scene_state: &'a Box<dyn SceneState>,
    ) -> Self {
        Self {
            component_manager,
            #[cfg(feature = "physics")]
            body_handles: Default::default(),
            organized_components: Default::default(),
            global_state,
            scene_state,
            ser_scene_state: None,
            ser_global_state: None,
        }
    }

    #[cfg(feature = "physics")]
    pub(crate) fn finish(
        self,
    ) -> (
        FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
        FxHashSet<RigidBodyHandle>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
    ) {
        (
            self.organized_components,
            self.body_handles,
            self.ser_scene_state,
            self.ser_global_state,
        )
    }

    #[cfg(not(feature = "physics"))]
    pub(crate) fn finish(
        self,
    ) -> FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>> {
        self.organized_components
    }

    fn add_group<C: ComponentController + serde::Serialize>(
        &mut self,
        target: &mut Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>,
        group_id: &u32,
    ) {
        let type_id = C::IDENTIFIER;
        if let Some(group_index) = self.component_manager.group_index(group_id) {
            let group = self.component_manager.group(*group_index).unwrap();
            if let Some(type_index) = group.type_index(type_id) {
                let type_ref = group.type_ref(*type_index).unwrap();
                target.push((*group_id, type_ref.serialize_components::<C>()));
                #[cfg(feature = "physics")]
                for (_, component) in type_ref {
                    if let Some(body_handle) = component.base().rigid_body_handle() {
                        self.body_handles.insert(body_handle);
                    }
                }
            }
        }
    }

    pub fn serialize_components<C: ComponentController + serde::Serialize>(
        &mut self,
        groups: GroupFilter,
    ) {
        let type_id = C::IDENTIFIER;
        let mut target = vec![];
        match groups {
            GroupFilter::All => {
                for group_id in self.component_manager.group_ids() {
                    self.add_group::<C>(&mut target, group_id)
                }
            }
            GroupFilter::Active => {
                for group_id in self.component_manager.active_group_ids() {
                    self.add_group::<C>(&mut target, group_id)
                }
            }
            GroupFilter::Specific(groups) => {
                for group_id in groups {
                    self.add_group::<C>(&mut target, group_id)
                }
            }
        }
        self.organized_components.insert(type_id, target);
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
        let (mut scene, components, scene_state, global_state): (
            Scene,
            FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
        ) = bincode::deserialize(&self.scene).unwrap();
        let mut de = SceneDeserializer::new(components, scene_state, global_state);
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let window_ratio = window_size.x as f32 / window_size.y as f32;
        scene.world_camera.resize(window_ratio);
        scene.id = self.id;

        let mut ctx = Context::from_fields(shura, &mut scene);
        (self.init)(&mut ctx, &mut de);
        de.finish();
        scene.component_manager.update_sets(&scene.world_camera);
        return scene;
    }
}

#[derive(serde::Deserialize)]
pub struct SceneDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
    scene_state: Option<Vec<u8>>,
    global_state: Option<Vec<u8>>,
}

impl SceneDeserializer {
    pub(crate) fn new(
        components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
        scene_state: Option<Vec<u8>>,
        global_state: Option<Vec<u8>>,
    ) -> Self {
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
        let components = self.components.remove(&type_id).unwrap();
        ctx.component_manager.register_callbacks::<C>();

        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<DynamicComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        #[allow(unused_mut)]
                        let mut component: DynamicComponent =
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
            let type_index = group.type_index(type_id).unwrap();
            let components = Arena::from_items(items, generation);
            group
                .type_mut(*type_index)
                .unwrap()
                .deserialize_components(components);
        }
    }

    pub fn deserialize_components_with<C: ComponentController + FieldNames>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, C>, &'de Context<'de>) -> C,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();
        ctx.component_manager.register_callbacks::<C>();

        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<DynamicComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        let wrapper = DeserializeWrapper::new(&data);
                        #[allow(unused_mut)]
                        let mut component: DynamicComponent = Box::new((de)(wrapper, ctx));
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
            let type_index = group.type_index(type_id).unwrap();
            let components = Arena::from_items(items, generation);
            group
                .type_mut(*type_index)
                .unwrap()
                .deserialize_components(components);
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
