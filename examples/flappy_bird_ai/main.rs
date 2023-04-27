use ::rand::prelude::Distribution;
use shura::{
    log::info,
    physics::{
        ActiveEvents, CollideType, ColliderBuilder, ColliderHandle, Group, InteractionGroups,
        LockedAxes, RigidBodyBuilder,
    },
    rand::{distributions::WeightedIndex, gen_range, thread_rng},
    *,
};

// Inspired by: https://github.com/bones-ai/rust-flappy-bird-ai

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        ctx.set_camera_scale(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.set_gravity(Vector::new(0.0, -20.0));
        ctx.set_scene_state(BirdSimulation::new(ctx));
        // ctx.set_window_size(Vector::new(800, 600));
        // ctx.set_window_resizable(false);
        ctx.add_component(Background::new(ctx));
        ctx.add_component(Ground::new(ctx));
        for _ in 0..500 {
            ctx.add_component(Bird::new());
        }
    }))
}

#[derive(State)]
struct BirdSimulation {
    bird_model: Model,
    bird_sprite: Sprite,
    pipe_model: Model,
    pipe_sprite: Sprite,
    last_spawn: f32,
    generation: u32,
}

impl BirdSimulation {
    const SPAWN_TIME: f32 = 3.5;
    const HOLE_SIZE: f32 = 2.2;
    const MIN_PIPE_Y: f32 = 0.5;
    pub fn new(ctx: &Context) -> Self {
        return Self {
            bird_model: ctx.create_model(ModelBuilder::cuboid(Bird::SIZE)),
            bird_sprite: ctx.create_sprite(include_bytes!("./sprites/yellowbird-downflap.png")),
            pipe_model: ctx.create_model(ModelBuilder::cuboid(Pipe::SIZE)),
            pipe_sprite: ctx.create_sprite(include_bytes!("./sprites/pipe-green.png")),
            last_spawn: -Self::SPAWN_TIME,
            // score: 0,
            generation: 0,
        };
    }

    fn spawn_pipes(ctx: &mut Context) {
        let total_time = ctx.total_time();
        let scene = ctx.scene_state_mut::<Self>();
        scene.last_spawn = total_time;
        let under_y = gen_range(
            -GAME_SIZE.y + Self::MIN_PIPE_Y + Ground::SIZE.y * 2.0
                ..GAME_SIZE.y - Self::MIN_PIPE_Y - Self::HOLE_SIZE,
        );
        let (_, pipe1) = ctx.add_component(Pipe::new(
            Vector::new(GAME_SIZE.x, under_y - Pipe::SIZE.y),
            false,
        ));
        let (_, pipe2) = ctx.add_component(Pipe::new(
            Vector::new(GAME_SIZE.x, under_y + Pipe::SIZE.y + Self::HOLE_SIZE),
            true,
        ));
        info!(
            "Spawning new pipes with ids: [{}, {}]",
            pipe1.id(),
            pipe2.id()
        );
    }
}

impl SceneStateController for BirdSimulation {
    fn update(ctx: &mut Context) {
        let mut new_pipe = false;
        let total_time = ctx.total_time();
        let fps = ctx.fps();
        let score = ctx
            .components::<Bird>(ComponentFilter::All)
            .find(|b| b.body().is_enabled())
            .unwrap()
            .score as u32;
        let scene = ctx.scene_state.downcast_mut::<Self>().unwrap();

        gui::Window::new("Flappy Bird")
            .anchor(gui::Align2::LEFT_TOP, gui::Vec2::default())
            .resizable(false)
            .collapsible(false)
            .show(&ctx.gui.clone(), |ui| {
                ui.label(&format!("FPS: {}", fps));
                ui.label(format!("Generation: {}", scene.generation));
                ui.label(format!("Score: {}", score));
            });

        if total_time >= scene.last_spawn + Self::SPAWN_TIME {
            new_pipe = true;
        }

        if new_pipe {
            Self::spawn_pipes(ctx);
        }
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
    const SIZE: Vector<f32> = Vector::new(0.3, 0.21176472);
    pub fn new() -> Self {
        Self {
            base: BaseComponent::new_body(
                RigidBodyBuilder::dynamic()
                    .locked_axes(LockedAxes::TRANSLATION_LOCKED_X)
                    .lock_rotations(),
                vec![ColliderBuilder::cuboid(Self::SIZE.x, Self::SIZE.y)
                    .active_events(ActiveEvents::COLLISION_EVENTS)
                    .collision_groups(InteractionGroups {
                        memberships: Group::GROUP_2,
                        filter: Group::GROUP_1,
                    })],
            ),
            score: 0.0,
            brain: NeuralNetwork::new(vec![4, 8, 1]),
        }
    }
}

impl ComponentController for Bird {
    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        let mut pipes = ctx.components::<Pipe>(ComponentFilter::All);
        let bottom_pipe = pipes.next().unwrap().translation();
        let top_pipe = pipes.next().unwrap().translation();

