#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Camera, Color, ComponentController, ComponentManager, Context, CursorManager, Dimension,
    DynamicComponent, Isometry, Shura, Sprite,
};

pub enum SceneSource {
    New {
        name: &'static str,
    },
    Existing {
        name: &'static str,
        existing: Scene,
    },
    #[cfg(feature = "serialize")]
    Serialized {
        name: &'static str,
        serialized: SerializedScene,
    },
}

impl SceneSource {
    pub fn name(&self) -> &'static str {
        return match &self {
            SceneSource::New { name } => name,
            SceneSource::Existing { name, existing } => name,
            SceneSource::Serialized { name, serialized } => name,
        };
    }

    pub fn new(name: &'static str) -> Self {
        Self::New { name }
    }

    pub fn existing(name: &'static str, existing: Scene) -> Self {
        Self::Existing { name, existing }
    }

    // #[cfg(feature = "serialize")]
    // pub fn serialized(name: &'static str, deserializer: SerializedScene) -> Self {
    //     Self::Serialized { name, serialized }
    // }
}

fn default_true() -> bool {
    return true;
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Scene {
    #[serde(skip)]
    #[serde(default = "default_true")]
    pub(crate) resized: bool,
    #[serde(skip)]
    #[serde(default)]
    pub(crate) switched: bool,
    #[serde(skip)]
    #[serde(default)]
    pub saved_sprites: Vec<(String, Sprite)>,

    pub(crate) name: &'static str,
    pub camera: Camera,
    pub cursor: CursorManager,
    pub component_manager: ComponentManager,
    pub clear_color: Option<Color>,
    #[cfg(feature = "physics")]
    pub world: World,
}

impl Scene {
    pub(crate) fn new(shura: &Shura, source: SceneSource) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;

        return match source {
            SceneSource::New { name } => Self {
                name,
                switched: false,
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
            },
            SceneSource::Existing { name, mut existing } => {
                existing.name = name;
                existing.camera.resize(window_ratio);
                existing
                    .cursor
                    .compute(&existing.camera.fov(), &window_size, &shura.input);
                existing
            }
            #[cfg(feature = "serialize")]
            SceneSource::Serialized { name, serialized } => {
                todo!()
            }
        };
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

#[cfg(feature = "serialize")]
pub struct SceneSerializer<'a, T: serde::Serializer> {
    scene: &'a Scene,
    ser_scene: String,
    ser_components: Vec<String>,
    serializer: T,
}

#[cfg(feature = "serialize")]
impl<'a, T: serde::Serializer> SceneSerializer<'a, T> {
    pub fn new(&self, scene: &'a Scene, serializer: T) -> Self {
        if scene.component_manager.current_component().is_some() {
            panic!("Cannot serialize during component update!")
        }

        let ser_scene = serde_json::to_string(scene).unwrap();

        Self {
            scene,
            ser_scene,
            ser_components: vec![],
            serializer,
        }
    }

    pub fn serialize_components<
        'de,
        C: ComponentController + serde::Serialize + serde::Deserialize<'de>,
    >(
        &mut self,
        groups: &[u32],
    ) {
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

    pub fn finish(self) -> SerializedScene {
        return SerializedScene {
            scene: self.ser_scene,
            components: self.ser_components,
        };
    }
}

#[cfg(feature = "serialize")]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SerializedScene {
    scene: String,
    components: Vec<String>,
}

impl SerializedScene {
    pub fn deserialize() {}
}

#[cfg(feature = "serialize")]
pub struct SceneDeserializer<'a> {
    shura: &'a Shura,
    scene: Scene,
    components: Vec<String>,
}

impl<'a> SceneDeserializer<'a> {
    // pub fn new(shura: &'a Shura, serialized: SerializedScene) -> Self {
    //     let de = serde_json::Deserializer::from_str(&serialized.scene);
    //     let scene = shura
    //         .deserialize(&mut de)
    //         .unwrap();
    //     Self {
    //         shura,
    //         scene,
    //         components: serialized.components
    //     }
    // }

    pub fn deserialize_components<T: ComponentController + for<'de> serde::Deserialize<'de>>(
        &mut self,
    ) {
    }

    pub fn deserialize_components_with_ctx<
        'de,
        T: ComponentController,
        A: serde::de::MapAccess<'de>,
    >(
        &mut self,
        deserialize: impl FnMut(A, &mut Context) -> DynamicComponent,
    ) {
        impl<'de, 'a> serde::de::DeserializeSeed<'de> for Context<'a> {
            type Value = DynamicComponent;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                impl<'de, 'a> serde::de::Visitor<'de> for Context<'a> {
                    type Value = DynamicComponent;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("Test AB")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        return Ok(deserialize(map, self));
                    }
                }
                return deserializer.deserialize_struct("", &[], self);
            }
        }
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
