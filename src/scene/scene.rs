#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{Camera, Color, ComponentManager, CursorManager, Gpu, Isometry, Sprite};

pub(crate) struct Scene {
    pub(super) resized: bool,
    pub(super) switched: bool,
    pub(super) name: &'static str,
    pub(super) saved_sprites: Vec<(String, Sprite)>,
    pub(super) camera: Camera,
    pub(super) cursor: CursorManager,
    pub(super) component_manager: ComponentManager,
    pub(super) clear_color: Option<Color>,
    #[cfg(feature = "physics")]
    pub(super) world: World,
}

impl Scene {
    pub fn new(gpu: &Gpu, window_ratio: f32, name: &'static str) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        Self {
            name,
            switched: false,
            resized: true,
            camera: Camera::new(
                gpu,
                Isometry::default(),
                window_ratio,
                DEFAULT_VERTICAL_CAMERA_FOV,
            ),
            cursor: CursorManager::new(),
            component_manager: ComponentManager::new(),
            clear_color: Some(Color::new(0.0, 0.0, 0.0, 1.0)),
            #[cfg(feature = "physics")]
            world: World::new(),
            saved_sprites: vec![]
        }
    }
}
