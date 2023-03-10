use rustc_hash::FxHashMap;

use crate::{
    Camera, ComponentManager, Context, CursorManager, Isometry, ScreenConfig, Shura, Sprite, Vector,
};

pub trait SceneCreator {
    fn id(&self) -> u32;
    fn create(&mut self, shura: &mut Shura) -> Scene;
}

pub struct NewScene<N: 'static + FnMut(&mut Context)> {
    pub id: u32,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context)> NewScene<N> {
    pub fn new(id: u32, init: N) -> NewScene<N> {
        Self { id, init }
    }
}

impl<N: 'static + FnMut(&mut Context)> SceneCreator for NewScene<N> {
    fn id(&self) -> u32 {
        self.id
    }

    fn create(&mut self, shura: &mut Shura) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let window_ratio = window_size.x as f32 / window_size.y as f32;
        let mut scene = Scene::new(window_ratio, self.id);
        let mut ctx = Context {
            shura,
            scene: &mut scene,
        };
        (self.init)(&mut ctx);
        return scene;
    }
}

#[cfg(feature = "serde")]
fn bool_true() -> bool {
    return true;
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Scene {
    pub(crate) id: u32,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "bool_true"))]
    pub(crate) resized: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "bool_true"))]
    pub(crate) switched: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub saved_sprites: FxHashMap<String, Sprite>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub cursor: CursorManager,
    pub render_config: ScreenConfig,
    pub camera: Camera,
    pub component_manager: ComponentManager,
}

impl Scene {
    pub(crate) fn new(ratio: f32, id: u32) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        let fov = Vector::new(
            DEFAULT_VERTICAL_CAMERA_FOV * ratio,
            DEFAULT_VERTICAL_CAMERA_FOV,
        );
        Self {
            id: id,
            switched: true,
            resized: true,
            camera: Camera::new(Isometry::default(), fov),
            cursor: CursorManager::new(),
            component_manager: ComponentManager::new(),
            render_config: ScreenConfig::new(),
            saved_sprites: Default::default(),
        }
    }

    pub fn resized(&self) -> bool {
        self.resized
    }

    pub fn switched(&self) -> bool {
        self.switched
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
