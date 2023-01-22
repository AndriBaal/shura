#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Camera, Color, ComponentController, ComponentManager, CursorManager, Dimension, Isometry,
    Shura, Sprite,
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
        }
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Scene {
    #[serde(skip)]
    #[serde(default = "True")]
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
            SceneSource::Serialized { name, serialized } => { todo!() }
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

    pub fn serialize_components<'de, C: ComponentController + serde::Serialize + serde::Deserialize<'de>>(&mut self, groups: &[u32]) {
        let name = C::name();
        for group_id in groups {
            if let Some(group_index) = self.scene.component_manager.group_index(group_id) {
                let group = self.scene.component_manager.group(*group_index).unwrap();
                if let Some(type_index) = group.type_index(name) {
                    let type_ref = group.type_ref(*type_index).unwrap();
                    for component in type_ref.iter() {

                    }
                }
            }
        }
    }
}

#[cfg(feature = "serialize")]
pub struct SerializedScene {
    scene: String,
    components: Vec<String>,
}
