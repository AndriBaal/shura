use instant::Duration;
use ron::ser::PrettyConfig;
use rustc_hash::FxHashMap;

#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    data::arena::Arena, Camera, Color, ComponentController, ComponentIdentifier, ComponentManager,
    ComponentTypeId, Context, CursorManager, Dimension, Isometry, Shura, Sprite,
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
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        let mut scene = Scene::new(window_ratio, self.id);
        let mut ctx = Context {
            shura,
            scene: &mut scene,
        };
        (self.init)(&mut ctx);
        return scene;
    }
}

#[cfg(feature = "serialize")]
pub struct SerializedScene<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> {
    pub id: u32,
    pub scene: String,
    pub init: N,
}

#[cfg(feature = "serialize")]
impl<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> SerializedScene<N> {
    pub fn new(id: u32, scene: String, init: N) -> SerializedScene<N> {
        Self { id, scene, init }
    }
}

#[cfg(feature = "serialize")]
impl<N: 'static + FnMut(&mut Context, &mut SceneDeserializer)> SceneCreator for SerializedScene<N> {
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

fn bool_true() -> bool {
    return true;
}

#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug)]
pub struct SceneRenderConfig {
    clear_color: Option<Color>,
    render_scale: f32,
    max_fps: Option<u32>,
}

impl Default for SceneRenderConfig {
    fn default() -> Self {
        Self {
            clear_color: Some(Color::new(0.0, 0.0, 0.0, 1.0)),
            render_scale: 1.0,
            max_fps: None,
        }
    }
}

impl SceneRenderConfig {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    pub fn max_fps(&self) -> Option<u32> {
        self.max_fps
    }

    pub fn set_render_scale(&self, shura: &mut Shura, render_scale: f32) {
        shura.defaults.apply_render_scale(&shura.gpu, render_scale);
    }

    pub fn set_clear_color(&mut self, clear_color: Option<Color>) {
        self.clear_color = clear_color;
    }

    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        self.max_fps = max_fps;
    }

    pub fn max_frame_time(&self) -> Option<Duration> {
        if let Some(max_fps) = self.max_fps {
            return Some(Duration::from_secs_f32(1.0 / max_fps as f32));
        }
        return None;
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
pub struct Scene {
    pub(crate) id: u32,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default = "bool_true"))]
    pub(crate) resized: bool,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default = "bool_true"))]
    pub(crate) switched: bool,
    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg_attr(feature = "serialize", serde(default))]
    pub saved_sprites: Vec<(String, Sprite)>,
    pub cursor: CursorManager,
    pub render_config: SceneRenderConfig,
    pub camera: Camera,
    pub component_manager: ComponentManager,
    #[cfg(feature = "physics")]
    pub world: World,
}

impl Scene {
    pub(crate) fn new(ratio: f32, id: u32) -> Self {
        const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 5.0;
        Self {
            id: id,
            switched: true,
            resized: true,
            camera: Camera::new(Isometry::default(), ratio, DEFAULT_VERTICAL_CAMERA_FOV),
            cursor: CursorManager::new(),
            component_manager: ComponentManager::new(),
            render_config: SceneRenderConfig::new(),
            #[cfg(feature = "physics")]
            world: World::new(),
            saved_sprites: vec![],
        }
    }

    pub(crate) fn before_deserialize(&mut self, id: u32, shura: &Shura) {
        let window_size: Dimension<u32> = shura.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        self.id = id;
        self.camera.resize(window_ratio);
        self.cursor
            .compute(&self.camera, &window_size, &shura.input);
        self.component_manager.update_sets(&self.camera);
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

#[cfg(feature = "serialize")]
#[derive(serde::Serialize)]
pub struct SceneSerializer<'a> {
    scene: &'a mut Scene,
    active_type: ComponentTypeId,
    components: FxHashMap<
        ComponentTypeId,
        Vec<(u32, Vec<Option<(&'a u32, &'a dyn erased_serde::Serialize)>>)>,
    >,
}

#[cfg(feature = "serialize")]
impl<'a> SceneSerializer<'a> {
    pub(crate) fn new(scene: &'a mut Scene, active_type: ComponentTypeId) -> Self {
        Self {
            scene,
            active_type,
            components: Default::default(),
        }
    }

    pub fn serialize(&mut self, pretty: bool) -> Option<String> {
        let world_cpy = self.scene.world.clone();
        // let to_remove = vec![];
        for (body_handle, _body) in self.scene.world.bodies() {
            // for 
        }
        let result =  if pretty {
            let pretty_config = PrettyConfig::new();
            ron::ser::to_string_pretty(self, pretty_config).ok()
        } else {
            ron::ser::to_string(self).ok()
        };

        return result;
    }

    pub fn serialize_components<
        C: ComponentController + ComponentIdentifier + serde::Serialize,
    >(
        &'a mut self,
        groups: &[u32],
    ) {
        let type_id = C::IDENTIFIER;
        let mut target = vec![];
        if type_id == self.active_type {
            panic!("Cannot serialize currently used component!");
        }
        for group_id in groups {
            if let Some(group_index) = self.scene.component_manager.group_index(group_id) {
                let group = self.scene.component_manager.group(*group_index).unwrap();
                if let Some(type_index) = group.type_index(type_id) {
                    let type_ref = group.type_ref(*type_index).unwrap();
                    target.push((*group_id, type_ref.serialize_components::<C>()))
                }
            }
        }
        self.components.insert(type_id, target);
    }
}

#[derive(serde::Deserialize)]
pub struct SceneDeserializer {
    components: FxHashMap<ComponentTypeId, Vec<(u32, ron::Value)>>,
}

impl SceneDeserializer {
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
