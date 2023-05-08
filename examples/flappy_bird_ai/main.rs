use shura::{
    log::info,
    physics::{parry::bounding_volume::BoundingVolume, Aabb, LockedAxes, RigidBodyBuilder},
    rand::{
        distributions::{Distribution, WeightedIndex},
        gen_range, thread_rng,
    },
    *,
};

// Inspired by: https://github.com/bones-ai/rust-flappy-bird-ai

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);
const AMOUNT_BIRDS: u32 = 1000;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.set_gravity(Vector::new(0.0, -15.0));
        ctx.insert_scene_state(BirdSimulation::new(ctx));
        ctx.add_component(Background::new(ctx));
        ctx.add_component(Ground::new(ctx));
        ctx.set_physics_priority(Some(10));
        ctx.set_window_resizable(false);
        for _ in 0..AMOUNT_BIRDS {
            ctx.add_component(Bird::new());
        }
    }))
}

#[derive(State)]
struct BirdSimulation {
    bird_model: Model,
    bird_sprite: Sprite,
    top_pipe_model: Model,
    bottom_pipe_model: Model,
    pipe_sprite: Sprite,
    spawn_timer: f32,
    generation: u32,
    high_score: u32,
}

impl BirdSimulation {
    pub fn new(ctx: &Context) -> Self {
        return Self {
            bird_model: ctx.create_model(ModelBuilder::cuboid(Bird::HALF_EXTENTS)),
            bird_sprite: ctx.create_sprite(include_bytes!("./sprites/yellowbird-downflap.png")),
            top_pipe_model: ctx.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation::new(180.0_f32.to_radians())),
            ),
            bottom_pipe_model: ctx.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS).vertex_translation(Vector::new(
                    0.0,
                    -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                )),
            ),
            pipe_sprite: ctx.create_sprite(include_bytes!("./sprites/pipe-green.png")),
            spawn_timer: Pipe::SPAWN_TIME,
            generation: 0,
            high_score: 0,
        };
    }

    fn spawn_pipes(&mut self, component_manager: &mut ComponentManager) {
        self.spawn_timer = 0.0;
        let pipe = component_manager.add_component(Pipe::new());
        info!("Spawning new pipes with id: {}]", pipe.id());
    }
}

impl SceneStateController for BirdSimulation {
    fn update(ctx: &mut Context) {
        let fps = ctx.fps();
        let scene = ctx.scene_states.get_mut::<Self>();
        let time_scale = ctx.component_manager.world_mut().time_scale;
        let delta = ctx.frame_manager.frame_time() * time_scale;
        scene.spawn_timer += delta;
        let score = ctx
            .component_manager
            .components::<Bird>(ComponentFilter::All)
            .find(|b| b.body().is_enabled())
            .unwrap()
            .score as u32;
        if score > scene.high_score {
            scene.high_score = score;
        }

        if scene.spawn_timer >= Pipe::SPAWN_TIME {
            scene.spawn_pipes(ctx.component_manager);
        }

        let pipes = ctx
            .component_manager
            .components::<Pipe>(ComponentFilter::All);
        let mut closest = Vector::new(GAME_SIZE.x, 0.0);
        for pipe in pipes {
            let translation = pipe.translation();
            if translation.x >= 0.0 && translation.x < closest.x {
                closest = translation;
            }
        }

        let bottom_y = closest.y - Pipe::HALF_HOLE_SIZE;
        let top_y = closest.y + Pipe::HALF_HOLE_SIZE;

        let bottom_aabb = Aabb::new(
            (closest
                - Vector::new(
                    Pipe::HALF_EXTENTS.x,
                    Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y * 2.0,
                ))
            .into(),
            (closest + Vector::new(Pipe::HALF_EXTENTS.x, -Pipe::HALF_HOLE_SIZE)).into(),
        );
        let top_aabb = Aabb::new(
            (closest - Vector::new(Pipe::HALF_EXTENTS.x, -Pipe::HALF_HOLE_SIZE)).into(),
            (closest
                + Vector::new(
                    Pipe::HALF_EXTENTS.x,
                    Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y * 2.0,
                ))
            .into(),
        );
        let x = closest.x;
        assert!(x >= 0.0);

        for bird in ctx
            .component_manager
            .components_mut::<Bird>(ComponentFilter::Active)
        {
            let mut body = bird.base.body_mut();
            let pos = *body.translation();
            let bl = pos - Bird::HALF_EXTENTS;
            let tr = pos + Bird::HALF_EXTENTS;
            let bird_aabb = Aabb::new(bl.into(), tr.into());
            assert!(bird_aabb.maxs > bird_aabb.mins);
            if bl.y < -GAME_SIZE.y + Ground::HALF_EXTENTS.y * 2.0 || tr.y > GAME_SIZE.y {
                body.set_enabled(false);
            }

            if bird_aabb.intersects(&bottom_aabb) || bird_aabb.intersects(&top_aabb) {
                body.set_enabled(false);
            }

            if !body.is_enabled() {
                continue;
            }

            bird.score += delta * 1.0;
            let out = bird.brain.predict(&vec![
                body.translation().y as f64,
                bottom_y as f64,
                top_y as f64,
                x as f64,
                body.linvel().y as f64,
            ])[0];

            if out >= 0.5 {
                body.set_linvel(Vector::new(0.0, 5.0), true);
            }
        }

        let dead_count = ctx
            .component_manager
            .components::<Bird>(ComponentFilter::All)
            .filter(|b| b.body().is_enabled())
            .count();

        if dead_count == 0 {
            let mut max_fitness = 0.0;
            let mut weights = Vec::new();

            for b in ctx
                .component_manager
                .components::<Bird>(ComponentFilter::All)
            {
                if b.score > max_fitness {
                    max_fitness = b.score;
                }
                weights.push(b.score);
            }
            weights
                .iter_mut()
                .for_each(|i| *i = (*i / max_fitness) * 100.0);

            let gene_pool = WeightedIndex::new(&weights).expect("Failed to generate gene pool");

            let amount = ctx
                .component_manager
                .amount_of_components::<Bird>(ComponentGroupId::DEFAULT);
            let mut rng = thread_rng();
            let mut new_birds = Vec::with_capacity(amount);
            for _ in 0..amount {
                let index = gene_pool.sample(&mut rng);
                let rand_bird = ctx
                    .component_manager
                    .component_by_index_mut::<Bird>(ComponentGroupId::DEFAULT, index as u32)
                    .unwrap();

                let mut new_bird = Bird::with_brain(&rand_bird);
                new_bird.brain.mutate();
                new_birds.push(new_bird);
            }
            ctx.component_manager
                .remove_components::<Bird>(ComponentFilter::All);
            ctx.component_manager.add_components(new_birds);

            scene.generation += 1;
            info!("Now at generation {}!", scene.generation);
            ctx.component_manager
                .remove_components::<Pipe>(ComponentFilter::All);
            scene.spawn_pipes(ctx.component_manager);
        }

        gui::Window::new("Flappy Bird")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(&format!("FPS: {}", fps));
                ui.label(format!("Generation: {}", scene.generation));
                ui.label(format!("Score: {}", score));
                ui.label(format!("High Score: {}", scene.high_score));
                ui.label(format!("Birds: {}", dead_count as u32));
                ui.add(
                    gui::Slider::new(
                        &mut ctx.component_manager.world_mut().time_scale,
                        0.1..=20.0,
                    )
                    .text("Speed"),
                );
            });
    }
}