        for bird in ctx.component_manager.path_mut(&active) {
            bird.score += ctx.frame_manager.frame_time() * 1.0;
            let mut body = bird.base.body_mut();

            let out = bird.brain.predict(&vec![
                bottom_pipe.x as f64,
                top_pipe.x as f64,
                body.translation().y as f64,
                body.linvel().y as f64,
            ])[0];

            if out >= 0.5 {
                body.set_linvel(Vector::new(0.0, 6.0), true);
            }
        }
    }

    fn collision(
        ctx: &mut Context,
        self_handle: ComponentHandle,
        _other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        _other_collider: ColliderHandle,
        collision_type: CollideType,
    ) {
        if collision_type == CollideType::Started {
            let bird = ctx.component_mut::<Self>(self_handle).unwrap();
            bird.body_mut().set_enabled(false);

            let dead_count = ctx
                .components::<Bird>(ComponentFilter::All)
                .filter(|b| b.body().is_enabled())
                .count();

            if dead_count == 0 {
                let mut max_fitness = 0.0;
                let mut weights = Vec::new();

                for b in ctx.components::<Bird>(ComponentFilter::All) {
                    if b.score > max_fitness {
                        max_fitness = b.score;
                    }
                    weights.push(b.score);
                }
                weights
                    .iter_mut()
                    .for_each(|i| *i = (*i / max_fitness) * 100.0);

                let gene_pool = WeightedIndex::new(&weights).expect("Failed to generate gene pool");

                let mut rng = thread_rng();
                for _ in 0..ctx.amount_of_components::<Bird>(DEFAULT_GROUP_ID) {
                    let index = gene_pool.sample(&mut rng);
                    let rand_bird = ctx
                        .components_mut::<Bird>(ComponentFilter::DEFAULT_GROUP)
                        .nth(index)
                        .unwrap();
                    rand_bird.brain.mutate();
                }

                let scene = ctx.scene_state_mut::<BirdSimulation>();
                scene.generation += 1;
                info!("Now at generation {}!", scene.generation);
                ctx.remove_components::<Pipe>(Default::default());
                BirdSimulation::spawn_pipes(ctx);
                for bird in ctx.components_mut::<Bird>(ComponentFilter::All) {
                    bird.score = 0.0;
                    let mut body = bird.body_mut();
                    body.set_linvel(Vector::new(0.0, 0.0), true);
                    body.set_translation(Vector::new(0.0, 0.0), true);
                    body.set_enabled(true);
                }
            }
        }
    }

    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
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
        // let scene = ctx.scene_state::<BirdSimulation>();
        // ctx.render_all(active, encoder, RenderConfig::default(), |r, instance| {
        //     r.render_sprite(instance, &scene.bird_model, &scene.bird_sprite)
        // });
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
    const SIZE: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Self::SIZE)),
            sprite: ctx.create_sprite(include_bytes!("./sprites/base.png")),
            base: BaseComponent::new_body(
                RigidBodyBuilder::fixed()
                    .translation(Vector::new(0.0, -GAME_SIZE.y + Self::SIZE.y)),
                vec![
                    ColliderBuilder::cuboid(Self::SIZE.x, Self::SIZE.y),
                    ColliderBuilder::segment(
                        Point::new(-GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - Self::SIZE.y),
                        Point::new(GAME_SIZE.x, GAME_SIZE.y + GAME_SIZE.y - Self::SIZE.y),
                    ),
                ],
            ),
        }
    }
}

impl ComponentController for Ground {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 2,
        ..DEFAULT_CONFIG
    };
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
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
    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
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
    const PIPE_SPEED: f32 = -2.0;
    const SIZE: Vector<f32> = Vector::new(0.65, 4.0);
    pub fn new(translation: Vector<f32>, top_down: bool) -> Self {
        return Self {
            base: BaseComponent::new_body(
                RigidBodyBuilder::kinematic_velocity_based()
                    .translation(translation)
                    .linvel(Vector::new(Self::PIPE_SPEED, 0.0))
                    .rotation(if top_down { std::f32::consts::PI } else { 0.0 }),
                vec![ColliderBuilder::cuboid(Self::SIZE.x, Self::SIZE.y)],
            ),
        };
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(active: &ComponentPath<Self>, ctx: &mut Context) {
        let mut to_remove: Vec<ComponentHandle> = vec![];
        for pipe in ctx.path_mut(active) {
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

    fn render(active: &ComponentPath<Self>, ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_state::<BirdSimulation>();
        ctx.render_all(active, encoder, RenderConfig::default(), |r, instances| {
            r.render_sprite(instances, &scene.pipe_model, &scene.pipe_sprite)
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
        if layer_sizes.len() < 2 {
            panic!("Need at least 2 layers");
        }
        for &size in layer_sizes.iter() {
            if size < 1 {
                panic!("Empty layers not allowed");
            }
        }

        let mut layers = Vec::new();
        let first_layer_size = *layer_sizes.first().unwrap();
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
        if inputs.len() != self.inputs {
            panic!("Bad input size");
        }

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
