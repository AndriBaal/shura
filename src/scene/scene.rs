use std::any::Any;

use rustc_hash::FxHashMap;

#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Arena, Camera, Color, ComponentController, ComponentManager, Context, CursorManager, Dimension,
    Isometry, Shura, Sprite,
};

pub trait SceneCreator {
    fn name(&self) -> &'static str;
    fn create(&mut self, shura: &mut Shura) -> Scene;
}

pub struct NewScene<N: 'static + FnMut(&mut Context)> {
    pub name: &'static str,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context)> SceneCreator for NewScene<N> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn create(&mut self, shura: &mut Shura) -> Scene {
        let mut scene = Scene::new(shura, self.name);
        let mut ctx = Context::new(shura, &mut scene);
        (self.init)(&mut ctx);
        return scene;
    }
}

// pub struct ExistingScene {
//     pub new_name: &'static str,
//     pub existing: DynamicScene,
// }

// impl SceneCreator for ExistingScene {
//     fn into_scene(mut self, shura: &mut Shura) -> DynamicScene {
//         let window_size: Dimension<u32> = shura.window.inner_size().into();
//         let window_ratio = window_size.width as f32 / window_size.height as f32;
//         let base = self.existing.base_mut();
//         base.name = self.name;
//         base.camera.resize(window_ratio);
//         base
//             .cursor
//             .compute(&base.camera.fov(), &window_size, &shura.input);
//         return self.existing;
//     }
// }

// #[cfg(feature = "serialize")]
// pub struct SerializedScene<
//     S: SceneController,
//     D: 'static + FnMut(&mut Context, ComponentDeserializer) -> S,
// > {
//     pub name: &'static str,
//     pub serializer: SceneSerializer,
//     pub deserialize: D,
// }

// #[cfg(feature = "serialize")]
// impl<S: SceneController, D: 'static + FnMut(&mut Context, ComponentDeserializer) -> S> SceneCreator
//     for SerializedScene<S, D>
// {
//     fn into_scene(self, shura: &mut Shura) -> DynamicScene {
//         todo!()
//     }
// }

#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Scene {
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default = "bool_true"))]
    pub(crate) resized: bool,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default = "bool_true"))]
    pub(crate) switched: bool,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default))]
    pub saved_sprites: Vec<(String, Sprite)>,

    pub(crate) name: &'static str,
    pub camera: Camera,
    pub cursor: CursorManager,
    pub component_manager: ComponentManager,
    pub clear_color: Option<Color>,
    #[cfg(feature = "physics")]
    pub world: World,
    // #[cfg_attr(feature = "serialize", serde(skip))]
    // #[cfg_attr(feature = "serialize", serde(default = "default_user_data"))]
    // pub scene_data: Box<dyn Any>
}

impl Scene {
    pub(crate) fn new(shura: &Shura, name: &'static str) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        Self {
            name,
            switched: true,
            resized: true,
            camera: Camera::new(
                &shura.gpu,
                Isometry::default(),
                window_ratio,
                DEFAULT_VERTICAL_CAMERA_FOV,
            ),
            cursor: CursorManager::new(),
            component_manager: ComponentManager::new(),
            clear_color: Some(Color::new(0.0, 0.0, 0.0, 1.0)),
            #[cfg(feature = "physics")]
            world: World::new(),
            saved_sprites: vec![],
            // scene_data: default_user_data()
        }
    }

    pub fn resized(&self) -> bool {
        self.resized
    }

    pub fn switched(&self) -> bool {
        self.switched
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

// pub(crate) fn default_user_data() -> Box<dyn Any> {
//     return Box::new(());
// }

#[cfg(feature = "serialize")]
#[derive(serde::Serialize)]
pub struct SceneSerializer {
    scene: Scene,
    components: FxHashMap<&'static str, Arena<Box<dyn erased_serde::Serialize>>>,
}

#[cfg(feature = "serialize")]
impl SceneSerializer {
    pub fn new(&self, scene: Scene) -> Self {
        if scene.component_manager.current_component().is_some() {
            panic!("Cannot serialize during component update!")
        }

        Self {
            scene,
            components: Default::default(),
        }
    }

    pub fn serialize_components<
        'de,
        C: ComponentController + serde::Serialize + serde::Deserialize<'de>,
        T: serde::Serializer,
    >(
        &mut self,
        groups: &[u32],
    ) {
        todo!();
        let name = C::name();
        for group_id in groups {
            if let Some(group_index) = self.scene.component_manager.group_index(group_id) {
                let group = self.scene.component_manager.group(*group_index).unwrap();
                if let Some(type_index) = group.type_index(name) {
                    let type_ref = group.type_ref(*type_index).unwrap();
                    for component in type_ref.iter() {}
                }
            }
        }
    }
}

#[cfg(feature = "serialize")]
pub struct ComponentDeserializer {
    components: Vec<String>,
    controller: Option<String>,
}

#[cfg(feature = "serialize")]
impl ComponentDeserializer {
    pub fn deserialize_components<T: ComponentController + for<'de> serde::Deserialize<'de>>(
        &mut self,
    ) {
    }

    pub fn deserialize_components_with_ctx<
        'a,
        'de,
        T: ComponentController,
        D: From<(String, &'a Context<'a>)> + serde::de::DeserializeSeed<'de, Value = T>,
    >(
        &mut self,
        deserializer: D,
        ctx: &'a mut Context,
    ) {
        let deserialized_components = self.components.pop().unwrap();
        // let deserializer = D::from((deserialized_components, &ctx));
    }

    pub fn deserialize_controller(&mut self) {
        todo!();
    }
}

// impl<'de> serde::de::DeserializeSeed<'de> for Shura {
//     type Value = Scene;

//     fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         impl<'de> serde::de::Visitor<'de> for Shura {
//             type Value = Camera;

//             fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 formatter.write_str("Test AB")
//             }

//             fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//             where
//                 A: serde::de::MapAccess<'de>,
//             {
//                 let mut position = None;
//                 let mut target = None;
//                 let mut vertical_fov = None;
//                 let mut ratio = None;

//                 while let Some(key) = map.next_key::<&str>()? {
//                     match key {
//                         "position" => position = Some(map.next_value()?),
//                         "target" => target = Some(map.next_value()?),
//                         "vertical_fov" => vertical_fov = Some(map.next_value()?),
//                         "ratio" => ratio = Some(map.next_value()?),
//                         _ => {}
//                     }
//                 }

//                 let cam = Camera::new(
//                     &self.gpu,
//                     position.unwrap(),
//                     ratio.unwrap(),
//                     vertical_fov.unwrap(),
//                 );
//                 cam.target = target;
//                 return Ok(cam);
//             }
//         }
//         return deserializer.deserialize_struct("", &[], self);
//     }
// }
