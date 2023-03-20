#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::physics::{ActiveEvents, CollideType};
use crate::{
    scene::context::ShuraFields, Context, FrameManager, GlobalState, Gpu, GpuDefaults, Input,
    InstanceIndex, RenderConfig, RenderEncoder, RenderOperation, Renderer, Scene, SceneCreator,
    SceneManager, Vector,
};
use log::{error, info};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

impl Drop for Shura {
    fn drop(&mut self) {
        for (_, scene) in self.scene_manager.end_scenes() {
            let scene = &mut scene.unwrap();
            let end = scene.state.get_end();
            let mut ctx = Context::new(self, scene);
            end(&mut ctx);
        }
    }
}

pub struct Shura {
    pub(crate) end: bool,
    pub(crate) frame_manager: FrameManager,
    pub(crate) scene_manager: SceneManager,
    pub(crate) window: winit::window::Window,
    pub(crate) input: Input,
    pub(crate) gpu: Gpu,
    pub(crate) global_state: Box<dyn GlobalState>,
    pub(crate) defaults: GpuDefaults,
    #[cfg(feature = "gui")]
    pub(crate) gui: Gui,
    #[cfg(feature = "audio")]
    pub(crate) audio: rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub(crate) audio_handle: rodio::OutputStreamHandle,
}

