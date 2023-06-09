use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::physics::{ActiveEvents, CollideType};
use crate::{
    audio::AudioManager, Context, FrameManager, GlobalStateManager, Gpu, GpuConfig, GpuDefaults,
    Input, RenderConfigTarget, RenderEncoder, RenderOperation, Renderer, Scene, SceneCreator,
    SceneManager, Vector,
};
#[cfg(target_arch = "wasm32")]
use rustc_hash::FxHashMap;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(feature = "log")]
use crate::{
    log::{error, info, LoggerBuilder},
    VERSION,
};

/// Configuration for the base of the game engine
pub struct ShuraConfig {
    pub window: winit::window::WindowBuilder,
    // pub global_states: Vec<Box<dyn GlobalStateController>>,
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
            use winit::event::{Event, WindowEvent};
            if let Some(shura) = &mut shura {
                for global in shura.states.iter_mut() {
                    global.winit_event(&event);
                }
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
                            while let Some(remove) = shura.scenes.remove.pop() {
                                if let Some(removed) = shura.scenes.scenes.remove(&remove) {
                                    let mut removed = removed.borrow_mut();
                                    for end in removed.states.ends() {
                                        let mut ctx = Context::new(shura, &mut removed);
                                        end(&mut ctx);
                                    }
                                }
                            }

                            while let Some(add) = shura.scenes.add.pop() {
                                let id = add.new_id();
                                let scene = add.create(shura);
                                shura.scenes.scenes.insert(id, Rc::new(RefCell::new(scene)));
                            }

                            let scene = shura.scenes.get_active_scene();
                            let mut scene = scene.borrow_mut();
                            if let Some(max_frame_time) = scene.screen_config.max_frame_time() {
                                let now = shura.frame.now();
                                let update_time = shura.frame.update_time();
                                if now < update_time + max_frame_time {
                                    return;
                                }
                            }

                            let update_status = shura.update(&mut scene);
                            match update_status {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => {
                                    let mint: mint::Vector2<u32> = shura.window.inner_size().into();
                                    let window_size: Vector<u32> = mint.into();
                                    shura.resize(window_size);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    *control_flow = winit::event_loop::ControlFlow::Exit
                                }
                                Err(_e) => {
                                    #[cfg(feature = "log")]
                                    error!("Render Error: {:?}", _e)
                                }
                            }

                            if shura.end {
                                *control_flow = winit::event_loop::ControlFlow::Exit;
                            }
                        }
                        Event::MainEventsCleared => {
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

pub struct Shura {
    pub end: bool,
    pub frame: FrameManager,
    pub scenes: SceneManager,
    pub window: winit::window::Window,
    pub input: Input,
    pub gpu: Gpu,
    pub states: GlobalStateManager,
    pub defaults: GpuDefaults,
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
            input: Input::new(window_size),
            states: GlobalStateManager::default(),
            #[cfg(feature = "audio")]
            audio: AudioManager::new(),
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(_event_loop, &gpu),
            window,
            gpu: gpu,
            defaults,
            #[cfg(target_arch = "wasm32")]
            auto_scale_canvas,
        }
    }

    fn resize(&mut self, new_size: Vector<u32>) {
        let config_size = self.gpu.render_size_no_scale();
        if new_size.x > 0 && new_size.y > 0 && new_size != config_size {
            self.scenes.resize();
            self.input.resize(new_size);
            self.gpu.resize(new_size);
            self.defaults.resize(&self.gpu, new_size);
            #[cfg(feature = "gui")]
            self.gui.resize(&new_size);
        }
    }

