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
use serde::{de::Visitor, Deserializer};
use std::{cmp, marker::PhantomData};

use crate::{
    Arena, ArenaEntry, ComponentController, ComponentIdentifier, ComponentManager, ComponentTypeId,
    Context, DynamicComponent, GroupFilter, Scene, SceneCreator, Shura, Vector,
};

pub struct ComponentSerializer<'a> {
    component_manager: &'a ComponentManager,
    organized_components:
        FxHashMap<ComponentTypeId, Vec<(u32 /* Group id */, Vec<Option<(u32, Vec<u8>)>>)>>,
    #[cfg(feature = "physics")]
    body_handles: FxHashSet<RigidBodyHandle>,
}

impl<'a> ComponentSerializer<'a> {
    pub(crate) fn new(component_manager: &'a ComponentManager) -> Self {
        Self {
            component_manager,
            #[cfg(feature = "physics")]
            body_handles: Default::default(),
            organized_components: Default::default(),
        }
    }

    pub(crate) fn finish(
        self,
    ) -> (
        FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
        FxHashSet<RigidBodyHandle>,
    ) {
        (self.organized_components, self.body_handles)
    }

    fn add_group<C: ComponentController + ComponentIdentifier + serde::Serialize>(
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

    pub fn serialize_components<C: ComponentController + ComponentIdentifier + serde::Serialize>(
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
}

pub struct SerializedScene<N: 'static + FnMut(&mut Context, &mut ComponentDeserializer)> {
    pub id: u32,
    pub scene: Vec<u8>,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context, &mut ComponentDeserializer)> SerializedScene<N> {
    pub fn new(id: u32, scene: Vec<u8>, init: N) -> SerializedScene<N> {
        Self { id, scene, init }
    }
}

impl<N: 'static + FnMut(&mut Context, &mut ComponentDeserializer)> SceneCreator
    for SerializedScene<N>
{
    fn id(&self) -> u32 {
        self.id
    }

    fn create(mut self, shura: &mut Shura) -> Scene {
        let (mut scene, components): (
            Scene,
            FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
        ) = bincode::deserialize(&self.scene).unwrap();
        let mut de = ComponentDeserializer::new(components);
        scene.before_deserialize(self.id, shura);

        let mut ctx = Context::new(shura, &mut scene);
        (self.init)(&mut ctx, &mut de);
        de.finish();
        return scene;
    }
}

#[derive(serde::Deserialize)]
pub struct ComponentDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
}

impl ComponentDeserializer {
    pub(crate) fn new(
        components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
    ) -> Self {
        Self { components }
    }

    pub(crate) fn finish(&self) {
        assert!(
            self.components.is_empty(),
            "All components need to be deserialized!"
        );
    }

    pub fn deserialize_components<
        C: serde::de::DeserializeOwned + ComponentController + ComponentIdentifier,
    >(
        &mut self,
        ctx: &mut Context,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();
        ctx.scene.component_manager.register_callbacks::<C>();

        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<DynamicComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        let mut component: DynamicComponent =
                            Box::new(bincode::deserialize::<C>(&data).unwrap());
                        #[cfg(feature = "physics")]
                        if component.base().is_rigid_body() {
                            component
                                .base_mut()
                                .init_rigid_body(ctx.scene.component_manager.world.clone());
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

    pub fn deserialize_components_with<C: ComponentController + ComponentIdentifier>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, C>, &'de Context<'de>) -> C,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();
        ctx.scene.component_manager.register_callbacks::<C>();

        for (group_id, components) in components {
            let mut items: Vec<ArenaEntry<DynamicComponent>> =
                Vec::with_capacity(components.capacity());
            let mut generation = 0;
            for component in components {
                let item = match component {
                    Some((gen, data)) => {
                        generation = cmp::max(generation, gen);
                        let wrapper = DeserializeWrapper::new(&data);
                        let mut component: DynamicComponent = Box::new((de)(wrapper, ctx));
                        #[cfg(feature = "physics")]
                        if component.base().is_rigid_body() {
                            component
                                .base_mut()
                                .init_rigid_body(ctx.scene.component_manager.world.clone());
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
}

pub struct DeserializeWrapper<'de, C: ComponentController> {
    de: bincode::Deserializer<
        SliceReader<'de>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    >,
    _marker: PhantomData<C>,
}

impl<'de, C: ComponentController> DeserializeWrapper<'de, C> {
    pub(crate) fn new(data: &'de [u8]) -> Self {
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let de = bincode::Deserializer::from_slice(&data, options);
        Self {
            de,
            _marker: PhantomData::<C>,
        }
    }

    pub fn deserialize(&mut self, visitor: impl Visitor<'de, Value = C>) -> C {
        self.de.deserialize_struct("", &[""], visitor).unwrap()
    }
}

impl Scene {
    pub(crate) fn before_deserialize(&mut self, id: u32, shura: &Shura) {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let window_ratio = window_size.x as f32 / window_size.y as f32;
        self.world_camera.resize(window_ratio);
        self.id = id;
    }
}
