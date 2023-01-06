#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "physics")]
use crate::{
    physics::{ActiveEvents, CollideType, ColliderHandle, PhysicsComponent},
    ArenaPath, ComponentHandle, DynamicScene,
};
use crate::{
    BoxedScene, Color, ComponentSet, Context, Dimension, FrameManager, Gpu, Input,
    PostproccessOperation, RenderOperation, Renderer, Scene, SceneController, SceneManager, Sprite,
};
use log::{error, info};
use winit::event_loop::EventLoop;

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

/// Start a new game with the given callback to initialize the first [SceneController].
pub fn init<S: SceneController, F: 'static + FnMut(&mut Context) -> S>(
    scene_name: &'static str,
    init: F,
) {
    let events = winit::event_loop::EventLoop::new();
    let shura = ShuraCore::new(&events, init, scene_name);
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
        browser_window
            .document()
            .and_then(|doc| doc.body())
            .and_then(|body| body.append_child(canvas).ok())
            .expect("Couldn't append canvas to document body!");

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
    shura.run(events);
}

pub(crate) struct ShuraCore<S: SceneController, F: FnMut(&mut Context) -> S> {
    pub init: Option<F>,
    pub frame_manager: FrameManager,
    pub scene_manager: SceneManager,
    pub window: winit::window::Window,
    pub input: Input,
    #[cfg(feature = "audio")]
    pub audio: (rodio::OutputStream, rodio::OutputStreamHandle),
    pub end: bool,

    pub gpu: Option<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: Option<Gui>,
}

