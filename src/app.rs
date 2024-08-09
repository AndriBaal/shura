use std::sync::Arc;

#[cfg(feature = "gui")]
use crate::gui::Gui;
use crate::{
    context::{Context, RenderContext},
    graphics::{AssetManager, Gpu, GpuConfig, RenderEncoder, GLOBAL_ASSETS, GLOBAL_GPU},
    input::Input,
    io::{AssetLoader, StorageLoader, GLOBAL_ASSET_LOADER, GLOBAL_STORAGE_LOADER},
    math::Vector2,
    scene::{Scene, SceneManager},
    system::{EndReason, UpdateOperation},
    time::TimeManager,
};
#[cfg(feature = "log")]
use crate::{
    log::{info, LoggerBuilder},
    VERSION,
};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(target_arch = "wasm32")]
use rustc_hash::FxHashMap;

pub struct WindowEventManager {
    pub events: Vec<Box<dyn FnMut(&mut Context, &winit::event::WindowEvent)>>,
}

impl Default for WindowEventManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowEventManager {
    pub fn new() -> Self {
        Self {
            events: Default::default(),
        }
    }

    pub fn add(&mut self, event: impl FnMut(&mut Context, &winit::event::WindowEvent) + 'static) {
        self.events.push(Box::new(event));
    }
}

