use shura::{
    prelude::*,
    random::rand::{
        distributions::{Distribution, WeightedIndex},
        thread_rng,
    },
    winit::window::WindowButtons,
};

const GAME_SIZE: Vector2<f32> = Vector2::new(11.25, 5.0);
const AMOUNT_BIRDS: u32 = 1000;

#[shura::main]
fn app(config: AppConfig) {
    App::run(config, || {
        Scene::new()
            .render_group2d("ground", RenderGroupUpdate::MANUAL)
            .render_group2d("background", RenderGroupUpdate::MANUAL)
            .render_group2d("pipe", RenderGroupUpdate::EVERY_FRAME)
            .render_group2d("bird", RenderGroupUpdate::EVERY_FRAME)
            .entity_single::<Background>()
            .entity_single::<Ground>()
            .entity_single::<BirdSimulation>()
            .entity::<Pipe>()
            .entity::<Bird>()
            .system(System::update(update))
            .system(System::setup(setup))
            .system(System::render(render))
    });
}

fn setup(ctx: &mut Context) {
    ctx.world_camera2d
        .set_scaling(WorldCameraScaling::Vertical(GAME_SIZE.y));
    ctx.entities
        .single()
        .set(ctx.world, BirdSimulation::new(ctx));
    ctx.entities.single().set(ctx.world, Background::new());
    ctx.entities.single().set(ctx.world, Ground::new());
    ctx.window.set_resizable(false);
    ctx.screen_config.set_vsync(false);
    ctx.screen_config.set_render_scale(0.5);
    ctx.window.set_enabled_buttons(WindowButtons::CLOSE);
    let mut birds = ctx.entities.get_mut::<Bird>();
    for _ in 0..AMOUNT_BIRDS {
        birds.add(ctx.world, Bird::new());
    }
}

