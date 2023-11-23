use std::{
    cell::RefCell,
    ops::DerefMut,
    rc::Rc,
    sync::{Arc, OnceLock},
};

pub static GLOBAL_GPU: OnceLock<Arc<Gpu>> = OnceLock::new();

#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "log")]
use crate::{
    log::{error, info, LoggerBuilder},
    VERSION,
};
use crate::{
    Context, DefaultResources, EndReason, EntityTypeId, EntityTypeImplementation, FrameManager,
    Gpu, GpuConfig, Input, RenderContext, RenderEncoder, Scene, SceneCreator, SceneManager,
    UpdateOperation, Vector2,
};
use rustc_hash::FxHashMap;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

pub struct AppConfig {
    pub window: winit::window::WindowBuilder,
    pub gpu: GpuConfig,
    #[cfg(target_os = "android")]
    pub android: AndroidApp,
    #[cfg(feature = "log")]
    pub logger: Option<LoggerBuilder>,
    #[cfg(target_arch = "wasm32")]
    pub canvas_attrs: FxHashMap<&'static str, &'static str>,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl AppConfig {
    pub fn default(#[cfg(target_os = "android")] android: AndroidApp) -> Self {
        AppConfig {
            window: winit::window::WindowBuilder::new()
                .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
                .with_title("App Game"),
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
                map.insert("tabindex", "0");
                map.insert("oncontextmenu", "return false;");
                map.insert(
                    "style",
                    "margin: auto; position: absolute; top: 0; bottom: 0; left: 0; right: 0;",
                );
                map
            },
        }
    }
}

// The Option<> is here to keep track of entities, that have already been added to scenes and therefore
// can not be registered as a global entities.
pub struct GlobalEntitys(
    pub(crate) 
        Rc<RefCell<FxHashMap<EntityTypeId, Option<Rc<RefCell<dyn EntityTypeImplementation>>>>>>,
);

