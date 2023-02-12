use bincode::Options;
use rapier2d::prelude::RigidBodyHandle;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    physics::PhysicsComponent, Arena, ComponentController, ComponentIdentifier, ComponentManager,
    ComponentTypeId, Context, GroupFilter, Scene, SceneCreator, Shura,
};

pub struct ComponentSerializer<'a> {
    component_manager: &'a ComponentManager,
    pub(crate) organized_components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<u8>)>>,
    pub(crate) body_handles: FxHashSet<RigidBodyHandle>,
}

impl<'a> ComponentSerializer<'a> {
    pub(crate) fn new(component_manager: &'a ComponentManager) -> Self {
        Self {
            component_manager,
            body_handles: Default::default(),
            organized_components: Default::default(),
        }
    }

    fn add_group<C: ComponentController + ComponentIdentifier + serde::Serialize>(
        &mut self,
        target: &mut Vec<(u32, Vec<u8>)>,
        group_id: &u32,
    ) {
        let type_id = C::IDENTIFIER;
        if let Some(group_index) = self.component_manager.group_index(group_id) {
            let group = self.component_manager.group(*group_index).unwrap();
            if let Some(type_index) = group.type_index(type_id) {
                let type_ref = group.type_ref(*type_index).unwrap();
                target.push((
                    *group_id,
                    bincode::serialize(&type_ref.serialize_components::<C>()).unwrap(),
                ));
                for component in type_ref {
                    if let Some(base) = component.1.base().downcast_ref::<PhysicsComponent>() {
                        if let Some(body_handle) = base.body_handle() {
                            self.body_handles.insert(body_handle);
                        }
                    } else {
                        break;
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

    fn create(&mut self, shura: &mut Shura) -> Scene {
        let (mut scene, components): (Scene, FxHashMap<ComponentTypeId, Vec<(u32, Vec<u8>)>>) =
            bincode::deserialize(&self.scene).unwrap();
        let mut de = ComponentDeserializer::new(components);
        scene.before_deserialize(self.id, shura);

        let mut ctx = Context {
            shura,
            scene: &mut scene,
        };
        (self.init)(&mut ctx, &mut de);
        return scene;
    }
}

#[derive(serde::Deserialize)]
pub struct ComponentDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<u8>)>>,
}

impl ComponentDeserializer {
    pub(crate) fn new(components: FxHashMap<ComponentTypeId, Vec<(u32, Vec<u8>)>>) -> Self {
        Self { components }
    }

    pub fn deserialize_components<
        'de,
        C: serde::de::DeserializeOwned + ComponentController + ComponentIdentifier,
    >(
        &'de mut self,
        ctx: &mut Context,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();

        for (group_id, components) in components {
            let components: Arena<C> = bincode::deserialize(&components).unwrap();
            let components = components.cast();
            let group = ctx.group_mut(group_id).unwrap();
            let type_index = group.type_index(type_id).unwrap();
            group
                .type_mut(*type_index)
                .unwrap()
                .deserialize_components(components);
        }
    }

    pub fn deserialize_components_with_ctx<
        'de,
        C: ComponentController + ComponentIdentifier,
        V: serde::de::DeserializeSeed<'de, Value = Vec<Option<C>>> + From<&'de mut Context<'de>>,
    >(
        &mut self,
        ctx: &mut Context,
        visitor: V,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();
        let seed = V::from(ctx);

        for (group_id, components) in components {
            let components: Vec<Option<C>> = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes()
                .deserialize_seed(seed, &components)
                .unwrap();
            let group = ctx.group_mut(group_id).unwrap();
            let type_index = group.type_index(type_id).unwrap();
            group
                .type_mut(*type_index)
                .unwrap()
                .deserialize_components(components);
        }
    }
}