impl Shura {
    /// Start a new game with the given callback to initialize the first [Scene](crate::Scene).
    pub fn init<C: SceneCreator + 'static>(
        #[cfg(target_os = "android")] app: AndroidApp,
        creator: C,
    ) {
        #[cfg(target_os = "android")]
        use winit::platform::android::EventLoopBuilderExtAndroid;
        // #[cfg(target_os = "android")]
        // android_logger::init_once(
        //     android_logger::Config::default().with_min_level(log::Level::Info),
        // );

        info!("Using shura version: {}", env!("CARGO_PKG_VERSION"));
        #[cfg(target_os = "android")]
        let events = winit::event_loop::EventLoopBuilder::new()
            .with_android_app(app)
            .build();
        #[cfg(not(target_os = "android"))]
        let events = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_title("Shura Game")
            .build(&events)
            .unwrap();
        let shura_window_id = window.id();

        #[cfg(target_arch = "wasm32")]
        {
            use console_error_panic_hook::hook;
            use winit::platform::web::WindowExtWebSys;

            std::panic::set_hook(Box::new(hook));
            wasm_logger::init(wasm_logger::Config::default().module_prefix("shura"));

            let canvas = &web_sys::Element::from(window.canvas());
            canvas.set_attribute("tabindex", "0").unwrap();
            canvas
                .set_attribute("oncontextmenu", "return false;")
                .unwrap();
            canvas
                .set_attribute(
                    "style",
                    "
                    margin: auto;
                    position: absolute;
                    top: 0;
                    bottom: 0;
                    left: 0;
                    right: 0;",
                )
                .unwrap();

            let browser_window = web_sys::window().unwrap();
            let document = browser_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(canvas).ok();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use env_logger::Builder;
            use log::LevelFilter;
            Builder::new()
                .filter_level(LevelFilter::Info)
                .filter_module("wgpu", LevelFilter::Warn)
                .filter_module("winit", LevelFilter::Warn)
                .filter_module("symphonia_core", LevelFilter::Warn)
                .init();
        }

        let mut init = Some(creator);
        let mut window = Some(window);
        let mut shura: Option<Shura> = if cfg!(target_os = "android") {
            None
        } else {
            Some(Shura::new(
                window.take().unwrap(),
                &events,
                init.take().unwrap(),
            ))
        };

        events.run(move |event, _target, control_flow| {
            use winit::event::{Event, WindowEvent};
            if let Some(shura) = &mut shura {
                shura.global_state.winit_event(&event);
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
                            let mut scene = shura.scene_manager.borrow_active_scene();
                            if let Some(max_frame_time) = scene.screen_config.max_frame_time() {
                                let now = shura.frame_manager.now();
                                let update_time = shura.frame_manager.update_time();
                                if now < update_time + max_frame_time {
                                    shura.scene_manager.return_active_scene(scene);
                                    return;
                                }
                            }

                            let update_status = shura.update(&mut scene);
                            shura.scene_manager.return_active_scene(scene);
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
                                Err(e) => error!("{:?}", e),
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
                            init.take().unwrap(),
                        ))
                    }
                    _ => {}
                }
            }
        });
    }

    fn new<C: SceneCreator>(
        window: winit::window::Window,
        _event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        creator: C,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(&window));
        let mint: mint::Vector2<u32> = (window.inner_size()).into();
        let window_size: Vector<u32> = mint.into();
        let defaults = GpuDefaults::new(&gpu, window_size);
        #[cfg(feature = "audio")]
        let (audio, audio_handle) = rodio::OutputStream::try_default().unwrap();
        let mut shura = Self {
            scene_manager: SceneManager::new(creator.id()),
            frame_manager: FrameManager::new(),
            input: Input::new(),
            global_state: Box::new(()),
            #[cfg(feature = "audio")]
            audio,
            #[cfg(feature = "audio")]
            audio_handle,
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(_event_loop, &gpu),
            window,
            gpu: gpu,
            defaults,
        };
        let scene = creator.create(ShuraFields::from_shura(&mut shura));
        shura.scene_manager.init(scene);
        return shura;
    }

    fn resize(&mut self, new_size: Vector<u32>) {
        let config_size = self.gpu.render_size_no_scale();
        if new_size.x > 0 && new_size.y > 0 && new_size != config_size {
            let active = self.scene_manager.resize();
            self.gpu.resize(new_size);
            self.defaults
                .resize(&self.gpu, new_size, &active.screen_config);
            #[cfg(feature = "gui")]
            self.gui.resize(&new_size);
        }
    }

    #[cfg(feature = "physics")]
    fn step(ctx: &mut Context) {
        let delta = ctx.frame_time();
        ctx.component_manager.world_mut().step(delta);
        // while let Ok(contact_force_event) = ctx.component_manager.event_receivers.1.try_recv() {
        // }
        while let Ok(collision_event) = ctx.component_manager.collision_event() {
            let collider_handle1 = collision_event.collider1();
            let collider_handle2 = collision_event.collider2();
            let collision_type = if collision_event.started() {
                CollideType::Started
            } else {
                CollideType::Stopped
            };

            if let Some(collider1_events) = ctx
                .collider(collider_handle1)
                .and_then(|c| Some(c.active_events()))
            {
                if let Some(collider2_events) = ctx
                    .collider(collider_handle2)
                    .and_then(|c| Some(c.active_events()))
                {
                    let (component_gype_id1, component1) =
                        ctx.component_from_collider(&collider_handle1).unwrap();
                    let (component_gype_id2, component2) =
                        ctx.component_from_collider(&collider_handle2).unwrap();
                    if collider1_events == ActiveEvents::COLLISION_EVENTS {
                        let callback = ctx
                            .component_manager
                            .component_callbacks(&component_gype_id1)
                            .call_collision;
                        (callback)(
                            ctx,
                            component1,
                            component2,
                            collider_handle1,
                            collider_handle2,
                            collision_type,
                        )
                    }
                    if collider2_events == ActiveEvents::COLLISION_EVENTS {
                        let callback = ctx
                            .component_manager
                            .component_callbacks(&component_gype_id2)
                            .call_collision;
                        (callback)(
                            ctx,
                            component2,
                            component1,
                            collider_handle2,
                            collider_handle1,
                            collision_type,
                        )
                    }
                }
            }
        }
    }

    fn update(&mut self, scene: &mut Scene) -> Result<(), wgpu::SurfaceError> {
        #[cfg(target_arch = "wasm32")]
        {
            let browser_window = web_sys::window().unwrap();
            let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
            let size = winit::dpi::PhysicalSize::new(width, height);
            if size != self.window.inner_size().into() {
                self.window.set_inner_size(size);
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
        let mint: mint::Vector2<u32> = self.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        self.frame_manager.update();
        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame_manager.total_time_duration(), &self.window);

        if scene.resized {
            scene
                .world_camera
                .resize(window_size.x as f32 / window_size.y as f32);
        }

        {
            let state_update = scene.state.get_update();
            let mut ctx = Context::new(self, scene);
            #[cfg(feature = "physics")]
            let (mut done_step, physics_priority) = {
                if let Some(physics_priority) = ctx.physics_priority() {
                    (false, physics_priority)
                } else {
                    (true, 0)
                }
            };
            let now = ctx.update_time();
            {
                let sets = ctx.component_manager.copy_active_components();
                state_update(&mut ctx);
                for set in sets.values() {
                    if set.paths().len() == 0 {
                        continue;
                    }

                    let config = set.config();
                    #[cfg(feature = "physics")]
                    if !done_step && config.priority > physics_priority {
                        done_step = true;
                        Self::step(&mut ctx);
                    }

                    match config.update {
                        crate::UpdateOperation::EveryFrame => {}
                        crate::UpdateOperation::Never => {
                            continue;
                        }
                        crate::UpdateOperation::EveryNFrame(frames) => {
                            if ctx.total_frames() % frames != 0 {
                                continue;
                            }
                        }
                        crate::UpdateOperation::AfterDuration(dur) => {
                            if now < set.last_update().unwrap() + dur {
                                continue;
                            }
                        }
                    }

                    (set.callbacks().call_update)(set.paths(), &mut ctx);
                }
            }

            #[cfg(feature = "physics")]
            if !done_step && ctx.physics_priority().is_some() {
                Self::step(&mut ctx);
            }
        }

        self.input.update();
        self.defaults
            .apply_render_scale(&self.gpu, scene.screen_config.render_scale());
        scene.world_camera.apply_target(&scene.component_manager);
        scene.component_manager.update_sets(&scene.world_camera);

        if !scene.component_manager.render_components() {
            return Ok(());
        }

        if scene.switched || scene.resized || scene.screen_config.vsync_changed {
            scene.screen_config.vsync_changed = false;
            self.gpu.apply_vsync(scene.screen_config.vsync());
        }

        scene.component_manager.buffer_sets(&self.gpu);
        self.defaults.buffer(
            &scene.world_camera,
            &self.gpu,
            self.frame_manager.total_time(),
            self.frame_manager.frame_time(),
        );

        let mut encoder = RenderEncoder::new(&self.gpu);
        if let Some(clear_color) = scene.screen_config.clear_color {
            encoder.clear_target(&self.defaults.target, clear_color);
        }

        {
            let ctx = Context::new(self, scene);
            for set in ctx.component_manager.copy_active_components().values() {
                if set.is_empty() {
                    continue;
                }
                let config = set.config();
                if config.render != RenderOperation::Never {
                    match config.render {
                        RenderOperation::EveryFrame => {
                            for path in set.paths() {
                                let group = ctx.component_manager.group(path.group_index).unwrap();
                                let component_type = group.type_ref(path.type_index).unwrap();
                                let buffer = component_type
                                    .buffer()
                                    .unwrap_or(&ctx.defaults.empty_instance);
                                let config = RenderConfig {
                                    camera: &ctx.defaults.world_camera,
                                    instances: buffer,
                                    target: &ctx.defaults.target,
                                    gpu: &ctx.gpu,
                                    defaults: &ctx.defaults,
                                    msaa: true,
                                };
                                if component_type.len() > 0 {
                                    (set.callbacks().call_render)(
                                        &[*path],
                                        &ctx,
                                        config,
                                        &mut encoder,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let output = self.gpu.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut renderer = Renderer::output_renderer(
                &mut encoder.inner,
                &self.gpu,
                &self.defaults,
                &output_view,
            );
            renderer.render_sprite_no_msaa(
                InstanceIndex::new(0),
                self.defaults.relative_camera.buffer().model(),
                self.defaults.target.sprite(),
            );
        }

        #[cfg(feature = "gui")]
        {
            self.gui.render(&self.gpu, &mut encoder.inner, &output_view);
        }

        encoder.submit(&self.gpu);
        output.present();

        scene.resized = false;
        scene.switched = false;
        Ok(())
    }
}