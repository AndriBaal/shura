use std::any::Any;

#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::physics::{ActiveEvents, CollideType};
use crate::{
    Context, FrameManager, Gpu, GpuDefaults, Input, InstanceIndex, RenderConfig, RenderEncoder,
    RenderOperation, Renderer, Scene, SceneCreator, SceneManager, Vector,
};
use log::{error, info};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

pub struct Shura {
    pub end: bool,
    pub frame_manager: FrameManager,
    pub scene_manager: SceneManager,
    pub window: winit::window::Window,
    pub input: Input,
    pub gpu: Gpu,
    pub global_state: Box<dyn Any>,
    pub defaults: GpuDefaults,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: rodio::OutputStreamHandle,
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
                                    shura.end(control_flow);
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
                                _ => shura.input.event(event),
                            }
                        }
                    }
                    Event::RedrawRequested(window_id)
                        if window_id == shura_window_id && !shura.end =>
                    {
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
                            shura.end(control_flow);
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
        let scene = creator.create(&mut shura);
        shura.scene_manager.init(scene);
        return shura;
    }

    fn end(&mut self, cf: &mut winit::event_loop::ControlFlow) {
        for (_, scene) in self.scene_manager.end_scenes() {
            let mut ctx = Context {
                scene: &mut scene.unwrap(),
                shura: self,
            };

            for (_, type_id) in ctx.scene.component_manager.end_callbacks() {
                let paths = ctx.scene.component_manager.all_paths(type_id);
                if paths.len() > 0 {
                    let callbacks = ctx.scene.component_manager.component_callbacks(&type_id);
                    (callbacks.call_end)(&paths, &mut ctx);
                }
            }
        }
        self.end = true;
        *cf = winit::event_loop::ControlFlow::Exit;
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
        ctx.scene.component_manager.world_mut().step(delta);
        // while let Ok(contact_force_event) = ctx.scene.component_manager.event_receivers.1.try_recv() {
        // }
        while let Ok(collision_event) = ctx.scene.component_manager.collision_event() {
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
                            .scene
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
                            .scene
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
        let mut ctx = Context {
            shura: self,
            scene: scene,
        };

        #[cfg(target_arch = "wasm32")]
        {
            use crate::log::warn;
            const MAX_WEBGL_TEXTURE_SIZE: u32 = 2048;
            let browser_window = web_sys::window().unwrap();
            let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
            let size = winit::dpi::PhysicalSize::new(width, height);
            if size.width > MAX_WEBGL_TEXTURE_SIZE || size.height > MAX_WEBGL_TEXTURE_SIZE {
                let max = size.width.max(size.height);
                let scale = MAX_WEBGL_TEXTURE_SIZE as f32 / max as f32;
                warn!("Auto scaling down to {} because the maximum WebGL texturesize has been surpassed!", scale);
                ctx.set_render_scale(scale);
            }
            if size != ctx.shura.window.inner_size().into() {
                ctx.shura.window.set_inner_size(size);
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
        let window_size = ctx.window_size();
        ctx.shura.frame_manager.update();
        #[cfg(feature = "gui")]
        ctx.shura.gui.begin(
            &ctx.shura.frame_manager.total_time_duration(),
            &ctx.shura.window,
        );

        if ctx.scene.resized {
            ctx.scene
                .world_camera
                .resize(window_size.x as f32 / window_size.y as f32);
        }

        if ctx.scene.switched {
            ctx.shura
                .defaults
                .apply_render_scale(&ctx.shura.gpu, ctx.scene.screen_config.render_scale());
        }

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
            let sets = ctx.scene.component_manager.copy_active_components();
            for set in sets.values() {
                if set.paths().len() < 1 {
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

        ctx.shura.input.update();
        ctx.scene
            .world_camera
            .apply_target(&ctx.scene.component_manager);
        ctx.scene
            .component_manager
            .update_sets(&ctx.scene.world_camera);
        ctx.scene.resized = false;
        ctx.scene.switched = false;

        if !ctx.render_components() {
            return Ok(());
        }

        ctx.scene.component_manager.buffer_sets(&ctx.shura.gpu);
        ctx.shura.defaults.buffer(
            &ctx.scene.world_camera,
            &ctx.shura.gpu,
            ctx.shura.frame_manager.total_time(),
            ctx.shura.frame_manager.frame_time(),
        );

        let mut encoder = RenderEncoder::new(&ctx.shura.gpu);
        if let Some(clear_color) = ctx.clear_color() {
            encoder.clear(&ctx.shura.defaults.target, clear_color);
        }

        {
            for set in ctx
                .scene
                .component_manager
                .copy_active_components()
                .values()
            {
                if set.is_empty() {
                    continue;
                }
                let config = set.config();
                if config.render != RenderOperation::Never {
                    match config.render {
                        RenderOperation::EveryFrame => {
                            for path in set.paths() {
                                let group =
                                    ctx.scene.component_manager.group(path.group_index).unwrap();
                                let component_type = group.type_ref(path.type_index).unwrap();
                                let buffer = component_type
                                    .buffer()
                                    .unwrap_or(&ctx.shura.defaults.empty_instance);
                                let config = RenderConfig {
                                    camera: &ctx.shura.defaults.world_camera,
                                    instances: buffer,
                                    target: &ctx.shura.defaults.target,
                                    gpu: &ctx.shura.gpu,
                                    defaults: &ctx.shura.defaults,
                                    smaa: true,
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
        let output = ctx.shura.gpu.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut renderer = Renderer::output_renderer(
                &mut encoder.encoder,
                &ctx.shura.gpu,
                &ctx.shura.defaults,
                &output_view,
            );
            renderer.render_sprite(
                ctx.shura.defaults.relative_camera.buffer().model(),
                ctx.shura.defaults.target.sprite(),
            );
            renderer.commit(InstanceIndex { index: 0 });
        }

        #[cfg(feature = "gui")]
        {
            ctx.shura
                .gui
                .render(&ctx.shura.gpu, &mut encoder.encoder, &output_view);
        }

        let encoder = encoder.encoder;
        ctx.shura
            .gpu
            .queue
            .submit(std::iter::once(encoder.finish()));
        // encoder.submit();
        output.present();

        Ok(())
    }
}