#[derive(Component)]
struct Bird {
    #[base]
    base: BaseComponent,
    brain: NeuralNetwork,
    score: f32,
}

impl Bird {
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.3, 0.21176472);
    pub fn new() -> Self {
        Self {
            base: BaseComponent::new_body(
                RigidBodyBuilder::dynamic()
                    .locked_axes(LockedAxes::TRANSLATION_LOCKED_X)
                    .lock_rotations()
                    .additional_mass(3.0),
                &[],
            ),
            score: 0.0,
            brain: NeuralNetwork::new(vec![5, 8, 1]),
        }
    }

    pub fn with_brain(other: &Bird) -> Self {
        let mut new_bird = Bird::new();
        new_bird.brain = other.brain.clone();
        new_bird
    }
}

impl ComponentController for Bird {
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_state::<BirdSimulation>();
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, bird, instance| {
                if bird.body().is_enabled() {
                    r.render_sprite(instance, &scene.bird_model, &scene.bird_sprite)
                }
            },
        );
    }
}

#[derive(Component)]
struct Ground {
    model: Model,
    sprite: Sprite,
    #[base]
    base: BaseComponent,
}

impl Ground {
    const HALF_EXTENTS: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS)),
            sprite: ctx.create_sprite(include_bytes!("./sprites/base.png")),
            base: BaseComponent::new(
                PositionBuilder::new()
                    .translation(Vector::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y)),
            ),
        }
    }
}

impl ComponentController for Ground {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, ground, instance| r.render_sprite(instance, &ground.model, &ground.sprite),
        );
    }
}

#[derive(Component)]
struct Background {
    model: Model,
    sprite: Sprite,
    #[base]
    base: BaseComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx.create_sprite(include_bytes!("./sprites/background-night.png"));
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(GAME_SIZE)),
            sprite,
            base: BaseComponent::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        ..DEFAULT_CONFIG
    };
    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        ctx.render_each(
            active,
            encoder,
            RenderConfig::default(),
            |r, background, instance| {
                r.render_sprite(instance, &background.model, &background.sprite)
            },
        );
    }
}

#[derive(Component)]
struct Pipe {
    #[base]
    base: BaseComponent,
}