fn update(ctx: &mut Context) {
    let mut pipes = ctx.entities.get_mut::<Pipe>();
    let mut simulation = ctx.entities.single_mut::<BirdSimulation>().unwrap();
    let mut birds = ctx.entities.get_mut::<Bird>();
    let fps = ctx.time.fps();
    let delta = ctx.time.delta() * simulation.time_scale;
    let step = ctx.time.delta() * simulation.time_scale * Pipe::VELOCITY;
    pipes.retain(ctx.world, |pipe, _| {
        let new_pos = pipe.pos.translation() + step;
        pipe.pos.set_translation(new_pos);
        if new_pos.x <= -GAME_SIZE.x {
            return false;
        }
        return true;
    });

    simulation.spawn_timer += delta;
    let score = birds
        .iter()
        .find(|b| b.pos.visibility == PositionComponent2DVisibility::Static(true))
        .unwrap()
        .score as u32;
    if score > simulation.high_score {
        simulation.high_score = score;
    }

    if simulation.spawn_timer >= Pipe::SPAWN_TIME {
        simulation.spawn_pipes(ctx.world, &mut pipes);
    }

    let mut closest = Vector2::new(GAME_SIZE.x, 0.0);
    for pipe in pipes.iter() {
        let translation = pipe.pos.translation();
        if translation.x >= 0.0 && translation.x < closest.x {
            closest = translation;
        }
    }

    let bottom_y = closest.y - Pipe::HALF_HOLE_SIZE;
    let top_y = closest.y + Pipe::HALF_HOLE_SIZE;

    let top_aabb = AABB::from_center(
        closest + Vector2::new(0.0, Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y),
        Pipe::HALF_EXTENTS,
    );
    let bottom_aabb = AABB::from_center(
        closest - Vector2::new(0.0, Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y),
        Pipe::HALF_EXTENTS,
    );
    let x = closest.x;
    assert!(x >= 0.0);

    birds.par_iter_mut().for_each(|bird| {
        bird.linvel += delta * Bird::GRAVITY;
        let new_pos = bird.pos.translation() + delta * bird.linvel;
        bird.pos.set_translation(new_pos);

        let bird_aabb = AABB::from_center(bird.pos.translation(), Bird::HALF_EXTENTS);
        if bird_aabb.min().y < -GAME_SIZE.y + Ground::HALF_EXTENTS.y * 2.0
            || bird_aabb.max().y > GAME_SIZE.y
            || bird_aabb.intersects(&bottom_aabb)
            || bird_aabb.intersects(&top_aabb)
        {
            bird.pos
                .set_visibility(PositionComponent2DVisibility::Static(false));
        }

        if bird.pos.visibility == PositionComponent2DVisibility::Static(false) {
            return;
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
    });

    let dead_count = birds
        .iter()
        .filter(|b| b.pos.visibility == PositionComponent2DVisibility::Static(true))
        .count();

    if dead_count == 0 {
        let mut max_fitness = 0.0;
        let mut weights = Vec::new();

        for bird in birds.iter_mut() {
            if bird.score > max_fitness {
                max_fitness = bird.score;
            }
            weights.push(bird.score);
        }
        weights
            .iter_mut()
            .for_each(|i| *i = (*i / max_fitness) * 100.0);

        let gene_pool = WeightedIndex::new(&weights)
            .expect(&format!("Failed to generate gene pool, {delta:?}"));

        let amount = birds.len();
        let mut rng = thread_rng();
        let mut new_birds = Vec::with_capacity(amount);
        for _ in 0..amount {
            let instances = gene_pool.sample(&mut rng);
            let rand_bird = birds.index_mut(instances).unwrap();

            let mut new_bird = Bird::with_brain(&rand_bird);
            new_bird.brain.mutate();
            new_birds.push(new_bird);
        }
        birds.remove_all(ctx.world);
        birds.add_many(ctx.world, new_birds);

        simulation.generation += 1;
        info!("Now at generation {}!", simulation.generation);
        pipes.remove_all(ctx.world);
        simulation.spawn_pipes(ctx.world, &mut pipes);
    }

    gui::Window::new("Flappy Bird")
        .anchor(gui::Align2::RIGHT_TOP, gui::Vec2::default())
        .resizable(false)
        .collapsible(false)
        .show(&ctx.gui.clone(), |ui| {
            ui.label(&format!("FPS: {}", fps));
            ui.label(format!("Generation: {}", simulation.generation));
            ui.label(format!("Score: {}", score));
            ui.label(format!("High Score: {}", simulation.high_score));
            ui.label(format!("Birds: {}", dead_count as u32));
            ui.add(gui::Slider::new(&mut simulation.time_scale, 0.1..=20.0).text("Speed"));
        });
}

fn render(ctx: &RenderContext, encoder: &mut RenderEncoder) {
    let simulation = ctx.entities.single::<BirdSimulation>().get().unwrap();
    encoder.render2d(
        Some(RgbaColor::new(220, 220, 220, 255).into()),
        |renderer| {
            ctx.group("background", |buffer| {
                renderer.draw_sprite(
                    buffer,
                    ctx.world_camera2d,
                    ctx.unit_mesh,
                    &simulation.background_sprite,
                )
            });

            ctx.group("ground", |buffer| {
                renderer.draw_sprite(
                    buffer,
                    ctx.world_camera2d,
                    ctx.unit_mesh,
                    &simulation.ground_sprite,
                )
            });
            ctx.group("pipe", |buffer| {
                renderer.draw_sprite(
                    buffer,
                    ctx.world_camera2d,
                    &simulation.top_pipe_mesh,
                    &simulation.pipe_sprite,
                );
                renderer.draw_sprite(
                    buffer,
                    ctx.world_camera2d,
                    &simulation.bottom_pipe_mesh,
                    &simulation.pipe_sprite,
                );
            });

            ctx.group("bird", |buffer| {
                renderer.draw_sprite(
                    buffer,
                    ctx.world_camera2d,
                    &simulation.bird_mesh,
                    &simulation.bird_sprite,
                )
            });
        },
    );
}

#[derive(Entity)]
struct BirdSimulation {
    bird_mesh: Mesh2D,
    top_pipe_mesh: Mesh2D,
    bottom_pipe_mesh: Mesh2D,
    pipe_sprite: Sprite,
    bird_sprite: Sprite,
    ground_sprite: Sprite,
    background_sprite: Sprite,
    spawn_timer: f32,
    generation: u32,
    high_score: u32,
    time_scale: f32,
}

impl BirdSimulation {
    pub fn new(ctx: &Context) -> Self {
        return Self {
            bird_mesh: ctx
                .gpu
                .create_mesh(&MeshBuilder2D::cuboid(Bird::HALF_EXTENTS)),
            bird_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "flappy_bird/sprites/yellowbird-downflap.png"
                ))),
            ground_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "flappy_bird/sprites/base.png"
                ))),
            background_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "flappy_bird/sprites/background-night.png"
                ))),
            top_pipe_mesh: ctx.gpu.create_mesh(
                &MeshBuilder2D::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector2::new(
                        0.0,
                        Pipe::HALF_HOLE_SIZE + Pipe::HALF_EXTENTS.y,
                    ))
                    .tex_coord_rotation(Rotation2::new(180.0_f32.to_radians()))
                    .apply(),
            ),
            bottom_pipe_mesh: ctx.gpu.create_mesh(
                &MeshBuilder2D::cuboid(Pipe::HALF_EXTENTS)
                    .vertex_translation(Vector2::new(
                        0.0,
                        -Pipe::HALF_HOLE_SIZE - Pipe::HALF_EXTENTS.y,
                    ))
                    .apply(),
            ),
            pipe_sprite: ctx
                .gpu
                .create_sprite(SpriteBuilder::bytes(include_asset_bytes!(
                    "flappy_bird/sprites/pipe-green.png",
                ))),
            spawn_timer: Pipe::SPAWN_TIME,
            generation: 0,
            high_score: 0,
            time_scale: 1.0,
        };
    }

    fn spawn_pipes(&mut self, world: &mut World, pipes: &mut Entities<Pipe>) {
        self.spawn_timer = 0.0;
        pipes.add(world, Pipe::new());
    }
}

