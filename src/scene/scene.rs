#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{Camera, Color, ComponentManager, CursorManager, Isometry, Sprite, Shura};


pub struct SceneDescriptor {
    pub(crate) name: &'static str,
    pub(crate) existing: Option<Scene>,
    // serialized: Option<u8>
}

impl SceneDescriptor {
    pub fn new(name: &'static str) -> Self {
        Self { name, existing: None }
    }

    pub fn existing(name: &'static str, scene: Scene) -> Self {
        Self {
            name,
            existing: Some(scene)
        }
    }
}

pub struct Scene {
    pub(crate) resized: bool,
    pub(crate) switched: bool,
    pub(crate) name: &'static str,
    pub saved_sprites: Vec<(String, Sprite)>,
    pub camera: Camera,
    pub cursor: CursorManager,
    pub component_manager: ComponentManager,
    pub clear_color: Option<Color>,
    #[cfg(feature = "physics")]
    pub world: World
}

impl Scene {
    pub(crate) fn new(shura: &Shura, descriptor: SceneDescriptor) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        let window_size = shura.window.inner_size();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        if let Some(existing) = descriptor.existing {
            existing.name = descriptor.name;
            return existing;
        } else {
            return Self {
                name: descriptor.name,
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
