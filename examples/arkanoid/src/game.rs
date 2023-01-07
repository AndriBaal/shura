use shura::{audio::*, physics::*, text::*, *};

pub struct GameScene {
    target_model: Model,
    target_color: Uniform<Color>,

    statistics: ComponentHandle,
    player: ComponentHandle,
    ball: ComponentHandle,
}

impl GameScene {
    pub fn new(ctx: &mut Context) -> Self {
        let field = GameField::new(ctx);
        ctx.create_component(None, field);

        let window_size = ctx.window_size();
        let statistics = Statistics::new(ctx);
        let (s, statistics) = ctx.create_component(None, statistics);
        s.scale_relative_width(window_size);

        let player = Player::new(ctx);
        let player = ctx.create_component(None, player).1;

        let ball = Ball::new(ctx);
        let ball = ctx.create_component(None, ball).1;

        let target_model = ctx.create_model(ModelBuilder::cuboid(Target::HALF_TARGET_SIZE));
        let target_color = ctx.create_uniform(Color::new_rgba(0, 0, 255, 255));

        GameScene {
            target_model,
            target_color,
            statistics,
            player,
            ball,
        }
    }

    fn reset(&self, ctx: &mut Context) {
        let ball = ctx
            .component_manager
            .component_mut::<Ball>(&self.ball)
            .unwrap();
        let ball_body = ball.body_mut(ctx.world);
        ball_body.set_translation(Ball::START_POS, true);
        ball_body.set_linvel(Vector::new(Ball::LINVEL, Ball::LINVEL), true);

        let player = ctx
            .component_manager
            .component_mut::<Player>(&self.player)
            .unwrap();
        let player_body = player.body_mut(ctx.world);
        player_body.set_translation(Player::START_POS, true);

        let statistics = ctx.component_mut::<Statistics>(&self.statistics).unwrap();
        statistics.reset_score();

        ctx.remove_components::<Target>(None);
        self.spawn_targets(ctx);
    }

    fn spawn_targets(&self, ctx: &mut Context) {
        for x in (-2..3).step_by(1) {
            for y in (2..7).step_by(1) {
                let target = Target::new(Vector::new(x as f32, y as f32 / 2.0));
                ctx.create_component(None, target);
            }
        }
    }
}

impl SceneController for GameScene {
    fn update(&mut self, ctx: &mut Context) {
        if ctx.scene_switched() {
            self.reset(ctx);
        }

        if ctx.resized() {
            let window_size = ctx.window_size();
            if window_size.width > window_size.height {
                ctx.set_vertical_fov(GameField::FIELD_SIZE.height);
            } else {
                ctx.set_horizontal_fov(GameField::FIELD_SIZE.width);
            }
        }

        if ctx.components::<Target>(None).len() == 0 {
            self.spawn_targets(ctx);
        }
    }
}

#[derive(Component)]
struct GameField {
    #[component]
    component: PhysicsComponent,
    model: Model,
    background: Uniform<Color>,
}

impl GameField {
    const LEFT: u128 = 0;
    const RIGHT: u128 = 1;
    const TOP: u128 = 2;
    const BOTTOM: u128 = 3;
    const FIELD_SIZE: Dimension<f32> = Dimension::new(5.0, 8.0);
    const HALF_FIELD_SIZE: Dimension<f32> = Dimension::new(2.5, 4.0);
    fn new(ctx: &mut Context) -> Self {
        let top_right = Self::HALF_FIELD_SIZE.into();
        let bottom_right = Point::new(Self::HALF_FIELD_SIZE.width, -Self::HALF_FIELD_SIZE.height);
        let bottom_left = (-Self::HALF_FIELD_SIZE).into();
        let top_left = Point::new(-Self::HALF_FIELD_SIZE.width, Self::HALF_FIELD_SIZE.height);

        return GameField {
            component: PhysicsComponent::new(
                RigidBodyBuilder::fixed(),
                vec![
                    ColliderBuilder::segment(top_left, top_right).user_data(Self::TOP),
                    ColliderBuilder::segment(top_right, bottom_right).user_data(Self::RIGHT),
                    ColliderBuilder::segment(bottom_right, bottom_left).user_data(Self::BOTTOM),
                    ColliderBuilder::segment(bottom_left, top_left).user_data(Self::LEFT),
                ],
            ),
            model: ctx.create_model(ModelBuilder::cuboid(Self::HALF_FIELD_SIZE)),
            background: ctx.create_uniform(Color::new_rgba(02, 02, 02, 255)),
        };
    }
}

impl ComponentController for GameField {
    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        renderer.render_color(&self.model, &self.background);
        renderer.commit(&instance);
    }

    fn config() -> &'static ComponentConfig {
        static CONFIG: ComponentConfig = ComponentConfig {
            priority: 1,
            does_move: false,
            ..ComponentConfig::default()
        };
        return &CONFIG;
    }
}

#[derive(Component)]
struct Statistics {
    model: Model,
    text: Sprite,
    #[component]
    component: PositionComponent,
    score: u32,
    highscore: u32,
}

