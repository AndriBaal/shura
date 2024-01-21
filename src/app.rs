use std::sync::Arc;

#[cfg(feature = "gui")]
use crate::gui::Gui;
use crate::{
    context::{Context, RenderContext},
    entity::GlobalEntities,
    graphics::{DefaultResources, Gpu, GpuConfig, RenderEncoder, GLOBAL_GPU},
    input::Input,
    math::Vector2,
    scene::{Scene, SceneManager},
    system::{EndReason, UpdateOperation},
    time::FrameManager,
};
#[cfg(feature = "log")]
use crate::{
    log::{error, info, LoggerBuilder},
    VERSION,
};
use wgpu::Surface;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::{event_loop::EventLoopWindowTarget, window::Window};

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

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

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
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
    pub frame: FrameManager,
    pub scenes: SceneManager,
    pub window: Arc<Window>,
    pub surface: Surface,
    pub globals: GlobalEntities,
    pub input: Input,
    pub defaults: DefaultResources,
    pub gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: AudioManager,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl App {
    pub fn run(config: AppConfig, init: impl FnOnce() -> Scene) {
        #[cfg(target_os = "android")]
        use winit::platform::android::EventLoopBuilderExtAndroid;

        #[cfg(feature = "log")]
        if let Some(logger) = config.logger {
            logger.init().ok();
        }

        #[cfg(feature = "log")]
        info!("Using shura version: {}", VERSION);

        #[cfg(target_os = "android")]
        let events = winit::event_loop::EventLoopBuilder::new()
            .with_android_app(config.android)
            .build();
        #[cfg(not(target_os = "android"))]
        let events = winit::event_loop::EventLoop::new().unwrap();
        let window = config.window.build(&events).unwrap();
        let shura_window_id = window.id();

        #[cfg(target_arch = "wasm32")]
        {
            use console_error_panic_hook::hook;
            use winit::platform::web::WindowExtWebSys;

            std::panic::set_hook(Box::new(hook));
            let canvas = &web_sys::Element::from(window.canvas());
            for (attr, value) in config.canvas_attrs {
                canvas.set_attribute(attr, value).unwrap();
            }

            let browser_window = web_sys::window().unwrap();
            let document = browser_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(canvas).ok();
        }

        let mut app = App::new(
            Arc::new(window),
            &events,
            config.gpu.clone(),
            config.scene_id,
            init,
            #[cfg(target_arch = "wasm32")]
            config.auto_scale_canvas,
        );

        events
            .run(move |event, event_loop| {
                use winit::event::{Event, WindowEvent};
                if let Some(callback) = &config.winit_event {
                    callback(&event);
                }
                if !app.end {
                    match event {
                        Event::WindowEvent {
                            ref event,
                            window_id,
                        } => {
                            #[cfg(feature = "gui")]
                            app.gui.handle_event(event);
                            if window_id == shura_window_id {
                                match event {
                                    WindowEvent::RedrawRequested => {
                                        let frame_result = app.process_frame();
                                        match frame_result {
                                            Ok(_) => {}
                                            Err(
                                                wgpu::SurfaceError::Lost
                                                | wgpu::SurfaceError::Outdated,
                                            ) => {
                                                #[cfg(feature = "log")]
                                                error!("Lost surface!");
                                                let mint: mint::Vector2<u32> =
                                                    app.window.inner_size().into();
                                                let window_size: Vector2<u32> = mint.into();
                                                app.resize(window_size);
                                            }
                                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                                #[cfg(feature = "log")]
                                                error!("Not enough memory!");
                                                app.end(event_loop);
                                            }
                                            Err(_e) => {
                                                #[cfg(feature = "log")]
                                                error!("Render error: {:?}", _e)
                                            }
                                        }

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
                        Event::LoopExiting => {
                            app.end(event_loop);
                        }
                        #[cfg(target_os = "android")]
                        Event::Resumed => {
                            app.gpu.resume(&app.window);
                        }
                        _ => {}
                    }
                }
            })
            .unwrap();
    }

    fn new(
        window: Arc<Window>,
        _event_loop: &EventLoopWindowTarget<()>,
        gpu: GpuConfig,
        scene_id: u32,
        scene: impl FnOnce() -> Scene,
        #[cfg(target_arch = "wasm32")] auto_scale_canvas: bool,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(window.clone(), gpu));
        let mint: mint::Vector2<u32> = (window.inner_size()).into();
        let window_size: Vector2<u32> = mint.into();
        let defaults = DefaultResources::new(&gpu, window_size);
        let gpu = Arc::new(gpu);
        let globals = GlobalEntities::default();

        GLOBAL_GPU.set(gpu.clone()).ok().unwrap();
        let scene = (scene)();
        Self {
            scenes: SceneManager::new(scene_id, scene, &globals),
            frame: FrameManager::new(),
            input: Input::new(window_size),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(&window, _event_loop, &gpu),
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas,
            resized: false,
            globals: globals,
            window,
            gpu,
            defaults,
        }
    }

    fn resize(&mut self, new_size: Vector2<u32>) {
        if new_size.x > 0 && new_size.y > 0 {
            #[cfg(feature = "log")]
            info!("Resizing window to: {} x {}", new_size.x, new_size.y,);
            self.scenes.resize();
            self.input.resize(new_size);
            self.gpu.resize(new_size);
            self.defaults.resize(&self.gpu, new_size);

            #[cfg(feature = "framebuffer")]
            {
                if let Some(scene) = self.scenes.try_get_active_scene() {
                    let scene = scene.borrow();

                    self.defaults
                        .apply_render_scale(&self.gpu, scene.screen_config.render_scale());
                }
            }
        }
    }

    fn process_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let scene_id = self.scenes.active_scene_id();
        let scene = self.scenes.get_active_scene();
        let mut scene = scene.borrow_mut();
        let scene = &mut scene;
        if let Some(max_frame_time) = scene.screen_config.max_frame_time() {
            let now = self.frame.now();
            let update_time = self.frame.update_time();
            if now < update_time + max_frame_time {
                return Ok(());
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
                    self.window.set_inner_size(size);
                    #[cfg(feature = "log")]
                    {
                        info!("Adjusting canvas to browser window!");
                    }
                }
            }
        }

        self.resized = self.scenes.switched() || scene.screen_config.changed;
        if self.resized {
            #[cfg(feature = "framebuffer")]
            let scale = scene.screen_config.render_scale();

            #[cfg(feature = "log")]
            {
                #[cfg(not(feature = "framebuffer"))]
                let render_size = self.gpu.render_size();

                #[cfg(feature = "framebuffer")]
                let render_size = {
                    let render_size = self.gpu.render_size();
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
                    self.gpu.config.lock().unwrap().present_mode,
                    scene.screen_config.vsync()
                );

                #[cfg(feature = "framebuffer")]
                info!("Using framebuffer scale: {}", scale);
            }
            scene.screen_config.changed = false;
            scene.world_camera2d.resize(window_size);
            scene.world_camera3d.resize(window_size);

            self.gpu.apply_vsync(scene.screen_config.vsync());
            #[cfg(feature = "framebuffer")]
            self.defaults.apply_render_scale(&self.gpu, scale);
            #[cfg(feature = "gui")]
            self.gui.resize(self.gpu.render_size());
        }

        self.update(scene_id, scene);
        if scene.render_entities {
            self.defaults.surface.start_frame(&self.gpu)?;
            self.render(scene_id, scene);
            self.defaults.surface.finish_frame();
        }
        self.frame.update();
        Ok(())
    }

    fn update(&mut self, scene_id: u32, scene: &mut Scene) {
        self.input.update();
        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();
        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame.total_time_duration(), &self.window);
        let (systems, mut ctx) = Context::new(&scene_id, self, scene);
        let now = ctx.frame.update_time();

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
                    if ctx.frame.total_frames() % *frames != 0 {
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

    fn render(&mut self, _scene_id: u32, scene: &mut Scene) {
        scene
            .entities
            .buffer(&mut scene.render_groups, &scene.groups, &scene.world);
        scene.render_groups.apply_buffers(&scene.groups, &self.gpu);
        self.defaults
            .world_camera2d
            .write(&self.gpu, &scene.world_camera2d);

        self.defaults
            .world_camera3d
            .write(&self.gpu, &scene.world_camera3d);

        self.defaults.times.write(
            &self.gpu,
            [self.frame.total_time(), self.frame.delta_time()],
        );

        let (systems, res) = RenderContext::new(&self.defaults, scene);
        let mut encoder = RenderEncoder::new(&self.gpu, &self.defaults);

        for render in &systems.render_systems {
            (render)(&res, &mut encoder);
        }

        #[cfg(feature = "framebuffer")]
        encoder.copy_target(&self.defaults.framebuffer, &self.defaults.surface);

        #[cfg(feature = "gui")]
        self.gui.render(&self.gpu, &self.defaults, &mut encoder);

        encoder.finish();
        self.gpu.submit();
    }

    fn end(&mut self, event_loop: &EventLoopWindowTarget<()>) {
        self.end = true;
        event_loop.exit();
        for (id, scene) in self.scenes.end_scenes() {
            let mut scene = scene.borrow_mut();
            let (systems, mut ctx) = Context::new(&id, self, &mut scene);
            for end in &systems.end_systems {
                (end)(&mut ctx, EndReason::Close)
            }
        }
    }
}
