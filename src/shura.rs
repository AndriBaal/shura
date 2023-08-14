use std::{cell::RefCell, ops::DerefMut, rc::Rc, sync::Arc};

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
    ComponentRenderer, ComponentTypeId, ComponentTypeImplementation, Context, ContextUse,
    ControllerManager, EndReason, FrameManager, Gpu, GpuConfig, GpuDefaults, Input, RenderEncoder,
    Renderer, SceneCreator, SceneManager, Vector,
};
use rustc_hash::FxHashMap;
#[cfg(target_arch = "wasm32")]
use rustc_hash::FxHashMap;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

/// Configuration for the base of the game engine
pub struct ShuraConfig {
    pub window: winit::window::WindowBuilder,
    pub gpu: GpuConfig,
    #[cfg(target_os = "android")]
    pub app: AndroidApp,
    #[cfg(feature = "log")]
    pub logger: Option<LoggerBuilder>,
    #[cfg(target_arch = "wasm32")]
    pub canvas_attrs: FxHashMap<&'static str, &'static str>,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl ShuraConfig {
    pub fn default(#[cfg(target_os = "android")] app: AndroidApp) -> Self {
        ShuraConfig {
            // global_states: vec![],
            window: winit::window::WindowBuilder::new()
                .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
                .with_title("Shura Game"),
            gpu: GpuConfig::default(),
            #[cfg(target_os = "android")]
            app,
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

impl ShuraConfig {
    /// Start a new game with the given callback to initialize the first [Scene](crate::Scene).
    pub fn init<C: SceneCreator + 'static>(self, init: C) {
        #[cfg(target_os = "android")]
        use winit::platform::android::EventLoopBuilderExtAndroid;

        #[cfg(feature = "log")]
        if let Some(logger) = self.logger {
            logger.init().ok();
        }

        #[cfg(feature = "log")]
        info!("Using shura version: {}", VERSION);

        #[cfg(target_os = "android")]
        let events = winit::event_loop::EventLoopBuilder::new()
            .with_android_app(self.app)
            .build();
        #[cfg(not(target_os = "android"))]
        let events = winit::event_loop::EventLoop::new();
        let window = self.window.build(&events).unwrap();
        let shura_window_id = window.id();

        #[cfg(target_arch = "wasm32")]
        {
            use console_error_panic_hook::hook;
            use winit::platform::web::WindowExtWebSys;

            std::panic::set_hook(Box::new(hook));
            let canvas = &web_sys::Element::from(window.canvas());
            for (attr, value) in self.canvas_attrs {
                canvas.set_attribute(attr, value).unwrap();
            }

            let browser_window = web_sys::window().unwrap();
            let document = browser_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(canvas).ok();
        }

        let mut init = Some(init);
        let mut window = Some(window);
        let mut shura: Option<Shura> = if cfg!(target_os = "android") {
            None
        } else {
            Some(Shura::new(
                window.take().unwrap(),
                &events,
                self.gpu.clone(),
                init.take().unwrap(),
                #[cfg(target_arch = "wasm32")]
                self.auto_scale_canvas,
            ))
        };

        events.run(move |event, _target, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;
            use winit::event::{Event, WindowEvent};
            if let Some(shura) = &mut shura {
                if !shura.end {
                    match event {
                        Event::WindowEvent {
                            ref event,
                            window_id,
                        } => {
                            #[cfg(feature = "gui")]
                            shura.gui.handle_event(&event);
                            if window_id == shura_window_id {
                                match event {
                                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                                        shura.end = true;
                                        *control_flow = winit::event_loop::ControlFlow::Exit;
                                        shura.end();
                                    }
                                    WindowEvent::Resized(physical_size) => {
                                        let mint: mint::Vector2<u32> = (*physical_size).into();
                                        let window_size: Vector<u32> = mint.into();
                                        shura.resize(window_size);
                                    }
                                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                        let mint: mint::Vector2<u32> = (**new_inner_size).into();
                                        let window_size: Vector<u32> = mint.into();
                                        shura.resize(window_size);
                                    }
                                    _ => shura.input.on_event(event),
                                }
                            }
                        }
                        Event::RedrawRequested(window_id) if window_id == shura_window_id => {
                            let update_status = shura.update();
                            match update_status {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    #[cfg(feature = "log")]
                                    error!("Lost surface!");
                                    let mint: mint::Vector2<u32> = shura.window.inner_size().into();
                                    let window_size: Vector<u32> = mint.into();
                                    shura.resize(window_size);
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

                            if shura.end {
                                *control_flow = winit::event_loop::ControlFlow::Exit;
                                shura.end();
                            }
                        }
                        Event::MainEventsCleared => {
                            // #[cfg(target_os = "windows")]
                            // shura.gpu.instance.poll_all(true);
                            shura.window.request_redraw();
                        }
                        #[cfg(target_os = "android")]
                        Event::Resumed => {
                            shura.gpu.resume(&shura.window);
                        }
                        _ => {}
                    }
                }
            } else {
                #[cfg(target_os = "android")]
                match event {
                    Event::Resumed => {
                        shura = Some(Shura::new(
                            window.take().unwrap(),
                            &_target,
                            self.gpu.clone(),
                            init.take().unwrap(),
                            #[cfg(target_arch = "wasm32")]
                            self.auto_scale_canvas,
                        ))
                    }
                    _ => {}
                }
            }
        });
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
// The Option<> is here to keep track of component, that have already been added to scenes and therefore
// can not be registered as a global component.
pub struct GlobalComponents(
    pub(crate)  Rc<
        RefCell<FxHashMap<ComponentTypeId, Option<Rc<RefCell<dyn ComponentTypeImplementation>>>>>,
    >,
);

/// Core of the game engine.
pub struct Shura {
    pub end: bool,
    pub frame: FrameManager,
    pub scenes: SceneManager,
    pub globals: GlobalComponents,
    pub window: winit::window::Window,
    pub input: Input,
    pub defaults: GpuDefaults,
    pub gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: AudioManager,
    #[cfg(target_arch = "wasm32")]
    pub auto_scale_canvas: bool,
}

impl Shura {
    fn new<C: SceneCreator + 'static>(
        window: winit::window::Window,
        _event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        gpu: GpuConfig,
        creator: C,
        #[cfg(target_arch = "wasm32")] auto_scale_canvas: bool,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(&window, gpu));
        let mint: mint::Vector2<u32> = (window.inner_size()).into();
        let window_size: Vector<u32> = mint.into();
        let defaults = GpuDefaults::new(&gpu, window_size);

        Self {
            scenes: SceneManager::new(creator.new_id(), creator),
            frame: FrameManager::new(),
            globals: GlobalComponents(Default::default()),
            input: Input::new(window_size),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(_event_loop, &gpu),
            window,
            gpu: Arc::new(gpu),
            defaults,
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas,
        }
    }

    fn resize(&mut self, new_size: Vector<u32>) {
        if new_size.x > 0 && new_size.y > 0 {
            // #[cfg(target_os = "android")]
            // let new_size = {
            //     match self.window.config().orientation() {
            //         Orientation::Port => {
            //             Vector::new(new_size.x.min(new_size.y), new_size.x.max(new_size.y))
            //         },
            //         Orientation::Land => {
            //             Vector::new(new_size.x.max(new_size.y), new_size.x.min(new_size.y))
            //         },
            //         _ => new_size
            //     }
            // };
            #[cfg(feature = "log")]
            info!("Resizing window to: {} x {}", new_size.x, new_size.y,);
            self.scenes.resize();
            self.input.resize(new_size);
            self.gpu.resize(new_size);
            self.defaults.resize(&self.gpu, new_size);
        }
    }

    #[cfg(feature = "physics")]
    fn world_step(ctx: &mut Context, controllers: &ControllerManager) {
        macro_rules! skip_fail {
            ($res:expr) => {
                match $res {
                    Some(val) => val,
                    None => {
                        continue;
                    }
                }
            };
        }

        ctx.world.step(ctx.frame);
        while let Ok(collision_event) = ctx.world.collision_event() {
            let collision_type = if collision_event.started() {
                CollideType::Started
            } else {
                CollideType::Stopped
            };
            let collider_handle1 = collision_event.collider1();
            let collider_handle2 = collision_event.collider2();
            let component1 = *skip_fail!(ctx.world.component_from_collider(&collider_handle1));
            let component2 = *skip_fail!(ctx.world.component_from_collider(&collider_handle2));
            let collider1_events = skip_fail!(ctx.world.collider(collider_handle1)).active_events();
            let collider2_events = skip_fail!(ctx.world.collider(collider_handle2)).active_events();

            let callback1 = skip_fail!(controllers.collisions().get(&component1.type_id()));
            let callback2 = skip_fail!(controllers.collisions().get(&component2.type_id()));

            if collider1_events == ActiveEvents::COLLISION_EVENTS {
                (callback1)(
                    ctx,
                    component1,
                    component2,
                    collider_handle1,
                    collider_handle2,
                    collision_type,
                );
            }

            if collider2_events == ActiveEvents::COLLISION_EVENTS {
                (callback2)(
                    ctx,
                    component2,
                    component1,
                    collider_handle2,
                    collider_handle1,
                    collision_type,
                );
            }
        }
    }

    fn update(&mut self) -> Result<(), wgpu::SurfaceError> {
        while let Some(remove) = self.scenes.remove.pop() {
            if let Some(removed) = self.scenes.scenes.remove(&remove) {
                let mut removed = removed.borrow_mut();
                let mut ctx = Context::new(self, &mut removed, ContextUse::Update);
                for (_, end) in ctx.components.controllers.clone().ends() {
                    (end)(&mut ctx, EndReason::RemoveScene)
                }
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
        if scene.started {
            #[cfg(feature = "log")]
            info!("Initializing scene {}!", scene.id);
            scene.started = false;
            scene
                .groups
                .update(&mut scene.components, &self.defaults.world_camera);
        }

        if self.scenes.switched() || scene.screen_config.changed {
            let scale = scene.screen_config.render_scale();
            let render_size = self.gpu.render_size(scale);
            #[cfg(feature = "log")]
            {
                if self.scenes.switched() {
                    info!("Switched to scene {}!", scene.id);
                }

                info!(
                    "Resizing render target to: {} x {} using VSYNC: {}",
                    render_size.x,
                    render_size.y,
                    scene.screen_config.vsync()
                );
            }
            scene.screen_config.changed = false;
            scene.world_camera.resize(window_size);

            self.gpu.apply_vsync(scene.screen_config.vsync());
            self.defaults.apply_render_scale(&self.gpu, scale);

            #[cfg(feature = "gui")]
            self.gui.resize(render_size);
        }

        self.frame.update();
        #[cfg(feature = "gamepad")]
        self.input.sync_gamepad();

        let output = self.gpu.surface.get_current_texture()?;

        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame.total_time_duration(), &self.window);
        Rc::get_mut(&mut scene.components.controllers)
            .unwrap()
            .apply();
        let callbacks_rc = scene.components.controllers.clone();
        let callbacks: &ControllerManager = &callbacks_rc;
        {
            let mut ctx = Context::new(self, scene, ContextUse::Update);
            #[cfg(feature = "physics")]
            let (mut done_step, physics_priority, world_force_update_level) = {
                if let Some(physics_priority) = ctx.world.physics_priority() {
                    (false, physics_priority, ctx.world.force_update_level)
                } else {
                    (true, 0, 0)
                }
            };
            let update_components = *ctx.update_components;
            let now = ctx.frame.update_time();
            {
                for (_priority, force_update_level, update, last_update, update_operation) in
                    callbacks.updates()
                {
                    #[cfg(feature = "physics")]
                    if !done_step
                        && *_priority > physics_priority
                        && world_force_update_level > update_components
                    {
                        done_step = true;
                        Self::world_step(&mut ctx, callbacks);
                    }

                    if *force_update_level < update_components {
                        continue;
                    }

                    match update_operation {
                        crate::UpdateOperation::EveryFrame => {}
                        crate::UpdateOperation::Never => {
                            continue;
                        }
                        crate::UpdateOperation::EveryNFrame(frames) => {
                            if ctx.frame.total_frames() % frames != 0 {
                                continue;
                            }
                        }
                        crate::UpdateOperation::AfterDuration(dur) => {
                            let last_update = last_update.as_ref().unwrap();
                            if now < *last_update.borrow() + *dur {
                                continue;
                            } else {
                                last_update.replace(now);
                            }
                        }
                    }

                    (update)(&mut ctx);
                }
            }

            #[cfg(feature = "physics")]
            if !done_step && world_force_update_level > update_components {
                Self::world_step(&mut ctx, callbacks);
            }
        }
        scene
            .world_camera
            .apply_target(&scene.world, &scene.components);
        self.defaults.buffer(
            &mut scene.world_camera.camera,
            &self.gpu,
            self.frame.total_time(),
            self.frame.frame_time(),
        );

        scene
            .groups
            .update(&mut scene.components, &self.defaults.world_camera);
        if !scene.render_components {
            self.input.update();
            return Ok(());
        }

        scene.components.buffer(&mut scene.world, &self.gpu);

        let ctx = Context::new(self, scene, ContextUse::Render);
        let mut encoder = RenderEncoder::new(&ctx.gpu, &ctx.defaults);

        {
            let mut renderer = ComponentRenderer {
                screenshot: None,
                renderer: encoder.renderer(
                    &ctx.defaults.world_target,
                    ctx.screen_config.clear_color,
                    true,
                ),
            };
            for (_priority, render, target) in callbacks.renders() {
                let (clear, target) = (target)(&ctx);
                if target as *const _ != renderer.target() as *const _ {
                    drop(renderer);
                    renderer = ComponentRenderer {
                        screenshot: None,
                        renderer: encoder.renderer(&ctx.defaults.world_target, clear, true),
                    };
                }
                (render)(&ctx, &mut renderer);
                if let Some(screenshot) = renderer.screenshot.take() {
                    let screenshot =
                        unsafe { (screenshot as *const crate::RenderTarget).as_ref().unwrap() };
                    drop(renderer);
                    encoder.copy_to_target(ctx.defaults.world_target.sprite(), screenshot);
                    renderer = ComponentRenderer {
                        screenshot: None,
                        renderer: encoder.renderer(
                            &ctx.defaults.world_target,
                            ctx.screen_config.clear_color,
                            true,
                        ),
                    };
                }
            }
        }

        #[cfg(feature = "gui")]
        ctx.gui.render(&ctx.gpu, &ctx.defaults, &mut encoder);
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            Renderer::output_renderer(&mut encoder.inner, &output_view, &ctx.defaults);
        }

        encoder.submit(&ctx.gpu);
        drop(ctx);
        self.input.update();
        output.present();
        Ok(())
    }

    pub fn end(&mut self) {
        for (_, scene) in self.scenes.end_scenes() {
            let mut scene = scene.borrow_mut();
            let mut ctx = Context::new(self, &mut scene, ContextUse::Update);
            for (_, end) in ctx.components.controllers.clone().ends() {
                (end)(&mut ctx, EndReason::EndProgram)
            }
        }
    }
}