impl Pipe {
    const PIPE_SPEED: f32 = -3.0;
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.65, 4.0);
    const HALF_HOLE_SIZE: f32 = 1.1;
    const MIN_PIPE_Y: f32 = 0.25;
    const SPAWN_TIME: f32 = 3.0;
    pub fn new() -> Self {
        let y = gen_range(
            -GAME_SIZE.y + Self::MIN_PIPE_Y + Pipe::HALF_HOLE_SIZE + Ground::HALF_EXTENTS.y * 2.0
                ..GAME_SIZE.y - Self::MIN_PIPE_Y - Pipe::HALF_HOLE_SIZE,
        );
        return Self {
            base: BaseComponent::new_body(
                RigidBodyBuilder::kinematic_velocity_based()
                    .translation(Vector::new(GAME_SIZE.x, y))
                    .linvel(Vector::new(Self::PIPE_SPEED, 0.0)),
                &[],
            ),
        };
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(active: &ActiveComponents<Self>, ctx: &mut Context) {
        let mut to_remove: Vec<ComponentHandle> = vec![];
        for pipe in ctx.active_mut(active) {
            let x = pipe.base.translation().x;
            if x <= -GAME_SIZE.x {
                let handle = pipe.base.handle();
                to_remove.push(handle);
                info!("Removing Pipe with id: {}", handle.id());
            }
        }

        for handle in to_remove {
            ctx.remove_component(handle);
        }
    }

    fn render(active: &ActiveComponents<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_state::<BirdSimulation>();
        ctx.render_all(active, encoder, RenderConfig::default(), |r, instances| {
            r.render_sprite(instances.clone(), &scene.top_pipe_model, &scene.pipe_sprite);
            r.render_sprite(instances, &scene.bottom_pipe_model, &scene.pipe_sprite);
        });
    }
}

#[derive(Clone)]
pub struct NeuralNetwork {
    inputs: usize,
    layers: Vec<NetworkLayer>,
}

impl NeuralNetwork {
    pub fn new(layer_sizes: Vec<usize>) -> Self {
        assert!(layer_sizes.len() >= 2, "Need at least 2 layers");
        for &size in layer_sizes.iter() {
            assert!(size >= 1, "Empty layers not allowed");
        }

        let first_layer_size = *layer_sizes.first().unwrap();
        let mut layers = Vec::new();
        let mut prev_layer_size = first_layer_size;

        for &layer_size in layer_sizes[1..].iter() {
            layers.push(NetworkLayer::new(layer_size, prev_layer_size));
            prev_layer_size = layer_size;
        }

        Self {
            layers,
            inputs: first_layer_size,
        }
    }

    pub fn predict(&self, inputs: &Vec<f64>) -> Vec<f64> {
        assert_eq!(inputs.len(), self.inputs, "Bad input size");

        let mut outputs = Vec::new();
        outputs.push(inputs.clone());
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let layer_results = layer.predict(&outputs[layer_index]);
            outputs.push(layer_results);
        }

        outputs.pop().unwrap()
    }

    pub fn mutate(&mut self) {
        self.layers.iter_mut().for_each(|l| l.mutate());
    }
}

#[derive(Clone)]
struct NetworkLayer {
    nodes: Vec<Vec<f64>>,
}

impl NetworkLayer {
    pub const BRAIN_MUTATION_RATE: f64 = 0.02;
    pub const BRAIN_MUTATION_VARIATION: f64 = 0.2;
    fn new(layer_size: usize, prev_layer_size: usize) -> Self {
        let mut nodes: Vec<Vec<f64>> = Vec::new();

        for _ in 0..layer_size {
            let mut node: Vec<f64> = Vec::new();
            for _ in 0..prev_layer_size + 1 {
                let random_weight: f64 = gen_range(-1.0f64..1.0f64);
                node.push(random_weight);
            }
            nodes.push(node);
        }

        Self { nodes }
    }

    fn predict(&self, inputs: &Vec<f64>) -> Vec<f64> {
        let mut layer_results = Vec::new();
        for node in self.nodes.iter() {
            layer_results.push(self.sigmoid(self.dot_prod(&node, &inputs)));
        }

        layer_results
    }

    fn mutate(&mut self) {
        for n in self.nodes.iter_mut() {
            for val in n.iter_mut() {
                if gen_range(0.0..1.0) >= Self::BRAIN_MUTATION_RATE {
                    continue;
                }
                *val += gen_range(-Self::BRAIN_MUTATION_VARIATION..Self::BRAIN_MUTATION_VARIATION);
            }
        }
    }

    fn dot_prod(&self, node: &Vec<f64>, values: &Vec<f64>) -> f64 {
        let mut it = node.iter();
        let mut total = *it.next().unwrap();
        for (weight, value) in it.zip(values.iter()) {
            total += weight * value;
        }

        total
    }

    fn sigmoid(&self, y: f64) -> f64 {
        1f64 / (1f64 + (-y).exp())
    }
}