impl<S: SceneController, F: 'static + FnMut(&mut Context) -> S> ShuraCore<S, F> {
    pub fn run(mut self, events: winit::event_loop::EventLoop<()>) {
        info!("Using shura version: {}", env!("CARGO_PKG_VERSION"));

        let mut active_scene: Option<BoxedScene> = if cfg!(target_os = "android") {
            None
        } else {
            Some(self.init())
        };

        events.run(move |event, _, control_flow| {
            use winit::event::{Event, WindowEvent};
            #[cfg(feature = "gui")]
            if let Some(gui) = self.gui.as_mut() {
                gui.handle_event(&event)
            }
            match event {
                #[cfg(target_os = "android")]
                Event::Resumed => {
                    if let Some(gpu) = self.gpu.as_mut() {
                        gpu.resume(&self.window);
                    } else {
                        active_scene = Some(self.init());
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } => {
                    if window_id == self.window.id() && self.gpu.is_some() {
                        let scene = active_scene.as_mut().unwrap();
                        match event {
                            WindowEvent::CloseRequested => {
                                self.end(scene, control_flow);
                            }
                            WindowEvent::Resized(physical_size) => {
                                self.resize(
                                    scene,
                                    (*physical_size as winit::dpi::PhysicalSize<u32>).into(),
                                );
                            }
                            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                self.resize(
                                    scene,
                                    (**new_inner_size as winit::dpi::PhysicalSize<u32>).into(),
                                );
                            }
                            _ => self.input.event(event),
                        }
                    }
                }
                Event::RedrawRequested(window_id)
                    if window_id == self.window.id() && self.gpu.is_some() =>
                {
                    if let Some(new_name) = self.scene_manager.new_active_scene() {
                        active_scene = Some(
                            self.scene_manager
                                .swap_active_scene(active_scene.take().unwrap(), new_name),
                        )
                    }
                    let scene = active_scene.as_mut().unwrap();
                    match self.update(scene) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            self.resize(scene, self.window.inner_size().into())
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            *control_flow = winit::event_loop::ControlFlow::Exit
                        }
                        Err(e) => error!("{:?}", e),
                    }
                    if self.end {
                        self.end(scene, control_flow);
                    }
                }
                Event::MainEventsCleared => {
                    self.window.request_redraw();
                }
                _ => {}
            }
        });
    }

    fn new(events: &EventLoop<()>, init: F, scene_name: &'static str) -> ShuraCore<S, F> {
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_title(scene_name)
            .build(events)
            .unwrap();

        Self {
            window,
            init: Some(init),
            scene_manager: SceneManager::new(scene_name),
            frame_manager: FrameManager::new(),
            input: Input::new(),
            #[cfg(feature = "audio")]
            audio: rodio::OutputStream::try_default().unwrap(),
            end: false,
            gpu: None,
            #[cfg(feature = "gui")]
            gui: None,
        }
    }

    fn init(&mut self) -> BoxedScene {
        let gpu = pollster::block_on(Gpu::new(&self.window));
        let mut init = self.init.take().unwrap();
        #[cfg(feature = "gui")]
        {
            self.gui = Some(Gui::new(&self.window, &gpu));
        }
        let window_size: Dimension<u32> = self.window.inner_size().into();
        let window_ratio = window_size.width as f32 / window_size.height as f32;
        let mut scene = Scene::new(&gpu, window_ratio, self.scene_manager.active_scene());
        self.gpu = Some(gpu);
        let controller = {
            let mut ctx = Context::new(&mut scene, self);
            Box::new((init)(&mut ctx))
        };
        return (controller, scene);
    }

    fn end(&mut self, main_scene: &mut BoxedScene, cf: &mut winit::event_loop::ControlFlow) {
        #[cfg(feature = "gui")]
        {
            let gui = self.gui.as_mut().unwrap();
            gui.begin(self.frame_manager.total_time_duration());
        }

        {
            let mut ctx = Context::new(&mut main_scene.1, self);
            main_scene.0.end(&mut ctx);
        }
        for (_, mut scene) in self.scene_manager.end_scenes() {
            let mut ctx = Context::new(&mut scene.1, self);
            scene.0.end(&mut ctx);
        }
        *cf = winit::event_loop::ControlFlow::Exit
    }

    fn resize(&mut self, main_scene: &mut BoxedScene, new_size: Dimension<u32>) {
        let gpu = self.gpu.as_mut().unwrap();
        let config_size = gpu.render_size_no_scale();
        if new_size.width > 0 && new_size.height > 0 && new_size != config_size {
            self.scene_manager.resize(main_scene);
            gpu.resize(new_size);
            #[cfg(feature = "gui")]
            {
                let gui = self.gui.as_mut().unwrap();
                gui.resize(&self.window, &new_size);
            }
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
        let (scene_controller, scene) = (&mut scene.0, &mut scene.1);
        self.frame_manager.update();

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
            .as_mut()
            .unwrap()
            .begin(self.frame_manager.total_time_duration());

        let res = {
            let mut ctx = Context::new(scene, self);

            scene_controller.update(&mut ctx);

            #[cfg(feature = "physics")]
            let mut done_step = false;
            let total_frames = ctx.total_frames();

            if ctx.update_components() {
                let sets = ctx.copy_active_components();
                for set in sets {
                    let config = set.config();

                    #[cfg(feature = "physics")]
                    if !done_step && config.priority > ctx.physics_priority() {
                        done_step = true;
                        Self::step(scene_controller, &mut ctx);
                    }

                    match config.update {
                        crate::UpdateOperation::None => {
                            continue;
                        }
                        crate::UpdateOperation::EveryNFrame(frames) => {
                            if ctx.total_frames() % frames != 0 {
                                continue;
                            }
                        }
                        _ => {}
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
                                                p.remove_from_world(ctx.world);
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
                ctx.set_current_component(None);
            }

            #[cfg(feature = "physics")]
            if !done_step {
                Self::step(scene_controller, &mut ctx);
                ctx.set_current_component(None);
            }

            scene_controller.after_update(&mut ctx);

            ctx.normalize_input();
            ctx.update_sets();

            if !ctx.render_components() {
                return Ok(());
            }

            ctx.buffer();
            ctx.finish()
        };

        let output = res.gpu.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = res.gpu.encoder();
        let defaults = &res.gpu.defaults;
        let relative_cmaera = &defaults.relative_camera;

        // Clear the texture
        if let Some(clear_color) = res.clear_color {
            Renderer::clear(
                &mut encoder,
                &defaults.target_view,
                &defaults.target_msaa,
                *clear_color,
            );
        }
        for (_, set) in res.manager.active_components() {
            if set.is_empty() {
                continue;
            }
            let config = set.config();
            let mut save_sprite: Option<String> = None;
            if config.render != RenderOperation::None {
                let mut renderer = if config.postproccess == PostproccessOperation::SeperateLayer {
                    Renderer::clear(
                        &mut encoder,
                        &defaults.layer_view,
                        &defaults.layer_msaa,
                        Color::TRANSPARENT,
                    );
                    Renderer::new(
                        &mut encoder,
                        &defaults.layer_view,
                        &defaults.layer_msaa,
                        res.gpu,
                    )
                } else {
                    Renderer::new(
                        &mut encoder,
                        &defaults.target_view,
                        &defaults.target_msaa,
                        res.gpu,
                    )
                };
                match &config.camera {
                    crate::CameraUse::World => {
                        renderer.enable_camera(&res.camera);
                    }
                    crate::CameraUse::Relative => {
                        renderer.enable_camera(&res.gpu.defaults.relative_camera);
                    }
                }
                match config.render {
                    RenderOperation::Solo => {
                        for path in set.paths() {
                            let group = res.manager.group(path.group_index).unwrap();
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
                            let group = res.manager.group(path.group_index).unwrap();
                            let component_type = group.type_ref(path.type_index).unwrap();
                            let len = component_type.len();
                            let buffer = component_type.buffer();
                            renderer.set_instance_buffer(buffer);
                            if let Some((_, first_component)) = component_type.iter().next() {
                                let grouped_render = first_component.get_grouped_render();
                                let instances = 0..len as u32;

                                let set = ComponentSet::new(vec![component_type], len);
                                grouped_render(scene_controller, &mut renderer, set, instances);
                                break;
                            }
                        }
                        save_sprite = renderer.save_sprite.take();
                    }
                    _ => {}
                }
            }

            if let Some(sprite_name) = save_sprite.take() {
                let mut sprite = Sprite::empty(&res.gpu, res.gpu.render_size());
                sprite.write_current_render(&mut encoder, res.gpu);
                res.saved_sprites.push((sprite_name, sprite));
            }

            if config.postproccess != PostproccessOperation::None {
                'outer: for path in set.paths() {
                    let group = res.manager.group(path.group_index).unwrap();
                    let component_type = group.type_ref(path.type_index).unwrap();
                    for (_, component) in component_type.iter() {
                        let postproccess = component.get_postproccess();
                        let instances = 0..1;
                        match config.postproccess {
                            PostproccessOperation::SameLayer => {
                                let mut copy = Sprite::empty(res.gpu, *defaults.target.size());
                                copy.write_current_render(&mut encoder, res.gpu);
                                let mut renderer = Renderer::new_compute(
                                    &mut encoder,
                                    res.gpu,
                                    &defaults.target_view,
                                    &defaults.target_msaa,
                                    &defaults.single_centered_instance,
                                    relative_cmaera.uniform(),
                                );
                                postproccess(
                                    &mut renderer,
                                    instances,
                                    relative_cmaera.model(),
                                    &copy,
                                );
                            }
                            PostproccessOperation::SeperateLayer => {
                                let mut renderer = Renderer::new_compute(
                                    &mut encoder,
                                    res.gpu,
                                    &defaults.target_view,
                                    &defaults.target_msaa,
                                    &defaults.single_centered_instance,
                                    relative_cmaera.uniform(),
                                );
                                postproccess(
                                    &mut renderer,
                                    instances,
                                    relative_cmaera.model(),
                                    &defaults.layer,
                                );
                            }
                            PostproccessOperation::None => {}
                        }
                        break 'outer;
                    }
                }

                if let Some(sprite_name) = save_sprite.take() {
                    let mut sprite = Sprite::empty(&res.gpu, res.gpu.render_size());
                    sprite.write_current_render(&mut encoder, res.gpu);
                    res.saved_sprites.push((sprite_name, sprite));
                }
            }
        }

        {
            let mut renderer = Renderer::new_compute(
                &mut encoder,
                res.gpu,
                &output_view,
                &defaults.present_msaa,
                &defaults.single_centered_instance,
                relative_cmaera.uniform(),
            );
            renderer.render_sprite(relative_cmaera.model(), &defaults.target);
            renderer.commit(&(0..1));
        }
        #[cfg(feature = "gui")]
        {
            res.gui
                .render(res.gpu, &mut encoder, &output_view, &res.window);
        }
        res.gpu.finish_enocder(encoder);
        output.present();

        Ok(())
    }
}