pub struct AppConfig {
    pub window_events: WindowEventManager,
    pub window: winit::window::WindowAttributes,
    pub gpu: GpuConfig,
    pub storage: Arc<dyn StorageLoader>,
    pub assets: Arc<dyn AssetLoader>,
    pub scene_id: u32,
    #[cfg(feature = "framebuffer")]
    pub apply_frame_buffer: bool,
    #[cfg(target_os = "android")]
    pub android: AndroidApp,
    #[cfg(feature = "log")]
    pub logger: Option<LoggerBuilder>,
    #[cfg(target_arch = "wasm32")]
    pub canvas_attrs: FxHashMap<String, String>,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AppConfig {
    pub const FIRST_SCENE_ID: u32 = 0;
    pub fn new(#[cfg(target_os = "android")] android: AndroidApp) -> Self {
        #[cfg(target_arch = "wasm32")]
        let (assets, storage) = (
            crate::io::WebAssetManager,
            crate::io::UnimplementedStorageLoader,
        );

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let (assets, storage) = (
            crate::io::NativeAssetManager,
            crate::io::NativeStorageLoader,
        );

        #[cfg(target_os = "android")]
        let (assets, storage) = (
            crate::io::AndroidAssetManager::new(&android),
            crate::io::UnimplementedStorageLoader,
        );

        AppConfig {
            window: winit::window::WindowAttributes::default()
                .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
                .with_title("App Game"),
            gpu: GpuConfig::default(),
            scene_id: Self::FIRST_SCENE_ID,
            storage: Arc::new(storage),
            assets: Arc::new(assets),
            #[cfg(target_os = "android")]
            android,
            #[cfg(feature = "log")]
            logger: Some(Default::default()),
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas: true,
            #[cfg(target_arch = "wasm32")]
            canvas_attrs: {
                let mut map = FxHashMap::default();
                map.insert("tabindex".into(), "0".into());
                map.insert("oncontextmenu".into(), "return false;".into());
                map.insert(
                    "style".into(),
                    "margin: auto; position: absolute; top: 0; bottom: 0; left: 0; right: 0;"
                        .into(),
                );
                map
            },
            #[cfg(feature = "framebuffer")]
            apply_frame_buffer: true,
            window_events: WindowEventManager::new(),
        }
    }

    pub fn window(mut self, window: winit::window::WindowAttributes) -> Self {
        self.window = window;
        self
    }

    pub fn gpu(mut self, gpu: GpuConfig) -> Self {
        self.gpu = gpu;
        self
    }

    pub fn scene_id(mut self, scene_id: u32) -> Self {
        self.scene_id = scene_id;
        self
    }

    pub fn storage(mut self, storage: impl StorageLoader) -> Self {
        self.storage = Arc::new(storage);
        self
    }

    pub fn assets(mut self, assets: impl AssetLoader) -> Self {
        self.assets = Arc::new(assets);
        self
    }

    pub fn window_event(
        mut self,
        event: impl FnMut(&mut Context, &winit::event::WindowEvent) + 'static,
    ) -> Self {
        self.window_events.add(event);
        self
    }

    #[cfg(feature = "log")]
    pub fn logger(mut self, logger: Option<LoggerBuilder>) -> Self {
        self.logger = logger;
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn canvas_attr(mut self, key: String, value: String) -> Self {
        self.canvas_attrs.insert(key, value);
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn auto_scale_canvas(mut self, auto_scale_canvas: bool) -> Self {
        self.auto_scale_canvas = auto_scale_canvas;
        self
    }
}

enum AppState<S: Into<Scene>, I: FnOnce() -> S> {
    Uninitialized { config: AppConfig, init: I },
    Initialized(App),
}

impl<S: Into<Scene>, I: FnOnce() -> S> AppState<S, I> {
    fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        take_mut::take(self, |state| match state {
            AppState::Initialized(_) => panic!(),
            AppState::Uninitialized { config, init } => {
                Self::Initialized(App::new(event_loop, config, init))
            }
        });
    }
}

impl<S: Into<Scene>, I: FnOnce() -> S> ApplicationHandler<()> for AppState<S, I> {
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        match self {
            AppState::Initialized(app) => app.window.request_redraw(),
            AppState::Uninitialized { .. } => (),
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if !cfg!(target_os = "android") {
            if let winit::event::StartCause::Init = cause {
                match self {
                    AppState::Initialized(app) => app.gpu.resume(&app.window),
                    AppState::Uninitialized { .. } => self.init(event_loop),
                };
            }
        }
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let app = match self {
            AppState::Initialized(app) => app,
            AppState::Uninitialized { .. } => return,
        };
        app.end(event_loop);
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if cfg!(target_os = "android") {
            match self {
                AppState::Initialized(app) => app.gpu.resume(&app.window),
                AppState::Uninitialized { .. } => self.init(event_loop),
            };
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let app = match self {
            AppState::Initialized(app) => app,
            AppState::Uninitialized { .. } => return,
        };

        #[cfg(feature = "gui")]
        app.gui.handle_event(&app.window, &event);

        if !app.window_events.events.is_empty() {
            let scene_id = app.scenes.active_scene_id();
            let scene = app.scenes.get_active_scene();
            let mut scene = scene.borrow_mut();
            let scene = &mut scene;

            let (window_events, _, mut ctx) = Context::new(&scene_id, app, scene, event_loop);
            for e in &mut window_events.events {
                e(&mut ctx, &event);
            }
        }

        if window_id == app.window.id() {
            match &event {
                WindowEvent::RedrawRequested => {
                    app.process_frame(event_loop);
                    if app.end {
                        event_loop.exit();
                    } else {
                        app.window.request_redraw();
                    }
                }
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    app.end = true;
                }
                WindowEvent::Resized(physical_size) => {
                    let width = physical_size.width.max(1);
                    let height = physical_size.height.max(1);
                    app.resize(Vector2::new(width, height));
                }
                _ => app.input.on_event(&event),
            }
        }
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}
    fn memory_warning(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: ()) {}
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
    }
}

pub struct App {
    pub(crate) window_events: WindowEventManager,
    pub(crate) storage: Arc<dyn StorageLoader>,
    pub(crate) assets: Arc<AssetManager>,
    pub(crate) end: bool,
    pub(crate) time: TimeManager,
    pub(crate) scenes: SceneManager,
    pub(crate) window: Arc<Window>,
    pub(crate) input: Input,
    pub(crate) gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub(crate) gui: Gui,
    #[cfg(feature = "audio")]
    pub(crate) audio: AudioManager,
    #[cfg(target_arch = "wasm32")]
    pub(crate) auto_scale_canvas: bool,
    #[cfg(feature = "framebuffer")]
    pub(crate) apply_framebuffer: bool,
}

impl App {
    pub fn run<S: Into<Scene>>(config: AppConfig, init: impl FnOnce() -> S) {
        #[cfg(feature = "log")]
        info!("Using shura version: {}", VERSION);

        #[cfg(target_os = "android")]
        let events = winit::event_loop::EventLoopBuilder::new()
            .with_android_app(config.android)
            .build()
            .unwrap();
        #[cfg(not(target_os = "android"))]
        let events: EventLoop<()> = winit::event_loop::EventLoop::new().unwrap();
        let mut app_state = AppState::Uninitialized { config, init };
        events.run_app(&mut app_state).unwrap();

        #[cfg(feature = "log")]
        info!("Goodbye!");
    }

