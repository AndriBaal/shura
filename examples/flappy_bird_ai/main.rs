use shura::{
    log::info,
    physics::{parry::bounding_volume::BoundingVolume, Aabb, LockedAxes, RigidBodyBuilder},
    rand::{
        distributions::{Distribution, WeightedIndex},
        gen_range, thread_rng,
    },
    winit::window::WindowButtons,
    *,
};

// Inspired by: https://github.com/bones-ai/rust-flappy-bird-ai

const GAME_SIZE: Vector<f32> = Vector::new(11.25, 5.0);
const AMOUNT_BIRDS: u32 = 1000;

#[shura::main]
fn shura_main(config: ShuraConfig) {
    config.init(NewScene::new(1, |ctx| {
        register!(ctx, [Background, Ground, Pipe, Bird]);
        ctx.world_camera
            .set_scaling(WorldCameraScale::Vertical(GAME_SIZE.y));
        ctx.scene_states.insert(BirdSimulation::new(ctx));
        ctx.components
            .add(GroupHandle::DEFAULT_GROUP, Background::new(ctx));
        ctx.components
            .add(GroupHandle::DEFAULT_GROUP, Ground::new(ctx));
        ctx.window.set_resizable(false);
        ctx.window.set_enabled_buttons(WindowButtons::CLOSE);
        for _ in 0..AMOUNT_BIRDS {
            ctx.components.add(GroupHandle::DEFAULT_GROUP, Bird::new());
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
    time_scale: f32,
}

impl BirdSimulation {
    pub fn new(ctx: &Context) -> Self {
        return Self {
            bird_model: ctx
                .gpu
                .create_model(ModelBuilder::cuboid(Bird::HALF_EXTENTS)),
            bird_sprite: ctx
                .gpu
                .create_sprite(include_bytes!("./sprites/yellowbird-downflap.png")),
            top_pipe_model: ctx.gpu.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation::new(180.0_f32.to_radians())),
            ),
            bottom_pipe_model: ctx.gpu.create_model(
                ModelBuilder::cuboid(Pipe::HALF_EXTENTS).vertex_translation(Vector::new(
                    0.0,
                    -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                )),
            ),
            pipe_sprite: ctx
                .gpu
                .create_sprite(include_bytes!("./sprites/pipe-green.png")),
            spawn_timer: Pipe::SPAWN_TIME,
            generation: 0,
            high_score: 0,
            time_scale: 1.0,
        };
    }

    fn spawn_pipes(&mut self, components: &mut ComponentManager) {
        self.spawn_timer = 0.0;
        components.add(GroupHandle::DEFAULT_GROUP, Pipe::new());
    }
}

impl SceneStateController for BirdSimulation {
    fn update(ctx: &mut Context) {
        let fps = ctx.frame.fps();
        let scene = ctx.scene_states.get_mut::<Self>();
        let delta = ctx.frame.frame_time() * scene.time_scale;
        scene.spawn_timer += delta;
        let score = ctx
            .components
            .iter::<Bird>(ComponentFilter::All)
            .find(|b| !b.dead)
            .unwrap()
            .score as u32;
        if score > scene.high_score {
            scene.high_score = score;
        }

        if scene.spawn_timer >= Pipe::SPAWN_TIME {
            scene.spawn_pipes(ctx.components);
        }

        let pipes = ctx.components.iter::<Pipe>(ComponentFilter::All);
        let mut closest = Vector::new(GAME_SIZE.x, 0.0);
        for pipe in pipes {
            let translation = pipe.pos.translation();
            if translation.x >= 0.0 && translation.x < closest.x {
                closest = translation;
            }
        }

        let bottom_y = closest.y - Pipe::HALF_HOLE_SIZE;
        let top_y = closest.y + Pipe::HALF_HOLE_SIZE;

        let top_aabb = AABB::from_center(
            closest + Vector::new(0.0, Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y),
            Pipe::HALF_EXTENTS,
        );
        let bottom_aabb = AABB::from_center(
            closest - Vector::new(0.0, Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y),
            Pipe::HALF_EXTENTS,
        );
        let x = closest.x;
        assert!(x >= 0.0);

        for bird in ctx.components.iter_mut::<Bird>(ComponentFilter::Active) {
            bird.linvel += delta * Bird::GRAVITY;
            let new_pos = bird.pos.translation() + delta * bird.linvel;
            bird.pos.set_translation(new_pos);

            let bird_aabb = AABB::from_center(bird.pos.translation(), Bird::HALF_EXTENTS);
            if bird_aabb.min.y < -GAME_SIZE.y + Ground::HALF_EXTENTS.y * 2.0
                || bird_aabb.max.y > GAME_SIZE.y
                || bird_aabb.intersects(&bottom_aabb)
                || bird_aabb.intersects(&top_aabb)
            {
                bird.dead = true;
            }

            if bird.dead {
                continue;
            }

            bird.score += delta * 1.0;
            let out = bird.brain.predict(&vec![
                bird.pos.translation().y as f64,
                bottom_y as f64,
                top_y as f64,
                x as f64,
                bird.linvel.y as f64,
            ])[0];

            if out >= 0.5 {
                bird.linvel.y = 5.0;
            }
        }

        let dead_count = ctx
            .components
            .iter::<Bird>(ComponentFilter::All)
            .filter(|b| !b.dead)
            .count();

        if dead_count == 0 {
            let mut max_fitness = 0.0;
            let mut weights = Vec::new();

            for b in ctx.components.iter::<Bird>(ComponentFilter::All) {
                if b.score > max_fitness {
                    max_fitness = b.score;
                }
                weights.push(b.score);
            }
            weights
                .iter_mut()
                .for_each(|i| *i = (*i / max_fitness) * 100.0);

            let gene_pool = WeightedIndex::new(&weights).expect("Failed to generate gene pool");

            let amount = ctx.components.len::<Bird>(ComponentFilter::All);
            let mut rng = thread_rng();
            let mut new_birds = Vec::with_capacity(amount);
            for _ in 0..amount {
                let index = gene_pool.sample(&mut rng);
                let rand_bird = ctx
                    .components
                    .index_mut::<Bird>(GroupHandle::DEFAULT_GROUP, index)
                    .unwrap();

                let mut new_bird = Bird::with_brain(&rand_bird);
                new_bird.brain.mutate();
                new_birds.push(new_bird);
            }
            ctx.components.remove_all::<Bird>(ComponentFilter::All);
            ctx.components
                .add_many(GroupHandle::DEFAULT_GROUP, new_birds);

            scene.generation += 1;
            info!("Now at generation {}!", scene.generation);
            ctx.components
                .remove_all::<Pipe>(ComponentFilter::All);
            scene.spawn_pipes(ctx.components);
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
                ui.add(gui::Slider::new(&mut scene.time_scale, 0.1..=20.0).text("Speed"));
            });
    }
}

#[derive(Component)]
struct Bird {
    #[base]
    pos: PositionComponent,
    brain: NeuralNetwork,
    score: f32,
    dead: bool,
    linvel: Vector<f32>,
}

impl Bird {
    const HALF_EXTENTS: Vector<f32> = Vector::new(0.3, 0.21176472);
    const GRAVITY: Vector<f32> = Vector::new(0.0, -15.0);
    pub fn new() -> Self {
        Self {
            pos: PositionComponent::new(Default::default()),
            score: 0.0,
            brain: NeuralNetwork::new(vec![5, 8, 1]),
            dead: false,
            linvel: Vector::default(),
        }
    }

    pub fn with_brain(other: &Bird) -> Self {
        let mut new_bird = Bird::new();
        new_bird.brain = other.brain.clone();
        new_bird
    }
}

impl ComponentController for Bird {
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_states.get::<BirdSimulation>();
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, bird, instance| {
            if !bird.dead {
                r.render_sprite(instance, &scene.bird_model, &scene.bird_sprite)
            }
        });
    }
}

