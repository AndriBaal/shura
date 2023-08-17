use bincode::{
    config::{AllowTrailing, FixintEncoding, WithOtherIntEncoding, WithOtherTrailing},
    de::read::SliceReader,
    DefaultOptions, Options,
};
use rustc_hash::FxHashMap;
use serde::{de::Visitor, Deserializer};
use std::marker::PhantomData;

use crate::{
    Arena, ArenaEntry, BoxedComponent, ComponentBuffer, ComponentController, ComponentManager,
    ComponentTypeId, ComponentTypeStorage, Context, ContextUse, FieldNames, GroupHandle, Scene,
    SceneCreator, Shura,
};

#[cfg(feature = "serde")]
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) enum SerializedComponentStorage {
    Single(Option<Vec<u8>>),
    Multiple(Vec<Option<(u32, Vec<u8>)>>),
    MultipleGroups(Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>),
}

/// Helper to serialize [Components](crate::Component) and [States](crate::State) of a [Scene]
pub struct SceneSerializer<'a> {
    components: &'a ComponentManager,
    ser_components: FxHashMap<ComponentTypeId, SerializedComponentStorage>,
}

impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(components: &'a ComponentManager) -> Self {
        Self {
            components,
            ser_components: Default::default(),
        }
    }

    pub(crate) fn finish(self) -> FxHashMap<ComponentTypeId, SerializedComponentStorage> {
        self.ser_components
    }

    pub fn serialize_components<C: Component + serde::Serialize>(&mut self) {
        let ty = self.components.type_ref::<C>();
        let ser = match &ty.storage {
            ComponentTypeStorage::Single { component, .. } => {
                let data = if let Some(component) = &component {
                    bincode::serialize(component.downcast_ref::<C>().unwrap()).ok()
                } else {
                    None
                };
                SerializedComponentStorage::Single(data)
            }
            ComponentTypeStorage::Multiple(multiple) => {
                let ser_components = multiple.components.serialize_components::<C>();
                SerializedComponentStorage::Multiple(ser_components)
            }
            ComponentTypeStorage::MultipleGroups(groups) => {
                let mut group_data = vec![];
                for (group_handle, group) in groups.iter_with_index() {
                    let ser_components = group.components.serialize_components::<C>();
                    group_data.push((GroupHandle(group_handle), ser_components));
                }
                SerializedComponentStorage::MultipleGroups(group_data)
            }
        };
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
        let (mut scene, ser_components): (
            Scene,
            FxHashMap<ComponentTypeId, SerializedComponentStorage>,
        ) = bincode::deserialize(&self.scene).unwrap();
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
    ser_components: FxHashMap<ComponentTypeId, SerializedComponentStorage>,
}

impl SceneDeserializer {
    pub(crate) fn new(
        ser_components: FxHashMap<ComponentTypeId, SerializedComponentStorage>,
    ) -> Self {
        Self { ser_components }
    }

    pub fn deserialize_components<
        C: serde::de::DeserializeOwned + ComponentController + ComponentBuffer,
    >(
        &mut self,
        ctx: &mut Context,
    ) {
        let type_id = C::IDENTIFIER;
        let mut ty = ctx.components.type_mut::<C>();
        if let Some(storage) = self.ser_components.remove(&type_id) {
            match storage {
                SerializedComponentStorage::Single(single) => match &mut ty.storage {
                    ComponentTypeStorage::Single { component, .. } => {
                        if let Some(single) = single {
                            *component =
                                Some(Box::new(bincode::deserialize::<C>(&single).unwrap()));
                        }
                    }
                    _ => unreachable!(),
                },
                SerializedComponentStorage::Multiple(ser_multiple) => match &mut ty.storage {
                    ComponentTypeStorage::Multiple(multiple) => {
                        let mut items: Vec<ArenaEntry<BoxedComponent>> =
                            Vec::with_capacity(ser_multiple.capacity());
                        let mut generation = 0;
                        for component in ser_multiple {
                            let item = match component {
                                Some((gen, data)) => {
                                    generation = std::cmp::max(generation, gen);
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
                        multiple.components = components;
                    }
                    _ => unreachable!(),
                },
                SerializedComponentStorage::MultipleGroups(ser_groups) => match &mut ty.storage {
                    ComponentTypeStorage::MultipleGroups(groups) => {
                        for (group_id, components) in ser_groups {
                            let mut items: Vec<ArenaEntry<BoxedComponent>> =
                                Vec::with_capacity(components.capacity());
                            let mut generation = 0;
                            for component in components {
                                let item = match component {
                                    Some((gen, data)) => {
                                        generation = std::cmp::max(generation, gen);
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
                            groups[group_id.0].components = components;
                        }
                    }
                    _ => unreachable!(),
                },
            }
        }
    }

    pub fn deserialize_components_with<C: Component + FieldNames + ComponentBuffer>(
        &mut self,
        ctx: &mut Context,
        mut de: impl for<'de> FnMut(DeserializeWrapper<'de, C>, &'de Context<'de>) -> C,
    ) {
        let type_id = C::IDENTIFIER;
        if let Some(storage) = self.ser_components.remove(&type_id) {
            match storage {
                SerializedComponentStorage::Single(single) => {
                    if let Some(single) = single {
                        let wrapper = DeserializeWrapper::new(&single);
                        let c: BoxedComponent = Box::new((de)(wrapper, ctx));
                        let mut ty = ctx.components.type_mut::<C>();
                        match &mut ty.storage {
                            ComponentTypeStorage::Single { component, .. } => {
                                *component = Some(c);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                SerializedComponentStorage::Multiple(ser_multiple) => {
                    let mut items: Vec<ArenaEntry<BoxedComponent>> =
                        Vec::with_capacity(ser_multiple.capacity());
                    let mut generation = 0;
                    for component in ser_multiple {
                        let item = match component {
                            Some((gen, data)) => {
                                generation = std::cmp::max(generation, gen);
                                let wrapper = DeserializeWrapper::new(&data);
                                let component: BoxedComponent = Box::new((de)(wrapper, ctx));
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
                    let mut ty = ctx.components.type_mut::<C>();
                    match &mut ty.storage {
                        ComponentTypeStorage::Multiple(multiple) => {
                            multiple.components = components;
                        }
                        _ => unreachable!(),
                    }
                }
                SerializedComponentStorage::MultipleGroups(ser_groups) => {
                    for (group_id, components) in ser_groups {
                        let mut items: Vec<ArenaEntry<BoxedComponent>> =
                            Vec::with_capacity(components.capacity());
                        let mut generation = 0;
                        for component in components {
                            let item = match component {
                                Some((gen, data)) => {
                                    generation = std::cmp::max(generation, gen);
                                    let wrapper = DeserializeWrapper::new(&data);
                                    let component: BoxedComponent = Box::new((de)(wrapper, ctx));
                                    ArenaEntry::Occupied {
                                        generation: gen,
                                        data: component,
                                    }
                                }
                                None => ArenaEntry::Free { next_free: None },
                            };
                            items.push(item);
                        }

                        let mut ty = ctx.components.type_mut::<C>();
                        match &mut ty.storage {
                            ComponentTypeStorage::MultipleGroups(groups) => {
                                let components = Arena::from_items(items, generation);
                                groups[group_id.0].components = components;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
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
