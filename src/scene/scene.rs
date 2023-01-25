use serde::de::DeserializeSeed;

#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Arena, Camera, Color, ComponentController, ComponentManager, Context, CursorManager, Dimension,
    DynamicComponent, DynamicScene, Isometry, SceneController, Shura, Sprite,
};

pub trait SceneCreator {
    fn name(&self) -> &'static str;
    fn into_scene(self, shura: &mut Shura) -> (DynamicScene, Scene);
}

pub struct NewScene<S: SceneController, N: 'static + FnMut(&mut Context) -> S> {
    name: &'static str,
    init: N,
}

impl<S: SceneController, N: 'static + FnMut(&mut Context) -> S> SceneCreator for NewScene<S, N> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn into_scene(mut self, shura: &mut Shura) -> (DynamicScene, Scene) {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        let mut scene = Scene {
            name: self.name,
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
        };

        let mut ctx = Context::new(&mut scene, shura);
        let controller = Box::new((self.init)(&mut ctx));
        drop(ctx);
        return (controller, scene);
    }
}

pub struct ExistingScene {
    name: &'static str,
    existing: (DynamicScene, Scene),
}

impl SceneCreator for ExistingScene {
    fn name(&self) -> &'static str {
        self.name
    }

    fn into_scene(mut self, shura: &mut Shura) -> (DynamicScene, Scene) {
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        self.existing.1.name = self.name;
        self.existing.1.camera.resize(window_ratio);
        self.existing
            .1
            .cursor
            .compute(&self.existing.1.camera.fov(), &window_size, &shura.input);
        return self.existing;
    }
}

#[cfg(feature = "serialize")]
pub struct SerializedScene<
    S: SceneController,
    D: 'static + FnMut(&mut Context, ComponentDeserializer) -> S,
> {
    name: &'static str,
    serializer: SceneSerializer,
    deserialize: D,
}

impl<S: SceneController, D: 'static + FnMut(&mut Context, ComponentDeserializer) -> S> SceneCreator
    for SerializedScene<S, D>
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn into_scene(self, shura: &mut Shura) -> (DynamicScene, Scene) {
        todo!()
    }
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
    pub(crate) fn new(shura: &mut Shura, source: impl SceneCreator) -> (DynamicScene, Self) {
        return source.into_scene(shura);

        // return match source {
        //     SceneSource::New { name } => Self {
        //         name,
        //         switched: false,
        //         resized: true,
        //         camera: Camera::new(
        //             &shura.gpu,
        //             Isometry::default(),
        //             window_ratio,
        //             DEFAULT_VERTICAL_CAMERA_FOV,
        //         ),
        //         cursor: CursorManager::new(),
        //         component_manager: ComponentManager::new(),
        //         clear_color: Some(Color::new(0.0, 0.0, 0.0, 1.0)),
        //         #[cfg(feature = "physics")]
        //         world: World::new(),
        //         saved_sprites: vec![],
        //     },
        //     SceneSource::Existing { name, mut existing } => {
        //         existing.name = name;
        //         existing.camera.resize(window_ratio);
        //         existing
        //             .cursor
        //             .compute(&existing.camera.fov(), &window_size, &shura.input);
        //         existing
        //     }
        //     #[cfg(feature = "serialize")]
        //     SceneSource::Serialized { name, serialized } => {
        //         todo!()
        //     }
        // };
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
#[derive(serde::Serialize)]
pub struct SceneSerializer {
    scene: Scene,
    components: Vec<String>,
    controller: Option<String,>
}

#[cfg(feature = "serialize")]
impl SceneSerializer {
    pub fn new(&self, scene: Scene) -> Self {
        if scene.component_manager.current_component().is_some() {
            panic!("Cannot serialize during component update!")
        }

        Self {
            scene,
            components: vec![],
            controller: None,
        }
    }

    pub fn with_scene_controller<S: SceneController + serde::Serialize>(&mut self) {
        todo!();
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

impl ComponentDeserializer {
    pub fn deserialize_components<T: ComponentController + for<'de> serde::Deserialize<'de>>(
        &mut self,
    ) {
    }

    pub fn deserialize_components_with_ctx<
        'a,
        'de,
        T: ComponentController,
        D: From<(String, &'a Context<'a>)> + DeserializeSeed<'de, Value = T>,
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