#[derive(Component)]
struct Ground {
    model: Model,
    sprite: Sprite,
    #[base]
    pos: PositionComponent,
}

impl Ground {
    const HALF_EXTENTS: Vector<f32> = Vector::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new(ctx: &Context) -> Self {
        Self {
            model: ctx
                .gpu
                .create_model(ModelBuilder::cuboid(Self::HALF_EXTENTS)),
            sprite: ctx.gpu.create_sprite(include_bytes!("./sprites/base.png")),
            pos: PositionComponent::new(
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
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, ground, instance| {
            r.render_sprite(instance, &ground.model, &ground.sprite)
        });
    }
}

#[derive(Component)]
struct Background {
    model: Model,
    sprite: Sprite,
    #[base]
    pos: PositionComponent,
}

impl Background {
    pub fn new(ctx: &Context) -> Self {
        let sprite = ctx
            .gpu
            .create_sprite(include_bytes!("./sprites/background-night.png"));
        Self {
            model: ctx.gpu.create_model(ModelBuilder::cuboid(GAME_SIZE)),
            sprite,
            pos: PositionComponent::default(),
        }
    }
}

impl ComponentController for Background {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 1,
        buffer: BufferOperation::Manual,
        ..DEFAULT_CONFIG
    };
    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        encoder.render_each::<Self>(ctx, RenderConfig::default(), |r, background, instance| {
            r.render_sprite(instance, &background.model, &background.sprite)
        });
    }
}

#[derive(Component)]
struct Pipe {
    #[base]
    pos: PositionComponent,
}

impl Pipe {
    const PIPE_VEL: Vector<f32> = Vector::new(-3.0, 0.0);
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
            pos: PositionComponent::new(
                PositionBuilder::new().translation(Vector::new(GAME_SIZE.x, y)),
            ),
        };
    }
}

impl ComponentController for Pipe {
    const CONFIG: ComponentConfig = ComponentConfig {
        priority: 3,
        ..DEFAULT_CONFIG
    };
    fn update(ctx: &mut Context) {
        let scene = ctx.scene_states.get_mut::<BirdSimulation>();
        let step = ctx.frame.frame_time() * scene.time_scale * Self::PIPE_VEL;
        ctx.components
            .retain::<Self>(ComponentFilter::Active, |pipe| {
                let new_pos = pipe.pos.translation() + step;
                pipe.pos.set_translation(new_pos);
                if new_pos.x <= -GAME_SIZE.x {
                    return false;
                }
                return true;
            });
    }

    fn render(ctx: &Context, encoder: &mut RenderEncoder) {
        let scene = ctx.scene_states.get::<BirdSimulation>();
        encoder.render_all::<Self>(ctx, RenderConfig::default(), |r, instances| {
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
