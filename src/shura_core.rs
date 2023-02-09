#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::{
    physics::{ActiveEvents, CollideType, ColliderHandle, PhysicsComponent},
    ArenaPath, ComponentHandle,
};
use crate::{
    Color, Context, Defaults, Dimension, FrameManager, Gpu, Input, PostproccessOperation,
    RenderOperation, Renderer, Scene, SceneCreator, SceneManager, Sprite,
};
use log::{error, info};

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

pub struct Shura {
    pub(crate) end: bool,
    pub frame_manager: FrameManager,
    pub scene_manager: SceneManager,
    pub window: winit::window::Window,
    pub input: Input,
    pub gpu: Gpu,
    pub(crate) defaults: Defaults,
    #[cfg(feature = "gui")]
    pub gui: Gui,
    #[cfg(feature = "audio")]
    pub audio: rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: rodio::OutputStreamHandle,
}

impl Shura {
    /// Start a new game with the given callback to initialize the first [SceneController].
    pub fn init<C: SceneCreator>(creator: C) {
        info!("Using shura version: {}", env!("CARGO_PKG_VERSION"));
        let events = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_title("Shura Game")
            .build(&events)
            .unwrap();
        let shura_window_id = window.id();
        let mut init = Some(creator);
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
                                    shura.resize(
                                        (*physical_size as winit::dpi::PhysicalSize<u32>).into(),
                                    );
                                }
                                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                    shura.resize(
                                        (**new_inner_size as winit::dpi::PhysicalSize<u32>).into(),
                                    );
                                }
                                _ => shura.input.event(event),
                            }
                        }
                    }
                    Event::RedrawRequested(window_id) if window_id == shura_window_id => {
                        let mut scene = shura.scene_manager.borrow_active_scene();
                        if let Some(max_frame_time) = scene.render_config.max_frame_time() {
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
                                shura.resize(shura.window.inner_size().into())
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
                        active = Some(Shura::new(
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
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        mut creator: C,
    ) -> Self {
        let gpu = pollster::block_on(Gpu::new(&window));
        let defaults = Defaults::new(&gpu);
        #[cfg(feature = "audio")]
        let (audio, audio_handle) = rodio::OutputStream::try_default().unwrap();
        let mut shura = Self {
            scene_manager: SceneManager::new(creator.id()),
            frame_manager: FrameManager::new(),
            input: Input::new(),
            #[cfg(feature = "audio")]
            audio,
            #[cfg(feature = "audio")]
            audio_handle,
            end: false,
            #[cfg(feature = "gui")]
            gui: Gui::new(event_loop, &gpu),
            window,
            gpu: gpu,
            defaults,
        };
        let scene = creator.create(&mut shura);
        shura.scene_manager.init(scene);
        return shura;
    }

    fn end(&mut self, cf: &mut winit::event_loop::ControlFlow) {
        // for scene in self.scene_manager.end_scenes() {
        //     scene.1.unwrap().end(self);
        // }
        *cf = winit::event_loop::ControlFlow::Exit
    }

    fn resize(&mut self, new_size: Dimension<u32>) {
        let config_size = self.gpu.render_size_no_scale();
        if new_size.width > 0 && new_size.height > 0 && new_size != config_size {
            let active = self.scene_manager.resize();
            self.gpu.resize(new_size);
            self.defaults
                .resize(&self.gpu, active.render_config.render_scale());
            #[cfg(feature = "gui")]
            self.gui.resize(&new_size);
        }
    }

    #[cfg(feature = "physics")]
    fn step(ctx: &mut Context) {
        use crate::ComponentTypeId;

        let delta = ctx.frame_time();
        ctx.scene.world.step(delta);
        // while let Ok(contact_force_event) = ctx.scene.world.event_receivers.1.try_recv() {
        // }
        while let Ok(collision_event) = ctx.scene.world.collision_event() {
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
                        ctx: &mut Context,
                        self_handle: ComponentHandle,
                        other_handle: ComponentHandle,
                        self_collider: ColliderHandle,
                        other_collider: ColliderHandle,
                        self_type: ComponentTypeId,
                        collide_type: CollideType,
                    ) {
                        let path = ArenaPath {
                            group_index: self_handle.group_index(),
                            type_index: self_handle.type_index(),
                        };
                        let i = self_handle.component_index().index() as usize;
                        if let Some(mut entry) =
                            ctx.scene.component_manager.borrow_component(path, i)
                        {
                            match &mut entry {
                                crate::data::arena::ArenaEntry::Occupied { data, .. } => {
                                    ctx.scene.component_manager.set_current_type(self_type);
                                    ctx.scene
                                        .component_manager
                                        .set_current_component(self_handle);
                                    data.collision(
                                        ctx,
                                        other_handle,
                                        self_collider,
                                        other_collider,
                                        collide_type,
                                    );
                                }
                                _ => {}
                            };

                            if ctx.scene.component_manager.remove_current_commponent() {
                                ctx.scene.component_manager.not_return_component(path, i)
                            } else {
                                ctx.scene.component_manager.return_component(path, i, entry);
                            }
                        }
                    }
                    let (component_type1, component1) = ctx.component_from_collider(&collider_handle1).unwrap();
                    let (component_type2, component2) = ctx.component_from_collider(&collider_handle2).unwrap();
                    let collider1_events = collider1.active_events();
                    let collider2_events = collider2.active_events();
                    if collider1_events == ActiveEvents::COLLISION_EVENTS {
                        call_collide(
                            ctx,
                            component1,
                            component2,
                            collider_handle1,
                            collider_handle2,
                            component_type1,
                            collision_type,
                        );
                    }
                    if collider2_events == ActiveEvents::COLLISION_EVENTS {
                        call_collide(
                            ctx,
                            component2,
                            component1,
                            collider_handle2,
                            collider_handle1,
                            component_type2,
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

        let window_size = ctx.window_size();
        ctx.shura.frame_manager.update();
        #[cfg(feature = "gui")]
        ctx.shura.gui.begin(
            &ctx.shura.frame_manager.total_time_duration(),
            &ctx.shura.window,
        );

        if ctx.scene.resized {
            ctx.scene
                .camera
                .resize(window_size.width as f32 / window_size.height as f32);
        }

        if ctx.scene.switched {
            ctx.shura
                .defaults
                .apply_render_scale(&ctx.shura.gpu, ctx.scene.render_config.render_scale());
        }

        ctx.scene
            .cursor
            .compute(&ctx.scene.camera, &window_size, &ctx.shura.input);

        #[cfg(feature = "physics")]
        let mut done_step = false;
        let now = ctx.update_time();

        if ctx.update_components() {
            let mut sets = ctx.scene.component_manager.borrow_active_components();
            for ((_, id), set) in &mut sets {
                let config = set.config();
                ctx.scene.component_manager.set_current_type(*id);

                #[cfg(feature = "physics")]
                if !done_step && config.priority > ctx.physics_priority() {
                    done_step = true;
                    Self::step(&mut ctx);
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
                        if let Some(mut entry) =
                            ctx.scene.component_manager.borrow_component(*path, i)
                        {
                            match &mut entry {
                                crate::data::arena::ArenaEntry::Occupied { data, .. } => {
                                    ctx.scene
                                        .component_manager
                                        .set_current_component(*data.base().handle());
                                    data.update(&mut ctx);
                                }
                                _ => (),
                            };

                            if ctx.scene.component_manager.remove_current_commponent() {
                                #[cfg(feature = "physics")]
                                match &mut entry {
                                    crate::data::arena::ArenaEntry::Occupied { data, .. } => {
                                        if let Some(p) =
                                            data.base_mut().downcast_mut::<PhysicsComponent>()
                                        {
                                            p.remove_from_world(&mut ctx.scene.world);
                                        }
                                    }
                                    _ => (),
                                };
                                ctx.scene.component_manager.not_return_component(*path, i)
                            } else {
                                ctx.scene
                                    .component_manager
                                    .return_component(*path, i, entry);
                            }
                            i += 1;
                        } else {
                            continue 'outer;
                        }
                    }
                }
            }

            ctx.scene.component_manager.return_active_components(sets)
        }

        #[cfg(feature = "physics")]
        if !done_step {
            Self::step(&mut ctx);
        }

        ctx.shura.input.update();
        ctx.scene.camera.apply_target(
            &ctx.scene.component_manager,
            #[cfg(feature = "physics")]
            &ctx.scene.world,
        );
        ctx.scene.component_manager.update_sets(&ctx.scene.camera);
        ctx.scene.resized = false;
        ctx.scene.switched = false;

        if !ctx.render_components() {
            return Ok(());
        }

        ctx.scene.component_manager.buffer_sets(
            &ctx.shura.gpu,
            #[cfg(feature = "physics")]
            &ctx.scene.world,
        );
        ctx.shura.defaults.buffer(
            &ctx.scene.camera,
            &ctx.shura.gpu,
            ctx.shura.frame_manager.total_time(),
            ctx.shura.frame_manager.frame_time(),
        );

        let output = ctx.shura.gpu.surface.get_current_texture()?;
        let mut encoder = ctx.shura.gpu.encoder();

        let mut saved_sprites = vec![];
        let render_size = ctx.render_size();
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Clear the texture
        if let Some(clear_color) = ctx.clear_color() {
            Renderer::clear(
                &mut encoder,
                &ctx.shura.defaults.target_view,
                &ctx.shura.defaults.target_msaa,
                clear_color,
            );
        }
        for set in ctx.scene.component_manager.active_components().values() {
            if set.is_empty() {
                continue;
            }
            let config = set.config();
            let mut save_sprite: Option<String> = None;
            if config.render != RenderOperation::None {
                let mut renderer = if config.postproccess == PostproccessOperation::SeperateLayer {
                    Renderer::clear(
                        &mut encoder,
                        &ctx.shura.defaults.layer_view,
                        &ctx.shura.defaults.layer_msaa,
                        Color::TRANSPARENT,
                    );
                    Renderer::new(
                        &ctx.shura.gpu,
                        &ctx.shura.defaults,
                        &mut encoder,
                        &ctx.shura.defaults.layer_view,
                        &ctx.shura.defaults.layer_msaa,
                    )
                } else {
                    Renderer::new(
                        &ctx.shura.gpu,
                        &ctx.shura.defaults,
                        &mut encoder,
                        &ctx.shura.defaults.target_view,
                        &ctx.shura.defaults.target_msaa,
                    )
                };
                match &config.camera {
                    crate::CameraUse::World => {
                        renderer.enable_camera(&ctx.shura.defaults.world_camera);
                    }
                    crate::CameraUse::Relative => {
                        renderer.enable_camera(&ctx.shura.defaults.relative_camera);
                    }
                }
                match config.render {
                    RenderOperation::Grouped => {
                        for path in set.paths() {
                            let group =
                                ctx.scene.component_manager.group(path.group_index).unwrap();
                            let component_type = group.type_ref(path.type_index).unwrap();
                            let len = component_type.len();
                            let buffer = component_type.buffer();
                            renderer.set_instance_buffer(buffer);
                            if let Some((_, first_component)) = component_type.iter().next() {
                                let instances = 0..len as u32;
                                first_component.call_grouped_render(
                                    &ctx,
                                    &mut renderer,
                                    component_type.iter(),
                                    instances,
                                );
                            }
                        }
                        save_sprite = renderer.save_sprite.take();
                    }
                    _ => {}
                }
            }

            if let Some(sprite_name) = save_sprite.take() {
                let mut sprite = Sprite::empty(&ctx.shura.gpu, render_size);
                sprite.write_current_render(
                    &ctx.shura.gpu,
                    &ctx.shura.defaults,
                    &mut encoder,
                    &ctx.shura.defaults.relative_camera,
                );
                saved_sprites.push((sprite_name, sprite));
            }

            if config.postproccess != PostproccessOperation::None {
                'outer: for path in set.paths() {
                    let group = ctx.scene.component_manager.group(path.group_index).unwrap();
                    let component_type = group.type_ref(path.type_index).unwrap();
                    for (_, component) in component_type.iter() {
                        let instances = 0..1;
                        match config.postproccess {
                            PostproccessOperation::SameLayer => {
                                let mut copy = Sprite::empty(
                                    &ctx.shura.gpu,
                                    *ctx.shura.defaults.target.size(),
                                );
                                copy.write_current_render(
                                    &ctx.shura.gpu,
                                    &ctx.shura.defaults,
                                    &mut encoder,
                                    &ctx.shura.defaults.relative_camera,
                                );
                                let mut renderer = Renderer::new(
                                    &ctx.shura.gpu,
                                    &ctx.shura.defaults,
                                    &mut encoder,
                                    &ctx.shura.defaults.target_view,
                                    &ctx.shura.defaults.target_msaa,
                                );
                                renderer
                                    .use_uniform(ctx.shura.defaults.relative_camera.uniform(), 0);
                                renderer.set_instance_buffer(
                                    &ctx.shura.defaults.single_centered_instance,
                                );
                                component.call_postproccess(
                                    &ctx,
                                    &mut renderer,
                                    instances,
                                    ctx.shura.defaults.relative_camera.model(),
                                    &copy,
                                );
                            }
                            PostproccessOperation::SeperateLayer => {
                                let mut renderer = Renderer::new(
                                    &ctx.shura.gpu,
                                    &ctx.shura.defaults,
                                    &mut encoder,
                                    &ctx.shura.defaults.target_view,
                                    &ctx.shura.defaults.target_msaa,
                                );
                                renderer
                                    .use_uniform(ctx.shura.defaults.relative_camera.uniform(), 0);
                                renderer.set_instance_buffer(
                                    &ctx.shura.defaults.single_centered_instance,
                                );
                                component.call_postproccess(
                                    &ctx,
                                    &mut renderer,
                                    instances,
                                    ctx.shura.defaults.relative_camera.model(),
                                    &ctx.shura.defaults.layer,
                                );
                            }
                            PostproccessOperation::None => {}
                        }
                        break 'outer;
                    }
                }

                if let Some(sprite_name) = save_sprite.take() {
                    let mut sprite = Sprite::empty(&ctx.shura.gpu, render_size);
                    sprite.write_current_render(
                        &ctx.shura.gpu,
                        &ctx.shura.defaults,
                        &mut encoder,
                        &ctx.shura.defaults.relative_camera,
                    );
                    saved_sprites.push((sprite_name, sprite));
                }
            }
        }

        ctx.scene.saved_sprites = saved_sprites;

        {
            let mut renderer = Renderer::new(
                &ctx.shura.gpu,
                &ctx.shura.defaults,
                &mut encoder,
                &output_view,
                &ctx.shura.defaults.present_msaa,
            );
            renderer.use_uniform(ctx.shura.defaults.relative_camera.uniform(), 0);
            renderer.set_instance_buffer(&ctx.shura.defaults.single_centered_instance);
            renderer.render_sprite(
                ctx.shura.defaults.relative_camera.model(),
                &ctx.shura.defaults.target,
            );
            renderer.commit(0..1);
        }
        #[cfg(feature = "gui")]
        {
            ctx.shura
                .gui
                .render(&ctx.shura.gpu, &mut encoder, &output_view);
        }
        ctx.shura.gpu.finish_enocder(encoder);
        output.present();

        Ok(())
    }
}