impl Statistics {
    fn new(ctx: &Context) -> Self {
        let mut component = PositionComponent::new();
        const HALF_MODEL_SIZE: Dimension<f32> = Dimension::new(0.13, 0.065);
        component.set_translation(Vector::new(-0.5, 0.5));
        Self {
            component,
            model: ctx.create_model(ModelBuilder::cuboid(HALF_MODEL_SIZE).translation(Vector::new(
                HALF_MODEL_SIZE.width,
                -HALF_MODEL_SIZE.height,
            ))),
            text: ctx.create_empty_sprite(Dimension::new(1, 1)),
            score: 0,
            highscore: 0,
        }
    }

    fn increase_score(&mut self) {
        self.score += 1;
        if self.score > self.highscore {
            self.highscore += 1;
        }
    }

    fn reset_score(&mut self) {
        self.score = 0;
    }
}

impl ComponentController for Statistics {
    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        if ctx.resized() {
            ctx.force_buffer_active::<Self>();
            self.component.scale_relative_width(ctx.window_size());
        }

        const SCALE: f32 = 70.0;
        const TEXT_COLOR: Color = Color::new(1.0, 1.0, 1.0, 1.0);
        self.text.write_text(
            ctx,
            TextDescriptor {
                font: None,
                size: Dimension::new(500, 250),
                clear_color: Some(Color::TRANSPARENT),
                sections: vec![
                    TextSection {
                        position: Vector::new(0.0, 0.0),
                        text: vec![Text::new(&format!("FPS: {}", ctx.fps()))
                            .with_scale(SCALE)
                            .with_color(TEXT_COLOR)],
                        ..TextSection::default()
                    },
                    TextSection {
                        position: Vector::new(0.0, 80.0),
                        text: vec![Text::new(&format!("Score: {}", self.score))
                            .with_scale(SCALE)
                            .with_color(TEXT_COLOR)],
                        ..TextSection::default()
                    },
                    TextSection {
                        position: Vector::new(0.0, 160.0),
                        text: vec![Text::new(&format!("Highscore: {}", self.highscore))
                            .with_scale(SCALE)
                            .with_color(TEXT_COLOR)],
                        ..TextSection::default()
                    },
                ],
            },
        );
    }

    fn render<'a>(
        &'a self,
        _scene: &DynamicScene,
        renderer: &mut Renderer<'a>,
        instances: Instances,
    ) {
        renderer.render_sprite(&self.model, &self.text);
        renderer.commit(&instances);
    }

    fn config() -> &'static ComponentConfig {
        static CONFIG: ComponentConfig = ComponentConfig {
            priority: 1000,
            does_move: false,
            camera: CameraUse::Relative,
            ..ComponentConfig::default()
        };
        return &CONFIG;
    }
}

#[derive(Component)]
struct Player {
    model: Model,
    #[component]
    component: PhysicsComponent,
}
impl Player {
    const HALF_SIZE: Dimension<f32> = Dimension::new(0.3, 0.04);
    const START_POS: Vector<f32> = Vector::new(0.0, -2.2);
    const LINVEL: f32 = 1.9;
    fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::cuboid(Self::HALF_SIZE)),
            component: PhysicsComponent::new(
                RigidBodyBuilder::dynamic()
                    .translation(Self::START_POS)
                    .lock_rotations()
                    .enabled_translations(true, false),
                vec![ColliderBuilder::cuboid(
                    Self::HALF_SIZE.width,
                    Self::HALF_SIZE.height,
                )],
            ),
        }
    }
}

impl ComponentController for Player {
    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        ctx: &mut Renderer<'a>,
        instances: Instances,
    ) {
        ctx.render_rainbow(&self.model);
        ctx.commit(&instances);
    }

    fn update(&mut self, _scene: &mut DynamicScene, ctx: &mut Context) {
        let body = self.component.body_mut(ctx.world);
        let mut linvel = *body.linvel();
        let translation = body.translation();
        if ctx.input.is_held(Key::A) || ctx.input.is_held(Key::Left) {
            linvel.x = -Self::LINVEL;
        } else if ctx.input.is_held(Key::D) || ctx.input.is_held(Key::Left) {
            linvel.x = Self::LINVEL;
        } else {
            linvel.x = 0.0;
        }

        if ctx.input.is_held(ScreenTouch) || ctx.input.is_held(MouseButton::Left) {
            let cursor_pos = ctx.cursor.cursor_world();
            if cursor_pos.x > translation.x {
                linvel.x = Self::LINVEL;
            } else {
                linvel.x = -Self::LINVEL;
            }
        }
        body.set_linvel(linvel, true);
    }
}

#[derive(Component)]
struct Target {
    #[component]
    component: PhysicsComponent,
}

