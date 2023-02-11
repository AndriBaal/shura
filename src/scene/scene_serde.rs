use rustc_hash::FxHashMap;

use crate::{ComponentManager, ComponentTypeId, SerializeableComponent, ComponentController, ComponentIdentifier, GroupFilter, Context, Arena, Shura, Scene, SceneCreator};


pub struct SerializedScene<N: 'static + FnMut(&mut Context, &mut ComponentDeserializer)> {
    pub id: u32,
    pub scene: String,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context, &mut ComponentDeserializer)> SerializedScene<N> {
    pub fn new(id: u32, scene: String, init: N) -> SerializedScene<N> {
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
        #[derive(serde::Deserialize)]
        struct DeserializeHelper {
            scene: Scene,
            components: FxHashMap<u32, Vec<(u32, ron::Value)>>,
        }
        impl From<DeserializeHelper> for (Scene, FxHashMap<u32, Vec<(u32, ron::Value)>>) {
            fn from(e: DeserializeHelper) -> (Scene, FxHashMap<u32, Vec<(u32, ron::Value)>>) {
                (e.scene, e.components)
            }
        }

        let (mut scene, mut components): (Scene, FxHashMap<u32, Vec<(u32, ron::Value)>>) =
            ron::from_str::<DeserializeHelper>(&self.scene)
                .unwrap()
                .into();
        scene.before_deserialize(self.id, shura);

        // let mut scene = Scene::new(window_ratio, self.id);
        // let mut ctx = Context::new(shura, &mut scene);
        // (self.init)(&mut ctx);
        return scene;
    }
}

pub struct ComponentSerializer<'a> {
    component_manager: &'a ComponentManager,
    pub(crate) components: FxHashMap<
        ComponentTypeId,
        Vec<(u32, Vec<Option<(&'a u32, &'a dyn SerializeableComponent)>>)>,
    >,
}

impl<'a> ComponentSerializer<'a> {
    pub(crate) fn new(component_manager: &'a ComponentManager) -> Self {
        Self {
            component_manager,
            components: Default::default(),
        }
    }

    pub(crate) fn finish(
        self,
    ) -> FxHashMap<
        ComponentTypeId,
        Vec<(u32, Vec<Option<(&'a u32, &'a dyn SerializeableComponent)>>)>,
    > {
        self.components
    }

    fn add_group<C: ComponentController + ComponentIdentifier + serde::Serialize>(
        &self,
        target: &mut Vec<(u32, Vec<Option<(&'a u32, &'a dyn SerializeableComponent)>>)>,
        group_id: &u32,
    ) {
        let type_id = C::IDENTIFIER;
        if let Some(group_index) = self.component_manager.group_index(group_id) {
            let group = self.component_manager.group(*group_index).unwrap();
            if let Some(type_index) = group.type_index(type_id) {
                let type_ref = group.type_ref(*type_index).unwrap();
                target.push((*group_id, type_ref.serialize_components::<C>()))
            }
        }
    }

    pub fn serialize_components<C: ComponentController + ComponentIdentifier + serde::Serialize>(
        &mut self,
        groups: GroupFilter,
    ) {
        let type_id = C::IDENTIFIER;
        let mut target = vec![];
        if type_id == self.component_manager.current_type() {
            panic!("Cannot serialize currently used component!");
        }
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
        self.components.insert(type_id, target);
    }
}

#[derive(serde::Deserialize)]
pub struct ComponentDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u32, ron::Value)>>,
}

impl ComponentDeserializer {
    pub fn deserialize<
        'de,
        C: serde::de::DeserializeOwned + ComponentController + ComponentIdentifier,
    >(
        &'de mut self,
        ctx: &'de mut Context<'de>,
    ) {
        let type_id = C::IDENTIFIER;
        let components = self.components.remove(&type_id).unwrap();

        for (group_id, components) in components {
            let components = components.into_rust::<Arena<ron::Value>>().unwrap();
            let components = components.cast::<C>();
            let group = ctx.group_mut(group_id).unwrap();
            let type_index = group.type_index(type_id).unwrap();
            group
                .type_mut(*type_index)
                .unwrap()
                .deserialize_components(components);
        }
    }

    pub fn deserialize_with_visitor<
        'de,
        C: ComponentController + ComponentIdentifier,
        V: serde::de::Visitor<'de, Value = Vec<Option<C>>> + From<&'de mut Context<'de>>,
    >(
        &mut self,
        ctx: &mut Context,
        visitor: V,
    ) {
    }
}