pub struct App {
    pub end: bool,
    pub resized: bool,
    pub frame: FrameManager,
    pub scenes: SceneManager,
    pub globals: GlobalEntitys,
    pub window: winit::window::Window,
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
    pub fn run<S: SceneCreator + 'static>(config: AppConfig, init: impl FnOnce() -> S + 'static) {
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
        let events = winit::event_loop::EventLoop::new();
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

        let mut init = Some(init);
        let mut window = Some(window);
        let mut app: Option<App> = if cfg!(target_os = "android") {
            None
        } else {
            Some(App::new(
                window.take().unwrap(),
                &events,
                config.gpu.clone(),
                init.take().unwrap(),
                #[cfg(target_arch = "wasm32")]
                config.auto_scale_canvas,
            ))
        };

        events.run(move |event, _target, control_flow| {
            use winit::event::{Event, WindowEvent};
            if let Some(app) = &mut app {
                if !app.end {
                    match event {
                        Event::WindowEvent {
                            ref event,
                            window_id,
                        } => {
                            #[cfg(feature = "gui")]
                            app.gui.handle_event(&event);
                            if window_id == shura_window_id {
                                match event {
                                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                                        *control_flow = winit::event_loop::ControlFlow::Exit;
                                        app.end = true;
                                        app.end();
                                    }
                                    WindowEvent::Resized(physical_size) => {
                                        let mint: mint::Vector2<u32> = (*physical_size).into();
                                        let window_size: Vector2<u32> = mint.into();
                                        app.resize(window_size);
                                    }
                                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                        let mint: mint::Vector2<u32> = (**new_inner_size).into();
                                        let window_size: Vector2<u32> = mint.into();
                                        app.resize(window_size);
                                    }
                                    _ => app.input.on_event(event),
                                }
                            }
                        }
                        Event::RedrawRequested(window_id) if window_id == shura_window_id => {
                            let frame_result = app.process_frame();
                            match frame_result {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    #[cfg(feature = "log")]
                                    error!("Lost surface!");
                                    let mint: mint::Vector2<u32> = app.window.inner_size().into();
                                    let window_size: Vector2<u32> = mint.into();
                                    app.resize(window_size);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    #[cfg(feature = "log")]
                                    error!("Not enough memory!");
                                    *control_flow = winit::event_loop::ControlFlow::Exit
                                }
                                Err(_e) => {
                                    #[cfg(feature = "log")]
                                    error!("Render error: {:?}", _e)
                                }
                            }

                            if app.end {
                                *control_flow = winit::event_loop::ControlFlow::Exit;
                            }
                        }
                        Event::MainEventsCleared => {
                            app.window.request_redraw();
                        }
                        Event::RedrawEventsCleared => {
                            app.input.update();
                        }
                        #[cfg(target_os = "android")]
                        Event::Resumed => {
                            app.gpu.resume(&app.window);
                        }
                        Event::LoopDestroyed => {
                            app.end();
                        }
                        _ => {}
                    }
                }
            } else {
                #[cfg(target_os = "android")]
                match event {
                    Event::Resumed => {
                        app = Some(App::new(
                            window.take().unwrap(),
                            &_target,
                            config.gpu.clone(),
                            init.take().unwrap(),
                            #[cfg(target_arch = "wasm32")]
                            config.auto_scale_canvas,
                        ))
                    }
                    _ => {}
                }
            }
        });
    }

    fn new<S: SceneCreator + 'static>(
        window: winit::window::Window,
        _event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        gpu: GpuConfig,
        creator: impl FnOnce() -> S,
        #[cfg(target_arch = "wasm32")] auto_scale_canvas: bool,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(&window, gpu));
        let mint: mint::Vector2<u32> = (window.inner_size()).into();
        let window_size: Vector2<u32> = mint.into();
        let defaults = DefaultResources::new(&gpu, window_size);
        let gpu = Arc::new(gpu);

        GLOBAL_GPU.set(gpu.clone()).ok().unwrap();
        let scene = (creator)();
        Self {
            scenes: SceneManager::new(scene.new_id(), scene),
            frame: FrameManager::new(),
            globals: GlobalEntitys(Default::default()),
            input: Input::new(window_size),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(&window, _event_loop, &gpu),
            window,
            gpu,
            defaults,
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas,
            resized: false,
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
        while let Some(remove) = self.scenes.remove.pop() {
            if let Some(removed) = self.scenes.scenes.remove(&remove) {
                let mut removed = removed.borrow_mut();
                let (systems, mut ctx) = Context::new(&remove, self, &mut removed);
                for end in &systems.end_systems {
                    (end)(&mut ctx, EndReason::RemoveScene)
                }
            }
        }

        while let Some(add) = self.scenes.add.pop() {
            let id = add.new_id();
            let scene = add.create(self);
            if let Some(old) = self.scenes.scenes.insert(id, Rc::new(RefCell::new(scene))) {
                let mut removed = old.borrow_mut();
                let (systems, mut ctx) = Context::new(&id, self, &mut removed);
                for end in &systems.end_systems {
                    (end)(&mut ctx, EndReason::Replaced)
                }
            }
        }

        let scene_id = self.scenes.active_scene_id();
        let scene = self.scenes.get_active_scene();
        let mut scene = scene.borrow_mut();
        let scene = scene.deref_mut();
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
            #[cfg(feature = "framebuffer")]
            let render_size = {
                let render_size = self.gpu.render_size();
                Vector2::new(
                    (render_size.x as f32 * scale) as u32,
                    (render_size.y as f32 * scale) as u32,
                )
            };

            #[cfg(feature = "log")]
            {
                #[cfg(not(feature = "framebuffer"))]
                let render_size = self.gpu.render_size();

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

        return Ok(());
    }

    fn update(&mut self, scene_id: u32, scene: &mut Scene) {
        self.frame.update(scene.entities.active_groups().len());
        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();
        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame.total_time_duration(), &self.window);
        let (systems, mut ctx) = Context::new(&scene_id, self, scene);
        let now = ctx.frame.update_time();

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

        scene
            .groups
            .update(&mut scene.entities, &scene.world_camera2d);
    }

    fn render(&mut self, _scene_id: u32, scene: &mut Scene) {
        scene.entities.buffer(&mut scene.components, &scene.world);
        scene.components.apply_buffers(&self.gpu);
        self.defaults
            .world_camera2d
            .write(&self.gpu, &scene.world_camera2d);

        self.defaults
            .world_camera3d
            .write(&self.gpu, &scene.world_camera3d);

        self.defaults.times.write(
            &self.gpu,
            [self.frame.total_time(), self.frame.frame_time()],
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

    fn end(&mut self) {
        for (id, scene) in self.scenes.end_scenes() {
            let mut scene = scene.borrow_mut();
            let (systems, mut ctx) = Context::new(&id, self, &mut scene);
            for end in &systems.end_systems {
                (end)(&mut ctx, EndReason::EndProgram)
            }
        }
    }
}