    fn new<S: Into<Scene>>(
        event_loop: &ActiveEventLoop,
        config: AppConfig,
        init: impl FnOnce() -> S,
    ) -> Self {
        #[cfg(target_os = "android")]
        use winit::platform::android::EventLoopBuilderExtAndroid;

        #[cfg(feature = "log")]
        if let Some(logger) = config.logger {
            logger.init().ok();
        }

        #[cfg(target_os = "android")]
        {
            #[cfg(feature = "log")]
            info!("Android SDK version: {}", AndroidApp::sdk_version());
            crate::asset::ANDROID_ASSETS
                .set(config.android.asset_manager())
                .unwrap();
            crate::asset::ANDROID_DATA
                .set(config.android.internal_data_path().unwrap())
                .unwrap();
        }

        let window = event_loop.create_window(config.window).unwrap();
        let window = Arc::new(window);

        #[cfg(target_arch = "wasm32")]
        {
            use console_error_panic_hook::hook;
            use winit::platform::web::WindowExtWebSys;

            std::panic::set_hook(Box::new(hook));
            let canvas = &web_sys::Element::from(window.canvas().unwrap());
            for (attr, value) in config.canvas_attrs {
                canvas.set_attribute(&attr, &value).unwrap();
            }

            let browser_window = web_sys::window().unwrap();
            let document = browser_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(canvas).ok();
        }

        let gpu = pollster::block_on(Gpu::new(window.clone(), config.gpu));
        let gpu = Arc::new(gpu);
        gpu.resume(&window);

        let assets = Arc::new(AssetManager::new(config.assets.clone(), gpu.clone()));

        GLOBAL_GPU.set(gpu.clone()).ok().unwrap();
        GLOBAL_ASSET_LOADER.set(config.assets.clone()).ok().unwrap();
        GLOBAL_ASSETS.set(assets.clone()).ok().unwrap();
        GLOBAL_STORAGE_LOADER
            .set(config.storage.clone())
            .ok()
            .unwrap();

        let size = gpu.surface_size();
        let scene = (init)();

        Self {
            window_events: config.window_events,
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            #[cfg(feature = "gui")]
            gui: Gui::new(&window, &gpu),
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas: config.auto_scale_canvas,
            #[cfg(feature = "framebuffer")]
            apply_framebuffer: config.apply_frame_buffer,
            window,
            gpu,
            assets,
            storage: config.storage,
            end: false,
            scenes: SceneManager::new(scene.into(), config.scene_id),
            time: TimeManager::new(),
            input: Input::new(size.cast::<f32>()),
        }
    }

    fn resize(&mut self, new_size: Vector2<u32>) {
        #[cfg(feature = "log")]
        info!("Resizing window to: {} x {}", new_size.x, new_size.y,);
        self.scenes.resize();
        self.input.resize(new_size);
        self.gpu.resize(new_size);
        let mut default_assets = self.assets.default_assets_mut();
        default_assets.resize(&self.gpu, new_size);
        #[cfg(feature = "gui")]
        self.gui.resize(new_size);
    }

    fn process_frame(&mut self, event_loop: &ActiveEventLoop) {
        let scene_id = self.scenes.active_scene_id();
        let scene = self.scenes.get_active_scene();
        let mut scene = scene.borrow_mut();
        let scene = &mut scene;
        let surface_size = self.gpu.surface_size();
        #[cfg(target_arch = "wasm32")]
        {
            if self.auto_scale_canvas {
                let browser_window = web_sys::window().unwrap();
                let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
                let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
                let size = winit::dpi::PhysicalSize::new(width, height);
                if size != self.window.inner_size().into() {
                    let _ = self.window.request_inner_size(size);
                }
            }
        }

        let resized = self.scenes.switched().is_some() || scene.screen_config.changed;
        if resized {
            #[cfg(feature = "framebuffer")]
            let scale = scene.screen_config.render_scale();

            #[cfg(feature = "log")]
            {
                let render_size = scene.screen_config.render_size(&self.gpu);

                #[cfg(feature = "framebuffer")]
                let render_size = {
                    Vector2::new(
                        (render_size.x as f32 * scale) as u32,
                        (render_size.y as f32 * scale) as u32,
                    )
                };

                if self.scenes.switched().is_some() {
                    info!("Switched to scene {}!", scene_id);
                }

                info!(
                    "Resizing render target to: {} x {} using present mode: {:?} (VSYNC: {})",
                    render_size.x,
                    render_size.y,
                    self.gpu.surface_config().present_mode,
                    scene.screen_config.vsync()
                );

                #[cfg(feature = "framebuffer")]
                info!("Using framebuffer scale: {}", scale);
            }
            scene.world_camera2d.resize(surface_size);
            scene.world_camera3d.resize(surface_size);

            self.gpu.apply_vsync(scene.screen_config.vsync());
            #[cfg(feature = "framebuffer")]
            {
                let mut default_assets = self.assets.default_assets_mut();
                default_assets.apply_render_scale(&self.gpu, &scene.screen_config);
            }
        }

        self.update(scene_id, scene, event_loop);
        scene.screen_config.changed = false;
        if scene.render_entities || resized {
            self.render(scene_id, scene, event_loop);
        }
        self.input.update();
    }

