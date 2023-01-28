#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::{
    physics::{ActiveEvents, CollideType, ColliderHandle, PhysicsComponent},
    ArenaPath, ComponentHandle, DynamicScene,
};
use crate::{
    BoxedScene, Camera, Color, ComponentSet, Context, Defaults, Dimension, FrameManager, Gpu,
    Input, PostproccessOperation, RenderOperation, Renderer, Scene, SceneController, SceneCreator,
    SceneManager, Sprite,
};
use log::{error, info};

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;
pub(crate) const RELATIVE_CAMERA_SIZE: f32 = 1.0;

pub struct Shura {
    pub(crate) relative_camera: Camera,
    pub(crate) end: bool,
    pub frame_manager: FrameManager,
    pub scene_manager: SceneManager,
    pub window: winit::window::Window,
    pub input: Input,
    pub gpu: Gpu,
    pub defaults: Defaults,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: rodio::OutputStreamHandle,
}

impl Shura {
    /// Start a new game with the given callback to initialize the first [SceneController].
    pub fn init(creator: impl SceneCreator + 'static) {
        info!("Using shura version: {}", env!("CARGO_PKG_VERSION"));
        let events = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_title(creator.name())
            .build(&events)
            .unwrap();
        let shura_window_id = window.id();
        let mut creator = Some(creator);
        let mut window = Some(window);

        #[cfg(target_arch = "wasm32")]
        {
            use console_error_panic_hook::hook;
            use winit::platform::web::WindowExtWebSys;

            std::panic::set_hook(Box::new(hook));
            wasm_logger::init(wasm_logger::Config::default().module_prefix("shura"));

            let canvas = &web_sys::Element::from(shura.window.canvas());
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
            let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;

            let document = browser_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(canvas).ok();

            shura.window.set_inner_size(Dimension::new(width, height));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use env_logger::Builder;
            use log::LevelFilter;
            Builder::new()
                .filter_level(LevelFilter::Info)
                .filter_module("wgpu", LevelFilter::Error)
                .filter_module("winit", LevelFilter::Warn)
                .filter_module("symphonia_core", LevelFilter::Warn)
                .init();
        }

        let mut active: Option<(Shura, BoxedScene)> = if cfg!(target_os = "android") {
            None
        } else {
            Some(Shura::new(
                window.take().unwrap(),
                &events,
                creator.take().unwrap(),
            ))
        };

        events.run(move |event, target, control_flow| {
            use winit::event::{Event, WindowEvent};
            if let Some((shura, scene)) = &mut active {
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
                                    shura.end(scene, control_flow);
                                }
                                WindowEvent::Resized(physical_size) => {
                                    shura.resize(
                                        scene,
                                        (*physical_size as winit::dpi::PhysicalSize<u32>).into(),
                                    );
                                }
                                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                    shura.resize(
                                        scene,
                                        (**new_inner_size as winit::dpi::PhysicalSize<u32>).into(),
                                    );
                                }
                                _ => shura.input.event(event),
                            }
                        }
                    }
                    Event::RedrawRequested(window_id) if window_id == shura_window_id => {
                        if let Some(new_active) = shura.scene_manager.apply_active_scene() {
                            let old_active = std::mem::replace(scene, new_active);
                            shura.scene_manager.add(old_active);
                        }

                        match shura.update(scene) {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                shura.resize(scene, shura.window.inner_size().into())
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                *control_flow = winit::event_loop::ControlFlow::Exit
                            }
                            Err(e) => error!("{:?}", e),
                        }
                        if shura.end {
                            shura.end(scene, control_flow);
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
                        active = Some(Shura::new(
                            window.take().unwrap(),
                            &target,
                            creator.take().unwrap(),
                        ))
                    }
                    _ => {}
                }
            }
        });
    }

    fn new(
        window: winit::window::Window,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        creator: impl SceneCreator,
    ) -> (Self, BoxedScene) {
        let gpu = pollster::block_on(Gpu::new(&window));
        let defaults = Defaults::new(&gpu);
        #[cfg(feature = "audio")]
        let (audio, audio_handle) = rodio::OutputStream::try_default().unwrap();
        let relative_camera = Camera::new(&gpu, Default::default(), 1.0, RELATIVE_CAMERA_SIZE);
        let mut shura = Self {
            scene_manager: SceneManager::new(creator.name()),
            frame_manager: FrameManager::new(),
            input: Input::new(),
            #[cfg(feature = "audio")]
            audio,
            #[cfg(feature = "audio")]
            audio_handle,
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(&window, event_loop, &gpu),
            window,
            gpu: gpu,
            relative_camera,
            defaults,
        };
        let scene = Scene::new(&mut shura, creator);
        return (shura, scene);
    }

    fn end(&mut self, main_scene: &mut BoxedScene, cf: &mut winit::event_loop::ControlFlow) {
        let mut ctx = Context::new(&mut main_scene.1, self);
        main_scene.0.end(&mut ctx);
        drop(ctx);
        for (_, mut scene) in self.scene_manager.end_scenes() {
            let mut ctx = Context::new(&mut scene.1, self);
            scene.0.end(&mut ctx);
        }
        *cf = winit::event_loop::ControlFlow::Exit
    }

    fn resize(&mut self, main_scene: &mut BoxedScene, new_size: Dimension<u32>) {
        let config_size = self.gpu.render_size_no_scale();
        if new_size.width > 0 && new_size.height > 0 && new_size != config_size {
            self.scene_manager.resize(main_scene);
            self.gpu.resize(new_size);
            self.defaults.resize(&self.gpu);
            #[cfg(feature = "gui")]
            self.gui.resize(&self.window, &new_size);
        }
    }

    #[cfg(feature = "physics")]
    fn step(controller: &mut DynamicScene, ctx: &mut Context) {
        ctx.step_world();
        // while let Ok(contact_force_event) = ctx.scene.world.event_receivers.1.try_recv() {
        // }
        while let Ok(collision_event) = ctx.collision_event() {
            let collider_handle1 = collision_event.collider1();
            let collider_handle2 = collision_event.collider2();
            let collision_type = if collision_event.started() {
                CollideType::Started
            } else {
                CollideType::Stopped
            };

            if let Some(collider1) = ctx.collider(collider_handle1) {
                if let Some(collider2) = ctx.collider(collider_handle2) {
                    fn call_collide(
                        scene: &mut DynamicScene,
                        ctx: &mut Context,
                        self_handle: ComponentHandle,
                        other_handle: ComponentHandle,
                        self_collider: ColliderHandle,
                        other_collider: ColliderHandle,
                        collide_type: CollideType,
                    ) {
                        let path = ArenaPath {
                            group_index: self_handle.group_index(),
                            type_index: self_handle.type_index(),
                        };
                        let i = self_handle.component_index().index() as usize;
                        if let Some(mut entry) = ctx.borrow_component(path, i) {
                            match &mut entry {
                                crate::data::arena::ArenaEntry::Occupied { data, .. } => {
                                    ctx.set_current_component(Some(self_handle));
                                    data.collision(
                                        scene,
                                        ctx,
                                        other_handle,
                                        self_collider,
                                        other_collider,
                                        collide_type,
                                    );
                                }
                                _ => {}
                            };

                            if ctx.remove_current_commponent() {
                                ctx.not_return_component(path, i);
                            } else {
                                ctx.return_component(path, i, entry);
                            }
                        }
                    }
                    let component1 = ctx.component_from_collider(&collider_handle1).unwrap();
                    let component2 = ctx.component_from_collider(&collider_handle2).unwrap();
                    let collider1_events = collider1.active_events();
                    let collider2_events = collider2.active_events();
                    if collider1_events == ActiveEvents::COLLISION_EVENTS {
                        call_collide(
                            controller,
                            ctx,
                            component1,
                            component2,
                            collider_handle1,
                            collider_handle2,
                            collision_type,
                        );
                    }
                    if collider2_events == ActiveEvents::COLLISION_EVENTS {
                        call_collide(
                            controller,
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

    fn update(&mut self, scene: &mut BoxedScene) -> Result<(), wgpu::SurfaceError> {
        self.frame_manager.update();
        let (scene_controller, scene) = (&mut scene.0, &mut scene.1);
        #[cfg(target_arch = "wasm32")]
        {
            let browser_window = web_sys::window().unwrap();
            let width: u32 = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height: u32 = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
            let size = Dimension::new(width, height);
            if size != self.window.inner_size().into() {
                self.window.set_inner_size(size);
            }
        }

        #[cfg(feature = "gui")]
        self.gui
            .begin(&self.frame_manager.total_time_duration(), &self.window);

        {
            let mut ctx = Context::new(scene, self);
            scene_controller.update(&mut ctx);

            #[cfg(feature = "physics")]
            let mut done_step = false;
            let total_frames = ctx.total_frames();
            let now = ctx.update_time();

            if ctx.update_components() {
                let mut sets = ctx.borrow_active_components();
                for set in sets.values_mut() {
                    let config = set.config();

                    #[cfg(feature = "physics")]
                    if !done_step && config.priority > ctx.physics_priority() {
                        done_step = true;
                        Self::step(scene_controller, &mut ctx);
                    }

                    match config.update {
                        crate::UpdateOperation::EveryFrame => {}
                        crate::UpdateOperation::None => {
                            continue;
                        }
                        crate::UpdateOperation::EveryNFrame(frames) => {
                            if ctx.total_frames() % frames != 0 {
                                continue;
                            }
                        }
                        crate::UpdateOperation::AfterDuration(dur) => {
                            if now > set.last_update().unwrap() + dur {
                                set.set_last_update(now);
                            } else {
                                continue;
                            }
                        }
                    }

                    'outer: for path in set.paths() {
                        let mut i = 0;
                        loop {
                            if let Some(mut entry) = ctx.borrow_component(*path, i) {
                                match &mut entry {
                                    crate::data::arena::ArenaEntry::Occupied { data, .. } => {
                                        if data.inner().handle().start() != total_frames {
                                            ctx.set_current_component(Some(*data.inner().handle()));
                                            data.update(scene_controller, &mut ctx);
                                        }
                                    }
                                    _ => (),
                                };

                                if ctx.remove_current_commponent() {
                                    #[cfg(feature = "physics")]
                                    match &mut entry {
                                        crate::data::arena::ArenaEntry::Occupied {
                                            data, ..
                                        } => {
                                            if let Some(p) =
                                                data.inner_mut().downcast_mut::<PhysicsComponent>()
                                            {
                                                p.remove_from_world(&mut ctx.scene.world);
                                            }
                                        }
                                        _ => (),
                                    };
                                    ctx.not_return_component(*path, i);
                                } else {
                                    ctx.return_component(*path, i, entry);
                                }
                                i += 1;
                            } else {
                                continue 'outer;
                            }
                        }
                    }
                }
                ctx.return_active_components(sets);
                ctx.set_current_component(None);
            }

            #[cfg(feature = "physics")]
            if !done_step {
                Self::step(scene_controller, &mut ctx);
                ctx.set_current_component(None);
            }

            scene_controller.after_update(&mut ctx);
        };

        if !scene.component_manager.render_components() {
            return Ok(());
        }

        scene.camera.buffer(&self.gpu);
        scene.component_manager.buffer_sets(
            &self.gpu,
            #[cfg(feature = "physics")]
            &scene.world,
        );
        self.defaults.buffer(
            &self.gpu,
            self.frame_manager.total_time(),
            self.frame_manager.delta_time(),
        );

        let output = self.gpu.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.gpu.encoder();

        // Clear the texture
        if let Some(clear_color) = &scene.clear_color {
            Renderer::clear(
                &mut encoder,
                &self.defaults.target_view,
                &self.defaults.target_msaa,
                *clear_color,
            );
        }
        for set in scene.component_manager.active_components().values() {
            if set.is_empty() {
                continue;
            }
            let config = set.config();
            let mut save_sprite: Option<String> = None;
            if config.render != RenderOperation::None {
                let mut renderer = if config.postproccess == PostproccessOperation::SeperateLayer {
                    Renderer::clear(
                        &mut encoder,
                        &self.defaults.layer_view,
                        &self.defaults.layer_msaa,
                        Color::TRANSPARENT,
                    );
                    Renderer::new(
                        &self.gpu,
                        &self.defaults,
                        &mut encoder,
                        &self.defaults.layer_view,
                        &self.defaults.layer_msaa,
                    )
                } else {
                    Renderer::new(
                        &self.gpu,
                        &self.defaults,
                        &mut encoder,
                        &self.defaults.target_view,
                        &self.defaults.target_msaa,
                    )
                };
                match &config.camera {
                    crate::CameraUse::World => {
                        renderer.enable_camera(&scene.camera);
                    }
                    crate::CameraUse::Relative => {
                        renderer.enable_camera(&self.relative_camera);
                    }
                }
                match config.render {
                    RenderOperation::Solo => {
                        for path in set.paths() {
                            let group = scene.component_manager.group(path.group_index).unwrap();
                            let component_type = group.type_ref(path.type_index).unwrap();
                            let buffer = component_type.buffer();
                            renderer.set_instance_buffer(buffer);
                            for (instance, (_, component)) in component_type.iter().enumerate() {
                                let instance = instance as u32;
                                component.render(
                                    scene_controller,
                                    &mut renderer,
                                    instance..instance + 1,
                                );
                            }
                        }
                        save_sprite = renderer.save_sprite.take();
                    }
                    RenderOperation::Grouped => {
                        for path in set.paths() {
                            let group = scene.component_manager.group(path.group_index).unwrap();
                            let component_type = group.type_ref(path.type_index).unwrap();
                            let len = component_type.len();
                            let buffer = component_type.buffer();
                            renderer.set_instance_buffer(buffer);
                            if let Some((_, first_component)) = component_type.iter().next() {
                                let grouped_render = first_component.get_grouped_render();
                                let instances = 0..len as u32;

                                let set = ComponentSet::new(vec![component_type], len);
                                grouped_render(scene_controller, &mut renderer, set, instances);
                            }
                        }
                        save_sprite = renderer.save_sprite.take();
                    }
                    _ => {}
                }
            }

            if let Some(sprite_name) = save_sprite.take() {
                let mut sprite = Sprite::empty(&self.gpu, self.gpu.render_size());
                sprite.write_current_render(
                    &self.gpu,
                    &self.defaults,
                    &mut encoder,
                    &self.relative_camera,
                );
                scene.saved_sprites.push((sprite_name, sprite));
            }

            if config.postproccess != PostproccessOperation::None {
                'outer: for path in set.paths() {
                    let group = scene.component_manager.group(path.group_index).unwrap();
                    let component_type = group.type_ref(path.type_index).unwrap();
                    for (_, component) in component_type.iter() {
                        let postproccess = component.get_postproccess();
                        let instances = 0..1;
                        match config.postproccess {
                            PostproccessOperation::SameLayer => {
                                let mut copy =
                                    Sprite::empty(&self.gpu, *self.defaults.target.size());
                                copy.write_current_render(
                                    &self.gpu,
                                    &self.defaults,
                                    &mut encoder,
                                    &self.relative_camera,
                                );
                                let mut renderer = Renderer::new(
                                    &self.gpu,
                                    &self.defaults,
                                    &mut encoder,
                                    &self.defaults.target_view,
                                    &self.defaults.target_msaa,
                                );
                                renderer.use_uniform(self.relative_camera.uniform(), 0);
                                renderer
                                    .set_instance_buffer(&self.defaults.single_centered_instance);
                                postproccess(
                                    &mut renderer,
                                    instances,
                                    self.relative_camera.model(),
                                    &copy,
                                );
                            }
                            PostproccessOperation::SeperateLayer => {
                                let mut renderer = Renderer::new(
                                    &self.gpu,
                                    &self.defaults,
                                    &mut encoder,
                                    &self.defaults.target_view,
                                    &self.defaults.target_msaa,
                                );
                                renderer.use_uniform(self.relative_camera.uniform(), 0);
                                renderer
                                    .set_instance_buffer(&self.defaults.single_centered_instance);
                                postproccess(
                                    &mut renderer,
                                    instances,
                                    self.relative_camera.model(),
                                    &self.defaults.layer,
                                );
                            }
                            PostproccessOperation::None => {}
                        }
                        break 'outer;
                    }
                }

                if let Some(sprite_name) = save_sprite.take() {
                    let mut sprite = Sprite::empty(&self.gpu, self.gpu.render_size());
                    sprite.write_current_render(
                        &self.gpu,
                        &self.defaults,
                        &mut encoder,
                        &self.relative_camera,
                    );
                    scene.saved_sprites.push((sprite_name, sprite));
                }
            }
        }

        {
            let mut renderer = Renderer::new(
                &self.gpu,
                &self.defaults,
                &mut encoder,
                &output_view,
                &self.defaults.present_msaa,
            );
            renderer.use_uniform(self.relative_camera.uniform(), 0);
            renderer.set_instance_buffer(&self.defaults.single_centered_instance);
            renderer.render_sprite(self.relative_camera.model(), &self.defaults.target);
            renderer.commit(&(0..1));
        }
        #[cfg(feature = "gui")]
        {
            self.gui.render(&self.gpu, &mut encoder, &output_view);
        }
        self.gpu.finish_enocder(encoder);
        output.present();

        Ok(())
    }
}
