use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use downcast_rs::{impl_downcast, Downcast};
use parking_lot::RwLock;
use rustc_hash::FxBuildHasher;

#[cfg(feature = "audio")]
use crate::audio::Sound;

#[cfg(feature = "text")]
use crate::text::{Font, FontBuilder, TextMesh, TextSection};

use crate::{
    entity::EntityGroupManager,
    graphics::{
        Camera, CameraBuffer, DefaultAssets, DepthBuffer, Gpu, Instance, InstanceBuffer, Mesh,
        MeshData, Model, ModelBuilder, RenderTarget, Shader, ShaderConfig, ShaderModule,
        ShaderModuleDescriptor, Sprite, SpriteArray, SpriteArrayBuilder, SpriteBuilder,
        SpriteRenderTarget, UniformData, Vertex,
    },
    io::ResourceLoader,
    math::Vector2,
};

use super::{SmartInstanceBuffer, SmartMesh};

pub static GLOBAL_ASSETS: OnceLock<Arc<AssetManager>> = OnceLock::new();
pub trait Asset: Send + Sync + Downcast {
    fn needs_update(&self) -> bool {
        false
    }
    fn prepare(&mut self, _groups: &EntityGroupManager) {}
    fn apply(&mut self, _gpu: &Gpu) {}
}
impl_downcast!(Asset);

pub type AssetKey = &'static str;

pub type AssetDynamic<'a> = dashmap::mapref::one::Ref<'a, AssetKey, Box<dyn Asset>>;
pub type AssetDynamicMut<'a> = dashmap::mapref::one::RefMut<'a, AssetKey, Box<dyn Asset>>;
pub type AssetWrap<'a, A> = dashmap::mapref::one::MappedRef<'a, AssetKey, Box<dyn Asset>, A>;
pub type AssetWrapMut<'a, A> = dashmap::mapref::one::MappedRefMut<'a, AssetKey, Box<dyn Asset>, A>;

pub struct AssetManager {
    pub loader: Arc<dyn ResourceLoader>,
    default_assets: RwLock<DefaultAssets>,
    gpu: Arc<Gpu>,
    assets: DashMap<AssetKey, Box<dyn Asset>, FxBuildHasher>,
}

impl AssetManager {
    pub(crate) fn new(loader: Arc<dyn ResourceLoader>, gpu: Arc<Gpu>) -> Self {
        Self {
            default_assets: RwLock::new(DefaultAssets::new(&gpu)),
            assets: DashMap::with_hasher(FxBuildHasher),
            loader,
            gpu,
        }
    }

    pub(crate) fn prepare(&self, groups: &EntityGroupManager) {
        for mut asset in self.assets.iter_mut() {
            asset.prepare(groups)
        }
    }

    pub(crate) fn apply(&self) {
        for mut asset in self.assets.iter_mut() {
            asset.apply(&self.gpu)
        }
    }