#[derive(Entity)]
struct Bird {
    #[shura(component = "bird")]
    pos: PositionComponent2D,
    brain: NeuralNetwork,
    score: f32,
    linvel: Vector2<f32>,
}

impl Bird {
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(0.3, 0.21176472);
    const GRAVITY: Vector2<f32> = Vector2::new(0.0, -15.0);
    pub fn new() -> Self {
        Self {
            pos: PositionComponent2D::new(),
            score: 0.0,
            brain: NeuralNetwork::new(vec![5, 8, 1]),
            linvel: Vector2::default(),
        }
    }

    pub fn with_brain(other: &Bird) -> Self {
        let mut new_bird = Bird::new();
        new_bird.brain = other.brain.clone();
        new_bird
    }
}

#[derive(Entity)]
struct Ground {
    #[shura(component = "ground")]
    pos: PositionComponent2D,
}

impl Ground {
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(GAME_SIZE.data.0[0][0], 0.9375);
    pub fn new() -> Self {
        Self {
            pos: PositionComponent2D::new()
                .with_translation(Vector2::new(0.0, -GAME_SIZE.y + Self::HALF_EXTENTS.y))
                .with_scaling(Self::HALF_EXTENTS * 2.0),
        }
    }
}

#[derive(Entity)]
struct Background {
    #[shura(component = "background")]
    pos: PositionComponent2D,
}

impl Background {
    pub fn new() -> Self {
        Self {
            pos: PositionComponent2D::default().with_scaling(GAME_SIZE * 2.0),
        }
    }
}

#[derive(Entity)]
struct Pipe {
    #[shura(component = "pipe")]
    pos: PositionComponent2D,
}

impl Pipe {
    const VELOCITY: Vector2<f32> = Vector2::new(-3.0, 0.0);
    const HALF_EXTENTS: Vector2<f32> = Vector2::new(0.65, 4.0);
    const HALF_HOLE_SIZE: f32 = 1.1;
    const MIN_PIPE_Y: f32 = 0.25;
    const SPAWN_TIME: f32 = 3.0;
    pub fn new() -> Self {
        let y = gen_range(
            -GAME_SIZE.y + Self::MIN_PIPE_Y + Pipe::HALF_HOLE_SIZE + Ground::HALF_EXTENTS.y * 2.0
                ..GAME_SIZE.y - Self::MIN_PIPE_Y - Pipe::HALF_HOLE_SIZE,
        );
        return Self {
            pos: PositionComponent2D::new().with_translation(Vector2::new(GAME_SIZE.x, y)),
        };
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
