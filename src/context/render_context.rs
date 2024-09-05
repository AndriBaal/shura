use std::sync::Arc;

use crate::{
    ecs::{SystemManager, World},
    graphics::{AssetManager, DefaultAssets, Gpu, RenderTarget, SurfaceRenderTarget},
    scene::Scene,
};

#[cfg(feature = "physics")]
use crate::physics::Physics;

pub struct RenderContext<'a> {
    pub assets: Arc<AssetManager>,
    pub gpu: Arc<Gpu>,
    pub surface_target: &'a SurfaceRenderTarget,
    pub default_assets: &'a DefaultAssets,
    #[cfg(feature = "physics")]
    pub physics: &'a Physics,
    pub world: &'a World,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        assets: Arc<AssetManager>,
        gpu: Arc<Gpu>,
        surface_target: &'a SurfaceRenderTarget,
        default_assets: &'a DefaultAssets,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                assets,
                gpu,
                default_assets,
                surface_target,
                #[cfg(feature = "physics")]
                physics: &scene.physics,
                world: &scene.world,
            },
        )
    }

    pub fn target(&self) -> &dyn RenderTarget {
        #[cfg(feature = "framebuffer")]
        return &self.default_assets.framebuffer;

        #[cfg(not(feature = "framebuffer"))]
        return self.surface_target;
    }
}