    pub fn default_assets(&self) -> impl Deref<Target = DefaultAssets> + '_ {
        self.default_assets.read()
    }

    pub(crate) fn default_assets_mut(&self) -> impl DerefMut<Target = DefaultAssets> + '_ {
        self.default_assets.write()
    }

    pub fn get_dyn(&self, key: AssetKey) -> AssetDynamic {
        self.assets
            .get(key)
            .unwrap_or_else(|| panic!("Cannot find asset '{key}'!"))
    }

    pub fn get<A: Asset>(&self, key: AssetKey) -> AssetWrap<A> {
        Ref::map(self.get_dyn(key), |asset| {
            asset.downcast_ref::<A>().unwrap_or_else(|| {
                panic!(
                    "Cannot convert asset '{}' to {}, is actually: {}!",
                    key,
                    std::any::type_name::<A>(),
                    std::any::type_name_of_val(asset)
                )
            })
        })
    }

    pub fn get_dyn_mut(&self, key: AssetKey) -> AssetDynamicMut {
        self.assets
            .get_mut(key)
            .unwrap_or_else(|| panic!("Cannot find asset '{key}'!"))
    }

    pub fn get_mut<A: Asset>(&self, key: AssetKey) -> AssetWrapMut<A> {
        RefMut::map(self.get_dyn_mut(key), |asset| {
            let name = std::any::type_name_of_val(asset);
            asset.downcast_mut::<A>().unwrap_or_else(|| {
                panic!(
                    "Cannot convert asset '{}' to {}, is actually: {}!",
                    key,
                    std::any::type_name::<A>(),
                    name
                )
            })
        })
    }

    pub fn unload(&self, key: &'static str) -> Option<Box<dyn Asset>> {
        self.assets.remove(key).map(|a| a.1)
    }

    pub fn sprite(&self, key: AssetKey) -> AssetWrap<Sprite> {
        self.get(key)
    }

    pub fn smart_mesh<V: Vertex>(&self, key: AssetKey) -> AssetWrap<SmartMesh<V>> {
        self.get(key)
    }

    pub fn smart_instances<I: Instance>(&self, key: AssetKey) -> AssetWrap<SmartInstanceBuffer<I>> {
        self.get(key)
    }

    pub fn instances<I: Instance>(&self, key: AssetKey) -> AssetWrap<InstanceBuffer<I>> {
        self.get(key)
    }

    pub fn write_instances<I: Instance>(&self, key: AssetKey, data: &[I]) {
        self.get_mut::<InstanceBuffer<I>>(key)
            .write(&self.gpu, data)
    }

    pub fn mesh<V: Vertex>(&self, key: AssetKey) -> AssetWrap<Mesh<V>> {
        self.get(key)
    }

    pub fn write_mesh<V: Vertex>(&self, key: AssetKey, data: impl MeshData<Vertex = V>) {
        self.get_mut::<Mesh<V>>(key).write(&self.gpu, data)
    }

    pub fn write_text<S: AsRef<str>>(
        &self,
        key: AssetKey,
        font: AssetKey,
        sections: &[TextSection<S>],
    ) {
        self.get_mut::<TextMesh>(key)
            .write(&self.gpu, &self.get(font), sections)
    }

    pub fn uniform<D: bytemuck::Pod + Send + Sync>(
        &self,
        key: AssetKey,
    ) -> AssetWrap<UniformData<D>> {
        self.get(key)
    }

    pub fn camera_buffer<C: Camera>(&self, key: AssetKey) -> AssetWrap<CameraBuffer<C>> {
        self.get(key)
    }

    pub fn render_target<R: RenderTarget>(&self, key: AssetKey) -> AssetWrap<R> {
        self.get(key)
    }

    pub fn sprite_array(&self, key: AssetKey) -> AssetWrap<SpriteArray> {
        self.get(key)
    }

    pub fn text_mesh(&self, key: AssetKey) -> AssetWrap<TextMesh> {
        self.get(key)
    }

    pub fn model(&self, key: AssetKey) -> AssetWrap<Model> {
        self.get(key)
    }

    pub fn shader(&self, key: AssetKey) -> AssetWrap<Shader> {
        self.get(key)
    }

    pub fn shader_module(&self, key: AssetKey) -> AssetWrap<ShaderModule> {
        self.get(key)
    }

    pub fn depth_buffer(&self, key: AssetKey) -> AssetWrap<DepthBuffer> {
        self.get(key)
    }

    #[cfg(feature = "audio")]
    pub fn sound(&self, key: AssetKey) -> AssetWrap<Sound> {
        self.get(key)
    }

    #[cfg(feature = "text")]
    pub fn font(&self, key: AssetKey) -> AssetWrap<Font> {
        self.get(key)
    }

    pub fn load<A: Asset>(&self, key: AssetKey, asset: A) {
        assert!(
            !self.assets.contains_key(key),
            "Asset {key} already exists!"
        );
        self.assets.insert(key, Box::new(asset));
    }

    pub fn load_sprite<D: Deref<Target = [u8]>>(&self, key: AssetKey, desc: SpriteBuilder<D>) {
        self.load(key, self.gpu.create_sprite(desc));
    }

    pub fn load_render_target(&self, key: AssetKey, size: Vector2<u32>) {
        self.load(key, SpriteRenderTarget::new(&self.gpu, size));
    }

    pub fn load_depth_buffer(
        &self,
        key: AssetKey,
        size: Vector2<u32>,
        format: wgpu::TextureFormat,
    ) {
        self.load(key, DepthBuffer::new(&self.gpu, size, format));
    }

    pub fn load_custom_render_target<D: Deref<Target = [u8]>>(
        &self,
        key: AssetKey,
        sprite: SpriteBuilder<D>,
    ) {
        self.load(key, SpriteRenderTarget::custom(&self.gpu, sprite));
    }

    pub fn load_smart_instance_buffer<I: Instance>(
        &self,
        key: AssetKey,
        smart_buffer: SmartInstanceBuffer<I>,
    ) {
        self.load(key, smart_buffer);
    }

    pub fn load_instance_buffer<I: Instance>(&self, key: AssetKey, instances: &[I]) {
        self.load(key, InstanceBuffer::new(&self.gpu, instances));
    }

    pub fn load_camera_buffer<C: Camera>(&self, key: AssetKey, camera: &C) {
        self.load(key, CameraBuffer::new(&self.gpu, camera));
    }

    pub fn load_smart_mesh<V: Vertex>(&self, key: AssetKey, smart_mesh: SmartMesh<V>) {
        self.load(key, smart_mesh);
    }

    pub fn load_mesh<V: Vertex>(&self, key: AssetKey, builder: &dyn MeshData<Vertex = V>) {
        self.load(key, Mesh::new(&self.gpu, builder));
    }

    pub fn load_model(&self, key: AssetKey, builder: ModelBuilder) {
        self.load(key, Model::new(&self.gpu, builder));
    }

    pub fn load_sprite_array<D: Deref<Target = [u8]>>(
        &self,
        key: AssetKey,
        desc: SpriteArrayBuilder<D>,
    ) {
        self.load(key, SpriteArray::new(&self.gpu, desc));
    }

    pub fn load_uniform_data<T: bytemuck::Pod + Send + Sync + 'static>(
        &self,
        key: AssetKey,
        data: T,
    ) {
        self.load(key, UniformData::new(&self.gpu, data));
    }

    pub fn load_shader(&self, key: AssetKey, config: ShaderConfig) {
        self.load(key, Shader::new(&self.gpu, config));
    }

    pub fn load_shader_module(&self, key: AssetKey, desc: ShaderModuleDescriptor<'_>) {
        self.load(key, self.gpu.device.create_shader_module(desc))
    }

    #[cfg(feature = "text")]
    pub fn load_font(&self, key: AssetKey, builder: FontBuilder) {
        self.load(key, Font::new(&self.gpu, builder));
    }

    #[cfg(feature = "text")]
    pub fn load_text_mesh<S: AsRef<str>>(
        &self,
        key: AssetKey,
        font: AssetKey,
        sections: &[TextSection<S>],
    ) {
        self.load(key, TextMesh::new(&self.gpu, &*self.font(font), sections));
    }
}

impl<D: bytemuck::Pod + Send + Sync> Asset for UniformData<D> {}
impl<C: Camera> Asset for CameraBuffer<C> {}
impl<R: RenderTarget> Asset for R {}
impl<V: Vertex> Asset for Mesh<V> {}
impl<I: Instance> Asset for InstanceBuffer<I> {}
impl Asset for Sprite {}
impl Asset for SpriteArray {}
impl Asset for TextMesh {}
impl Asset for Model {}
impl Asset for Shader {}
impl Asset for ShaderModule {}
impl Asset for DepthBuffer {}
#[cfg(feature = "audio")]
impl Asset for Sound {}
#[cfg(feature = "text")]
impl Asset for Font {}
