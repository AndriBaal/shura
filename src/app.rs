use std::{
    cell::RefCell,
    ops::DerefMut,
    rc::Rc,
    sync::{Arc, OnceLock},
};

pub static GLOBAL_GPU: OnceLock<Arc<Gpu>> = OnceLock::new();

#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::physics::{ActiveEvents, CollideType};
#[cfg(feature = "log")]
use crate::{
    log::{error, info, LoggerBuilder},
    VERSION,
};
use crate::{
    ComponentTypeId, ComponentTypeImplementation, Context,
    DefaultResources, EndReason, FrameManager, Gpu, GpuConfig, Input,
    RenderEncoder, RenderTarget, Scene, SceneCreator, SceneManager, Vector, UpdateOperation,
};
use rustc_hash::FxHashMap;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

/// Configuration for the base of the game engine
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
            // global_states: vec![],
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

// The Option<> is here to keep track of component, that have already been added to scenes and therefore
// can not be registered as a global component.
pub struct GlobalComponents(
    pub(crate)  Rc<
        RefCell<FxHashMap<ComponentTypeId, Option<Rc<RefCell<dyn ComponentTypeImplementation>>>>>,
    >,
);

/// Core of the game engine.
pub struct App {
    pub end: bool,
    pub resized: bool,
    pub frame: FrameManager,
    pub scenes: SceneManager,
    pub globals: GlobalComponents,
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