    fn update(&mut self, scene_id: u32, scene: &mut Scene, event_loop: &ActiveEventLoop) {
        if let Some(max_frame_time) = scene.screen_config.max_frame_time() {
            let last_update = self.time.update();
            if instant::Instant::now() < last_update + max_frame_time {
                return;
            }
        }
        self.time.tick();

        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();
        #[cfg(feature = "gui")]
        self.gui.begin(&self.time.total_duration(), &self.window);
        let (_, systems, mut ctx) = Context::new(&scene_id, self, scene, event_loop);
        let now = ctx.time.update();

        for (_, setup) in systems.setup_systems.drain(..) {
            (setup)(&mut ctx)
        }

        if *ctx.started {
            if let Some(last_id) = ctx.scenes.switched() {
                for (_, switch) in &systems.switch_systems {
                    (switch)(&mut ctx, last_id)
                }
            }
        }

        if ctx.screen_config.changed {
            for (_, resize) in &systems.resize_systems {
                (resize)(&mut ctx);
            }
        }

        let receiver = ctx.tasks.receiver();
        while let Ok(callback) = receiver.try_recv() {
            (callback)(&mut ctx);
        }

        for (_, (update_operation, update)) in &mut systems.update_systems {
            match update_operation {
                UpdateOperation::EveryFrame => (),
                UpdateOperation::EveryNFrame(frames) => {
                    if ctx.time.total_frames() % *frames != 0 {
                        continue;
                    }
                }
                UpdateOperation::UpdaterAfter(last_update, dur) => {
                    if now < *last_update + *dur {
                        continue;
                    } else {
                        *last_update = now;
                    }
                }
            }

            (update)(&mut ctx);
        }
        scene.started = true;
        scene.groups.update(&scene.world_camera2d);
    }

    fn buffer(&mut self, scene_id: u32, scene: &mut Scene, event_loop: &ActiveEventLoop) {
        self.assets.prepare(&scene.groups);
        let (_, _, ctx) = &Context::new(&scene_id, self, scene, event_loop);
        ctx.entities.buffer(&ctx);
        self.assets.apply();

        let mut default_assets = self.assets.default_assets_mut();
        default_assets
            .world_camera2d
            .write(&self.gpu, &scene.world_camera2d);

        default_assets
            .world_camera3d
            .write(&self.gpu, &scene.world_camera3d);

        default_assets
            .times
            .write(&self.gpu, [self.time.total(), self.time.delta()]);
    }

    fn render(&mut self, scene_id: u32, scene: &mut Scene, event_loop: &ActiveEventLoop) {
        self.buffer(scene_id, scene, event_loop);

        let surface_target = self.gpu.start_frame(&self.gpu);
        let default_assets = self.assets.default_assets();

        let (systems, ctx) =
            RenderContext::new(self.assets.clone(), &surface_target, &default_assets, scene);
        let mut encoder =
            RenderEncoder::new(&self.gpu, &self.assets, &default_assets, ctx.target());

        for (_, render) in &systems.render_systems {
            (render)(&ctx, &mut encoder);
        }

        #[cfg(feature = "framebuffer")]
        {
            if self.apply_framebuffer {
                encoder.copy_target(&default_assets.framebuffer, &surface_target);
            }
        }

        #[cfg(feature = "gui")]
        self.gui.render(&surface_target, &self.gpu, &mut encoder);

        encoder.finish();
        self.gpu.submit();
        surface_target.finish();
    }

    fn end(&mut self, event_loop: &ActiveEventLoop) {
        self.end = true;
        let scenes = self.scenes.end_scenes();
        for (id, scene) in scenes {
            let mut scene = scene.borrow_mut();
            let (_, systems, mut ctx) = Context::new(&id, self, &mut scene, event_loop);
            for (_, end) in &systems.end_systems {
                (end)(&mut ctx, EndReason::Close)
            }
        }
    }
}