    #[cfg(feature = "physics")]
    fn physics_step(ctx: &mut Context) {
        macro_rules! skip_fail {
            ($res:expr) => {
                match $res {
                    Some(val) => val,
                    None => {
                        info!("ululul");
                        continue
                    },
                }
            };
        }

        let delta = ctx.frame.frame_time();
        ctx.components.apply_world_mapping(ctx.world);
        ctx.world.step(delta);
        // while let Ok(contact_force_event) = ctx.components.event_receivers.1.try_recv() {
        // }
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

            let callable1 = ctx.components.callable(&component1.type_index());
            let callable2 = ctx.components.callable(&component2.type_index());
            let callback1 = callable1.callbacks.collision;
            let callback2 = callable2.callbacks.collision;

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

    fn update(&mut self, scene: &mut Scene) -> Result<(), wgpu::SurfaceError> {
        self.frame.update();
        #[cfg(feature = "gamepad")]
        self.input.sync_controller();
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
        if scene.switched || scene.resized || scene.screen_config.changed {
            scene.screen_config.changed = false;
            scene.world_camera.resize(window_size);

            self.gpu.apply_vsync(scene.screen_config.vsync());
            self.defaults
                .apply_render_scale(&self.gpu, scene.screen_config.render_scale());
        }
        if scene.started {
            self.input.update();
            scene.world_camera.apply_target(
                #[cfg(feature = "physics")]
                &scene.world,
                &scene.components,
            );
            self.defaults.buffer(
                &mut scene.world_camera.camera,
                &self.gpu,
                self.frame.total_time(),
                self.frame.frame_time(),
            );
            scene.components.update_sets(&self.defaults.world_camera);
            scene.components.buffer(
                #[cfg(feature = "physics")]
                &mut scene.world,
                &self.gpu,
            );
        }
        let output = self.gpu.surface.get_current_texture()?;

        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame.total_time_duration(), &self.window);

        {
            let mut ctx = Context::new(self, scene);
            #[cfg(feature = "physics")]
            let (mut done_step, physics_priority) = {
                if let Some(physics_priority) = ctx.world.physics_priority() {
                    (false, physics_priority)
                } else {
                    (true, 0)
                }
            };
            let mut prev_priority = i16::MIN;
            let now = ctx.frame.update_time();
            {
                // let types = ctx.components.callable_types();
                // let mut types = types.borrow_mut();
                for ((priority, _), type_index) in ctx.components.priorities().borrow().iter() {
                    for update in ctx.scene_states.updates(prev_priority, *priority) {
                        update(&mut ctx);
                    }

                    #[cfg(feature = "physics")]
                    if !done_step && *priority > physics_priority {
                        done_step = true;
                        Self::physics_step(&mut ctx);
                    }

                    let ty = ctx.components.callable_mut(type_index);
                    match ty.config.update {
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
                            if now < ty.last_update.unwrap() + dur {
                                continue;
                            } else {
                                ty.last_update = Some(now);
                            }
                        }
                    }

                    (ty.callbacks.update)(&mut ctx);
                    prev_priority = *priority;
                }
            }

            #[cfg(feature = "physics")]
            if !done_step && ctx.world.physics_priority().is_some() {
                Self::physics_step(&mut ctx);
            }
            for update in ctx.scene_states.updates(prev_priority, i16::MAX) {
                update(&mut ctx);
            }
        }
        self.input.update();
        self.defaults
            .apply_render_scale(&self.gpu, scene.screen_config.render_scale());
        scene.world_camera.apply_target(
            #[cfg(feature = "physics")]
            &scene.world,
            &scene.components,
        );
        self.defaults.buffer(
            &mut scene.world_camera.camera,
            &self.gpu,
            self.frame.total_time(),
            self.frame.frame_time(),
        );

        scene.components.update_sets(&self.defaults.world_camera);
        if !scene.render_components {
            scene.resized = false;
            scene.switched = false;
            scene.started = false;
            return Ok(());
        }

        scene.components.buffer(
            #[cfg(feature = "physics")]
            &mut scene.world,
            &self.gpu,
        );
        let ctx = Context::new(self, scene);
        let mut encoder = RenderEncoder::new(ctx.gpu, &ctx.defaults);
        if let Some(clear_color) = ctx.screen_config.clear_color {
            encoder.clear(RenderConfigTarget::World, clear_color);
        }

        {
            let mut prev_priority = i16::MIN;
            for ((priority, _), type_index) in ctx.components.priorities().borrow().iter() {
                for render in ctx.scene_states.renders(prev_priority, *priority) {
                    render(&ctx, &mut encoder);
                }
                let ty = ctx.components.callable(type_index);
                if ty.config.render != RenderOperation::Never {
                    match ty.config.render {
                        RenderOperation::EveryFrame => {
                            (ty.callbacks.render)(&ctx, &mut encoder);
                        }
                        _ => {}
                    }
                }
                prev_priority = *priority;
            }
        }
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut renderer =
                Renderer::output_renderer(&mut encoder.inner, &output_view, ctx.defaults, ctx.gpu);
            renderer.use_camera(&ctx.defaults.relative_camera.0);
            renderer.use_instances(&ctx.defaults.single_centered_instance);
            renderer.use_shader(&ctx.defaults.sprite_no_msaa);
            renderer.use_model(ctx.defaults.relative_camera.0.model());
            renderer.use_sprite(ctx.defaults.world_target.sprite(), 1);
            renderer.draw(0);
        }

        #[cfg(feature = "gui")]
        ctx.gui.render(&ctx.gpu, &mut encoder.inner, &output_view);

        encoder.finish();
        ctx.gpu.submit_encoders();
        output.present();

        scene.resized = false;
        scene.switched = false;
        scene.started = false;
        Ok(())
    }
}

impl Drop for Shura {
    fn drop(&mut self) {
        for (_, scene) in self.scenes.end_scenes() {
            let mut scene = scene.borrow_mut();
            for end in scene.states.ends() {
                let mut ctx = Context::new(self, &mut scene);
                end(&mut ctx);
            }
        }
    }
}