    /// Start a new game with the given callback to initialize the first [Scene](crate::Scene).
    pub fn run<C: SceneCreator + 'static>(config: AppConfig, init: impl FnOnce() -> C + 'static) {
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
                                        app.end = true;
                                        *control_flow = winit::event_loop::ControlFlow::Exit;
                                        app.end();
                                    }
                                    WindowEvent::Resized(physical_size) => {
                                        let mint: mint::Vector2<u32> = (*physical_size).into();
                                        let window_size: Vector<u32> = mint.into();
                                        app.resize(window_size);
                                    }
                                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                        let mint: mint::Vector2<u32> = (**new_inner_size).into();
                                        let window_size: Vector<u32> = mint.into();
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
                                    let window_size: Vector<u32> = mint.into();
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

    fn new<C: SceneCreator + 'static>(
        window: winit::window::Window,
        _event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        gpu: GpuConfig,
        creator: impl FnOnce() -> C,
        #[cfg(target_arch = "wasm32")] auto_scale_canvas: bool,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(&window, gpu));
        let mint: mint::Vector2<u32> = (window.inner_size()).into();
        let window_size: Vector<u32> = mint.into();
        let defaults = DefaultResources::new(&gpu, window_size);
        let gpu = Arc::new(gpu);

        GLOBAL_GPU.set(gpu.clone()).ok().unwrap();
        let scene = (creator)();
        Self {
            scenes: SceneManager::new(scene.new_id(), scene),
            frame: FrameManager::new(),
            globals: GlobalComponents(Default::default()),
            input: Input::new(window_size),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(_event_loop, &gpu),
            window,
            gpu,
            defaults,
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas,
            resized: false,
        }
    }

    fn resize(&mut self, new_size: Vector<u32>) {
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

    // #[cfg(feature = "physics")]
    // fn world_step(ctx: &mut Context) {
    //     macro_rules! skip_fail {
    //         ($res:expr) => {
    //             match $res {
    //                 Some(val) => val,
    //                 None => {
    //                     continue;
    //                 }
    //             }
    //         };
    //     }

    //     ctx.world.step(ctx.frame);
    //     while let Ok(collision_event) = ctx.world.collision_event() {
    //         let collision_type = if collision_event.started() {
    //             CollideType::Started
    //         } else {
    //             CollideType::Stopped
    //         };
    //         let collider_handle1 = collision_event.collider1();
    //         let collider_handle2 = collision_event.collider2();
    //         let component1 = *skip_fail!(ctx.world.component_from_collider(&collider_handle1));
    //         let component2 = *skip_fail!(ctx.world.component_from_collider(&collider_handle2));
    //         let collider1_events = skip_fail!(ctx.world.collider(collider_handle1)).active_events();
    //         let collider2_events = skip_fail!(ctx.world.collider(collider_handle2)).active_events();

    //         let callback1 = skip_fail!(controllers.collisions().get(&component1.type_id()));
    //         let callback2 = skip_fail!(controllers.collisions().get(&component2.type_id()));

    //         if collider1_events == ActiveEvents::COLLISION_EVENTS {
    //             (callback1)(
    //                 ctx,
    //                 component1,
    //                 component2,
    //                 collider_handle1,
    //                 collider_handle2,
    //                 collision_type,
    //             );
    //         }

    //         if collider2_events == ActiveEvents::COLLISION_EVENTS {
    //             (callback2)(
    //                 ctx,
    //                 component2,
    //                 component1,
    //                 collider_handle2,
    //                 collider_handle1,
    //                 collision_type,
    //             );
    //         }
    //     }
    // }

    fn process_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        while let Some(remove) = self.scenes.remove.pop() {
            if let Some(removed) = self.scenes.scenes.remove(&remove) {
                let mut removed = removed.borrow_mut();
                let mut ctx = Context::new(self, &mut removed);
                // for (_, end) in ctx.components.controllers.clone().ends() {
                //     (end)(&mut ctx, EndReason::RemoveScene)
                // }
            }
        }

        while let Some(add) = self.scenes.add.pop() {
            let id = add.new_id();
            let scene = add.create(self);
            self.scenes.scenes.insert(id, Rc::new(RefCell::new(scene)));
        }

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
        let window_size: Vector<u32> = mint.into();
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
                        info!(
                            "{:?}",
                            browser_window
                                .document()
                                .unwrap()
                                .body()
                                .unwrap()
                                .client_width()
                        );
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
                Vector::new(
                    (render_size.x as f32 * scale) as u32,
                    (render_size.y as f32 * scale) as u32,
                )
            };

            #[cfg(feature = "log")]
            {
                #[cfg(not(feature = "framebuffer"))]
                let render_size = self.gpu.render_size();

                if self.scenes.switched() {
                    info!("Switched to scene {}!", scene.id);
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
            scene.world_camera.resize(window_size);

            self.gpu.apply_vsync(scene.screen_config.vsync());
            #[cfg(feature = "framebuffer")]
            self.defaults.apply_render_scale(&self.gpu, scale);
            #[cfg(feature = "gui")]
            self.gui.resize(self.gpu.render_size());
        }

        self.update(scene);
        if scene.render_components {
            self.defaults.surface.start_frame(&self.gpu)?;
            self.render(scene);
            self.defaults.surface.finish_frame();
        }

        return Ok(());
    }

    fn update(&mut self, scene: &mut Scene) {

        self.frame.update(scene.components.active_groups().len());
        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();
        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame.total_time_duration(), &self.window);
        {
            let (systems, mut ctx) = Context::new(self, scene);
            // #[cfg(feature = "physics")]
            // let (mut done_step, physics_priority, world_force_update_level) = {
            //     if let Some(physics_priority) = ctx.world.physics_priority() {
            //         (false, physics_priority, ctx.world.force_update_level)
            //     } else {
            //         (true, 0, 0)
            //     }
            // };
            let now = ctx.frame.update_time();
            {
                for (update_operation, update) in &mut systems.update_systems
                {
                    // #[cfg(feature = "physics")]
                    // if !done_step
                    //     && *_priority > physics_priority
                    //     && world_force_update_level > update_components
                    // {
                    //     done_step = true;
                    //     Self::world_step(&mut ctx, callbacks);
                    // }

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
            }

            // #[cfg(feature = "physics")]
            // if !done_step && world_force_update_level > update_components {
            //     Self::world_step(&mut ctx, callbacks);
            // }
        }

        scene
            .world_camera
            .apply_target(&scene.world, &scene.components);
        scene
            .groups
            .update(&mut scene.components, &scene.world_camera);
    }

    fn render(&mut self, scene: &mut Scene) {
        scene.components.buffer(&mut scene.world, &self.gpu);
        scene.world_camera.buffer(&self.gpu);
        self.defaults
            .buffer(&self.gpu, self.frame.total_time(), self.frame.frame_time());

        let (systems, ctx) = Context::new(self, scene);
        let mut encoder = RenderEncoder::new(&ctx.gpu, &ctx.defaults, &ctx.world_camera);


        {
            for render in &systems.render_systems {
                (render)(&ctx, &mut encoder);
            }
            // for (_priority, render, target) in callbacks.renders() {
            //     if let Some((clear, target)) = (target)(&mut components) {
            //         if target as *const _ != components.renderer.target() as *const _ {
            //             let encoder = unsafe { &mut *encoder_ptr };
            //             drop(components);
            //             components = ComponentRenderer::new(&ctx, encoder.renderer(target, clear));
            //         }
            //     }
            //     (render)(&mut components);
            //     if let Some(screenshot) = components.screenshot.take() {
            //         let encoder = unsafe { &mut *encoder_ptr };
            //         let target_ptr: *const dyn RenderTarget =
            //             components.renderer.target as *const _;
            //         let target = unsafe { target_ptr.as_ref().unwrap() };
            //         drop(components);

            //         if let Some(sprite) = target.downcast_ref::<crate::SpriteRenderTarget>() {
            //             encoder.copy_target(sprite, screenshot);
            //         } else {
            //             encoder.deep_copy_target(target, screenshot);
            //         }

            //         components = ComponentRenderer::new(&ctx, encoder.renderer(target, None));
            //     }
            // }
        }

        #[cfg(feature = "framebuffer")]
        encoder.copy_target(&ctx.defaults.framebuffer, &ctx.defaults.surface);

        #[cfg(feature = "gui")]
        ctx.gui.render(&ctx.gpu, &ctx.defaults, &mut encoder);

        encoder.submit(&ctx.gpu);
    }

    fn end(&mut self) {
        for (_, scene) in self.scenes.end_scenes() {
            let mut scene = scene.borrow_mut();
            let mut ctx = Context::new(self, &mut scene);
            // for (_, end) in ctx.components.controllers.clone().ends() {
            //     (end)(&mut ctx, EndReason::EndProgram)
            // }
        }
    }
}
