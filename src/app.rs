use std::sync::Arc;

#[cfg(feature = "gui")]
use crate::gui::Gui;
use crate::{
    context::{Context, RenderContext},
    graphics::{Gpu, GpuConfig, RenderEncoder, Surface, GLOBAL_GPU},
    input::Input,
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
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::Window,
};

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(target_arch = "wasm32")]
use rustc_hash::FxHashMap;

pub struct AppConfig {
    pub window: winit::window::WindowBuilder,
    pub winit_event: Option<Box<dyn Fn(&winit::event::Event<()>)>>,
    pub gpu: GpuConfig,
    pub scene_id: u32,
    #[cfg(target_os = "android")]
    pub android: AndroidApp,
    #[cfg(feature = "log")]
    pub logger: Option<LoggerBuilder>,
    #[cfg(target_arch = "wasm32")]
    pub canvas_attrs: FxHashMap<String, String>,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl AppConfig {
    pub const FIRST_SCENE_ID: u32 = 0;
    pub fn new(#[cfg(target_os = "android")] android: AndroidApp) -> Self {
        AppConfig {
            window: winit::window::WindowBuilder::new()
                .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
                .with_title("App Game"),
            winit_event: None,
            gpu: GpuConfig::default(),
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
            scene_id: Self::FIRST_SCENE_ID,
        }
    }

    pub fn window(mut self, window: winit::window::WindowBuilder) -> Self {
        self.window = window;
        self
    }

    pub fn gpu(mut self, gpu: GpuConfig) -> Self {
        self.gpu = gpu;
        self
    }

    #[cfg(feature = "log")]
    pub fn logger(mut self, logger: Option<LoggerBuilder>) -> Self {
        self.logger = logger;
        self
    }

    pub fn winit_event(
        mut self,
        event: Option<impl for<'a> Fn(&'a winit::event::Event<()>) + 'static>,
    ) -> Self {
        if let Some(event) = event {
            self.winit_event = Some(Box::new(event));
        } else {
            self.winit_event = None;
        }
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

pub struct App {
    pub end: bool,
    pub resized: bool,
    pub time: TimeManager,
    pub scenes: SceneManager,
    pub window: Arc<Window>,
    pub surface: Surface,
    pub input: Input,
    pub gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: AudioManager,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl App {
    pub fn run(mut config: AppConfig, init: impl FnOnce() -> Scene) {
        let winit_event = config.winit_event.take();
        let first_scene_id = config.scene_id;
        let (events, mut app) = App::new(config);
        let shura_window_id = app.window.id();
        let mut init = Some(init);
        events
            .run(move |event, event_loop| {
                use winit::event::{Event, WindowEvent};
                if let Some(callback) = &winit_event {
                    callback(&event);
                }
                if !app.end {
                    match event {
                        ref e if Surface::start_condition(e) => {
                            app.surface.resume(&app.gpu, app.window.clone());

                            if init.is_some() {
                                let size = app.surface.size();
                                app.gpu.initialize(&app.surface);
                                app.surface.update_msaa(&app.gpu, size);
                                app.input.resize(size);
                                let scene = (init.take().unwrap())();
                                app.scenes.add(first_scene_id, scene);
                            }
                        }
                        Event::WindowEvent {
                            ref event,
                            window_id,
                        } => {
                            #[cfg(feature = "gui")]
                            app.gui.handle_event(event);
                            if window_id == shura_window_id && init.is_none() {
                                match event {
                                    WindowEvent::RedrawRequested => {
                                        app.process_frame();
                                        if app.end {
                                            app.end(event_loop);
                                        } else {
                                            app.window.request_redraw()
                                        }
                                    }
                                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                                        app.end(event_loop);
                                    }
                                    WindowEvent::Resized(physical_size) => {
                                        let mint: mint::Vector2<u32> = (*physical_size).into();
                                        let window_size: Vector2<u32> = mint.into();
                                        app.resize(window_size);
                                    }
                                    _ => app.input.on_event(event),
                                }
                            }
                        }
                        Event::Suspended => {
                            app.surface.suspend();
                        }
                        Event::LoopExiting => {
                            app.end(event_loop);
                        }
                        _ => {}
                    }
                }
            })
            .unwrap();
    }

    fn new(config: AppConfig) -> (EventLoop<()>, Self) {
        #[cfg(target_os = "android")]
        use winit::platform::android::EventLoopBuilderExtAndroid;

        #[cfg(feature = "log")]
        if let Some(logger) = config.logger {
            logger.init().ok();
        }

        #[cfg(feature = "log")]
        info!("Using shura version: {}", VERSION);

        #[cfg(target_os = "android")]
        {
            #[cfg(feature = "log")]
            info!("Android SDK version: {}", AndroidApp::sdk_version());
            crate::resource::ANDROID_ASSETS
                .set(config.android.asset_manager())
                .unwrap();
            crate::resource::ANDROID_DATA
                .set(config.android.internal_data_path().unwrap())
                .unwrap();
        }

        #[cfg(target_os = "android")]
        let events = winit::event_loop::EventLoopBuilder::new()
            .with_android_app(config.android)
            .build()
            .unwrap();
        #[cfg(not(target_os = "android"))]
        let events = winit::event_loop::EventLoop::new().unwrap();
        let window = config.window.build(&events).unwrap();
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

        let mut surface = Surface::new();
        let gpu = pollster::block_on(Gpu::new(&mut surface, window.clone(), config.gpu));
        let gpu = Arc::new(gpu);

        GLOBAL_GPU.set(gpu.clone()).ok().unwrap();

        let shura = Self {
            scenes: SceneManager::new(config.scene_id),
            time: TimeManager::new(),
            input: Input::new(),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(&window, _event_loop, &gpu),
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas: config.auto_scale_canvas,
            resized: false,
            window,
            gpu,
            surface,
        };

        return (events, shura);
    }

    fn resize(&mut self, new_size: Vector2<u32>) {
        if new_size.x > 0 && new_size.y > 0 {
            #[cfg(feature = "log")]
            info!("Resizing window to: {} x {}", new_size.x, new_size.y,);
            let mut default_resources = self.gpu.default_resources_mut();
            self.scenes.resize();
            self.input.resize(new_size);
            self.surface.resize(&self.gpu, new_size);
            default_resources.resize(&self.gpu, new_size);

            #[cfg(feature = "framebuffer")]
            {
                if let Some(scene) = self.scenes.try_get_active_scene() {
                    let scene = scene.borrow();

                    default_resources.apply_render_scale(
                        &self.surface,
                        &self.gpu,
                        scene.screen_config.render_scale(),
                    );
                }
            }
        }
    }

    fn process_frame(&mut self) {
        let scene_id = self.scenes.active_scene_id();
        let scene = self.scenes.get_active_scene();
        let mut scene = scene.borrow_mut();
        let scene = &mut scene;
        self.time.tick();
        if let Some(max_frame_time) = scene.screen_config.max_frame_time() {
            let now = self.time.now();
            let update_time = self.time.update();
            if now < update_time + max_frame_time {
                return;
            }
        }

        let mint: mint::Vector2<u32> = self.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();
        #[cfg(target_arch = "wasm32")]
        {
            if self.auto_scale_canvas {
                let browser_window = web_sys::window().unwrap();
                let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
                let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
                let size = winit::dpi::PhysicalSize::new(width, height);
                if size != self.window.inner_size().into() {
                    self.window.set_min_inner_size(Some(size));
                    // #[cfg(feature = "log")]
                    // {
                    //     info!("Adjusting canvas to browser window!");
                    // }
                }
            }
        }

        self.resized = self.scenes.switched() || scene.screen_config.changed;
        if self.resized {
            #[cfg(feature = "framebuffer")]
            let scale = scene.screen_config.render_scale();

            #[cfg(feature = "log")]
            {
                let render_size = self.surface.size();

                #[cfg(feature = "framebuffer")]
                let render_size = {
                    Vector2::new(
                        (render_size.x as f32 * scale) as u32,
                        (render_size.y as f32 * scale) as u32,
                    )
                };

                if self.scenes.switched() {
                    info!("Switched to scene {}!", scene_id);
                }

                info!(
                    "Resizing render target to: {} x {} using present mode: {:?} (VSYNC: {})",
                    render_size.x,
                    render_size.y,
                    self.surface.config().present_mode,
                    scene.screen_config.vsync()
                );

                #[cfg(feature = "framebuffer")]
                info!("Using framebuffer scale: {}", scale);
            }
            scene.screen_config.changed = false;
            scene.world_camera2d.resize(window_size);
            scene.world_camera3d.resize(window_size);

            self.surface
                .apply_vsync(&self.gpu, scene.screen_config.vsync());
            #[cfg(feature = "framebuffer")]
            {
                let mut default_resources = self.gpu.default_resources_mut();
                default_resources.apply_render_scale(&mut self.surface, &self.gpu, scale);
            }
            #[cfg(feature = "gui")]
            self.gui.resize(self.gpu.render_size());
            // self.time.tick();
        }

        self.update(scene_id, scene);
        if scene.render_entities {
            self.render(scene);
        }
        self.input.update();
    }

    fn update(&mut self, scene_id: u32, scene: &mut Scene) {
        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();
        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.time.total_time_duration(), &self.window);
        let (systems, mut ctx) = Context::new(&scene_id, self, scene);
        let now = ctx.time.update();

        for setup in systems.setup_systems.drain(..) {
            (setup)(&mut ctx)
        }

        let receiver = ctx.tasks.receiver();
        while let Ok(callback) = receiver.try_recv() {
            (callback)(&mut ctx);
        }

        if ctx.resized {
            for resize in &systems.resize_systems {
                (resize)(&mut ctx);
            }
        }

        for (update_operation, update) in &mut systems.update_systems {
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

        scene.groups.update(&scene.world_camera2d);
    }

    fn buffer(&mut self, scene: &mut Scene) {
        let mut default_resources = self.gpu.default_resources_mut();
        scene
            .entities
            .buffer(&mut scene.render_groups, &scene.groups, &scene.world);
        scene.render_groups.apply_buffers(&scene.groups, &self.gpu);
        default_resources
            .world_camera2d
            .write(&self.gpu, &scene.world_camera2d);

        default_resources
            .world_camera3d
            .write(&self.gpu, &scene.world_camera3d);

        default_resources
            .times
            .write(&self.gpu, [self.time.total(), self.time.delta()]);
    }

    fn render(&mut self, scene: &mut Scene) {
        self.buffer(scene);

        let surface_target = self.surface.start_frame(&self.gpu);
        let default_resources = self.gpu.default_resources();

        #[cfg(feature = "framebuffer")]
        let default_target = &default_resources.framebuffer;
        #[cfg(not(feature = "framebuffer"))]
        let default_target = &surface_target;

        let (systems, res) = RenderContext::new(&surface_target, &default_resources, scene);
        let mut encoder = RenderEncoder::new(&self.gpu, default_target, &default_resources);

        for render in &systems.render_systems {
            (render)(&res, &mut encoder);
        }

        #[cfg(feature = "framebuffer")]
        encoder.copy_target(&default_resources.framebuffer, &surface_target);

        #[cfg(feature = "gui")]
        self.gui.render(&self.gpu, &default_resources, &mut encoder);

        encoder.finish();
        self.gpu.submit();
        surface_target.finish()
    }

    fn end(&mut self, event_loop: &EventLoopWindowTarget<()>) {
        self.end = true;
        event_loop.exit();
        let scenes = self.scenes.end_scenes();
        #[cfg(feature = "log")]
        if scenes.len() != 0 {
            info!("Goodbye!");
        }
        for (id, scene) in scenes {
            let mut scene = scene.borrow_mut();
            let (systems, mut ctx) = Context::new(&id, self, &mut scene);
            for end in &systems.end_systems {
                (end)(&mut ctx, EndReason::Close)
            }
        }
    }
}