impl Target {
    const HALF_TARGET_SIZE: Dimension<f32> = Dimension::new(0.24, 0.08);
    fn new(start_pos: Vector<f32>) -> Self {
        Target {
            component: PhysicsComponent::new(
                RigidBodyBuilder::fixed().translation(start_pos),
                vec![ColliderBuilder::cuboid(
                    Self::HALF_TARGET_SIZE.width,
                    Self::HALF_TARGET_SIZE.height,
                )],
            ),
        }
    }
}
impl ComponentController for Target {
    fn render_grouped<'a>(
        scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        _components: ComponentSet<DynamicComponent>,
        instances: Instances,
    ) where
        Self: Sized,
    {
        let game_scene = scene.downcast_ref::<GameScene>().unwrap();
        renderer.render_color(&game_scene.target_model, &game_scene.target_color);
        renderer.commit(&instances);
    }

    fn config() -> &'static ComponentConfig {
        static CONFIG: ComponentConfig = ComponentConfig {
            render: RenderOperation::Grouped,
            ..ComponentConfig::default()
        };
        return &CONFIG;
    }
}

#[derive(Component)]
struct Ball {
    model: Model,
    color: Uniform<Color>,
    #[component]
    component: PhysicsComponent,
    sink: Sink,
    bounce: Sound,
    game_over: Sound,
}
impl Ball {
    const RADIUS: f32 = 0.1;
    const LINVEL: f32 = 2.0;
    const START_POS: Vector<f32> = Vector::new(0.0, 0.0);
    fn new(ctx: &Context) -> Self {
        Self {
            model: ctx.create_model(ModelBuilder::ball(Self::RADIUS, 24)),
            color: ctx.create_uniform(Color::new_rgba(0, 255, 0, 255)),
            sink: ctx.create_sink(),
            bounce: ctx.create_sound(include_bytes!("../res/bounce.wav")),
            game_over: ctx.create_sound(include_bytes!("../res/game_over.wav")),
            component: PhysicsComponent::new(
                RigidBodyBuilder::dynamic()
                    .lock_rotations()
                    .translation(Self::START_POS)
                    .linvel(Vector::new(Self::LINVEL, Self::LINVEL)),
                vec![ColliderBuilder::ball(Self::RADIUS)
                    .restitution(5.0)
                    .active_events(ActiveEvents::COLLISION_EVENTS)],
            ),
        }
    }
}
impl ComponentController for Ball {
    fn render<'a>(
        &'a self,
        _scene: &'a DynamicScene,
        renderer: &mut Renderer<'a>,
        instance: Instances,
    ) {
        renderer.render_color(&self.model, &self.color);
        renderer.commit(&instance);
    }

    fn collision(
        &mut self,
        scene: &mut DynamicScene,
        ctx: &mut Context,
        other_handle: ComponentHandle,
        _self_collider: ColliderHandle,
        other_collider: ColliderHandle,
        collide_type: CollideType,
    ) {
        if collide_type == CollideType::Started {
            let body = self.component.body(ctx.world);
            let pos = *body.translation();
            let mut linvel = *body.linvel();
            let other = ctx
                .component_manager
                .component_dynamic_mut(&other_handle)
                .unwrap();

            if let Some(target) = other.downcast_mut::<Target>() {
                let target_body = target.component.body_mut(ctx.world);
                let target_pos = target_body.translation();

                if pos.y < target_pos.y - Target::HALF_TARGET_SIZE.height {
                    linvel.y = -Ball::LINVEL;
                } else if pos.y > target_pos.y + Target::HALF_TARGET_SIZE.height {
                    linvel.y = Ball::LINVEL;
                }

                if pos.x < target_pos.x - Target::HALF_TARGET_SIZE.width {
                    linvel.x = -Ball::LINVEL;
                } else if pos.x > target_pos.x + Target::HALF_TARGET_SIZE.width {
                    linvel.x = Ball::LINVEL;
                }

                ctx.remove_component(&other_handle);

                let scene = scene.downcast_ref::<GameScene>().unwrap();
                let statistics = ctx.component_mut::<Statistics>(&scene.statistics).unwrap();
                statistics.increase_score();
            } else if let Some(_) = other.downcast_mut::<GameField>() {
                let collider = ctx.collider(other_collider).unwrap();
                match collider.user_data {
                    GameField::LEFT => {
                        linvel.x = Ball::LINVEL;
                    }
                    GameField::RIGHT => {
                        linvel.x = -Ball::LINVEL;
                    }
                    GameField::TOP => {
                        linvel.y = -Ball::LINVEL;
                    }
                    GameField::BOTTOM => {
                        self.sink.append(self.game_over.decode());
                        ctx.scene_manager.set_active_scene("menu");
                        return;
                    }
                    _ => {}
                }
            } else if let Some(player) = other.downcast_mut::<Player>() {
                let player_body = player.component.body_mut(ctx.world);
                let player_pos = player_body.translation();
                if pos.y < player_pos.y - Target::HALF_TARGET_SIZE.height {
                    linvel.y = -Ball::LINVEL;
                } else if pos.y > player_pos.y + Target::HALF_TARGET_SIZE.height {
                    linvel.y = Ball::LINVEL;
                }

                if pos.x < player_pos.x - Target::HALF_TARGET_SIZE.width {
                    linvel.x = -Ball::LINVEL;
                } else if pos.x > player_pos.x + Target::HALF_TARGET_SIZE.width {
                    linvel.x = Ball::LINVEL;
                }
            }
            self.sink.append(self.bounce.decode());
            self.component.body_mut(ctx.world).set_linvel(linvel, true);
        }
    }
}
