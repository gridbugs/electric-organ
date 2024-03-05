use crate::{
    colours,
    controls::{AppInput, Controls},
    game_instance::{GameInstance, GameInstanceStorable, Mode},
    image::Images,
    music::{MusicState, Track},
    text,
};
use chargrid::{self, border::BorderStyle, control_flow::*, menu, prelude::*};
use game::{
    witness::{self, FireEquipped, Running, Witness},
    Config as GameConfig, ExternalEvent, GameOverReason, Victory,
};
use general_storage_static::{self as storage, format, StaticStorage as Storage};
use line_2d;
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
use rgb_int::Rgb24;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    music_volume: f32,
    sfx_volume: f32,
    won: bool,
    first_run: bool,
    victories: Vec<Victory>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_volume: 0.2,
            sfx_volume: 0.5,
            won: false,
            first_run: true,
            victories: Vec::new(),
        }
    }
}

/// An interactive, renderable process yielding a value of type `T`
pub type AppCF<T> = CF<Option<T>, GameLoopData>;
pub type State = GameLoopData;

const MENU_BACKGROUND: Rgba32 = Rgba32::new_rgb(0, 0, 0);
const MENU_FADE_SPEC: menu::identifier::fade_spec::FadeSpec = {
    use menu::identifier::fade_spec::*;
    FadeSpec {
        on_select: Fade {
            to: To {
                rgba32: Layers {
                    foreground: colours::VAPORWAVE_BACKGROUND.to_rgba32(255),
                    background: colours::VAPORWAVE_FOREGROUND
                        .to_rgba32(255)
                        .saturating_scalar_mul_div(4, 3),
                },
                bold: false,
                underline: false,
            },
            from: From::current(),
            durations: Layers {
                foreground: Duration::from_millis(128),
                background: Duration::from_millis(128),
            },
        },
        on_deselect: Fade {
            to: To {
                rgba32: Layers {
                    foreground: colours::VAPORWAVE_FOREGROUND.to_rgba32(255),
                    background: colours::VAPORWAVE_BACKGROUND.to_rgba32(127),
                },
                bold: false,
                underline: false,
            },
            from: From::current(),
            durations: Layers {
                foreground: Duration::from_millis(128),
                background: Duration::from_millis(128),
            },
        },
    }
};

pub enum InitialRngSeed {
    U64(u64),
    Random,
}

struct RngSeedSource {
    next_seed: u64,
    seed_rng: Isaac64Rng,
}

impl RngSeedSource {
    fn new(initial_rng_seed: InitialRngSeed) -> Self {
        let mut seed_rng = Isaac64Rng::from_entropy();
        let next_seed = match initial_rng_seed {
            InitialRngSeed::U64(seed) => seed,
            InitialRngSeed::Random => seed_rng.gen(),
        };
        Self {
            next_seed,
            seed_rng,
        }
    }

    fn next_seed(&mut self) -> u64 {
        let seed = self.next_seed;
        self.next_seed = self.seed_rng.gen();
        #[cfg(feature = "print_stdout")]
        println!("RNG Seed: {}", seed);
        #[cfg(feature = "print_log")]
        log::info!("RNG Seed: {}", seed);
        seed
    }
}

pub struct AppStorage {
    pub handle: Storage,
    pub save_game_key: String,
    pub config_key: String,
    pub controls_key: String,
}

impl AppStorage {
    const SAVE_GAME_STORAGE_FORMAT: format::Bincode = format::Bincode;
    const CONFIG_STORAGE_FORMAT: format::JsonPretty = format::JsonPretty;
    const CONTROLS_STORAGE_FORMAT: format::JsonPretty = format::JsonPretty;

    fn save_game(&mut self, instance: &GameInstanceStorable) {
        let result = self.handle.store(
            &self.save_game_key,
            &instance,
            Self::SAVE_GAME_STORAGE_FORMAT,
        );
        if let Err(e) = result {
            use storage::{StoreError, StoreRawError};
            match e {
                StoreError::FormatError(e) => log::error!("Failed to format save file: {}", e),
                StoreError::Raw(e) => match e {
                    StoreRawError::IoError(e) => {
                        log::error!("Error while writing save data: {}", e)
                    }
                },
            }
        }
    }

    fn load_game(&self) -> Option<GameInstanceStorable> {
        let result = self.handle.load::<_, GameInstanceStorable, _>(
            &self.save_game_key,
            Self::SAVE_GAME_STORAGE_FORMAT,
        );
        match result {
            Err(e) => {
                use storage::{LoadError, LoadRawError};
                match e {
                    LoadError::FormatError(e) => log::error!("Failed to parse save file: {}", e),
                    LoadError::Raw(e) => match e {
                        LoadRawError::IoError(e) => {
                            log::error!("Error while reading save data: {}", e)
                        }
                        LoadRawError::NoSuchKey => (),
                    },
                }
                None
            }
            Ok(instance) => Some(instance),
        }
    }

    fn clear_game(&mut self) {
        if self.handle.exists(&self.save_game_key) {
            if let Err(e) = self.handle.remove(&self.save_game_key) {
                use storage::RemoveError;
                match e {
                    RemoveError::IoError(e) => {
                        log::error!("Error while removing data: {}", e)
                    }
                    RemoveError::NoSuchKey => (),
                }
            }
        }
    }

    fn save_config(&mut self, config: &Config) {
        let result = self
            .handle
            .store(&self.config_key, &config, Self::CONFIG_STORAGE_FORMAT);
        if let Err(e) = result {
            use storage::{StoreError, StoreRawError};
            match e {
                StoreError::FormatError(e) => log::error!("Failed to format config: {}", e),
                StoreError::Raw(e) => match e {
                    StoreRawError::IoError(e) => {
                        log::error!("Error while writing config: {}", e)
                    }
                },
            }
        }
    }

    fn load_config(&self) -> Option<Config> {
        let result = self
            .handle
            .load::<_, Config, _>(&self.config_key, Self::CONFIG_STORAGE_FORMAT);
        match result {
            Err(e) => {
                use storage::{LoadError, LoadRawError};
                match e {
                    LoadError::FormatError(e) => log::error!("Failed to parse config file: {}", e),
                    LoadError::Raw(e) => match e {
                        LoadRawError::IoError(e) => {
                            log::error!("Error while reading config: {}", e)
                        }
                        LoadRawError::NoSuchKey => (),
                    },
                }
                None
            }
            Ok(instance) => Some(instance),
        }
    }

    fn save_controls(&mut self, controls: &Controls) {
        let result =
            self.handle
                .store(&self.controls_key, &controls, Self::CONTROLS_STORAGE_FORMAT);
        if let Err(e) = result {
            use storage::{StoreError, StoreRawError};
            match e {
                StoreError::FormatError(e) => log::error!("Failed to format controls: {}", e),
                StoreError::Raw(e) => match e {
                    StoreRawError::IoError(e) => {
                        log::error!("Error while writing controls: {}", e)
                    }
                },
            }
        }
    }

    fn load_controls(&self) -> Option<Controls> {
        let result = self
            .handle
            .load::<_, Controls, _>(&self.controls_key, Self::CONTROLS_STORAGE_FORMAT);
        match result {
            Err(e) => {
                use storage::{LoadError, LoadRawError};
                match e {
                    LoadError::FormatError(e) => {
                        log::error!("Failed to parse controls file: {}", e)
                    }
                    LoadError::Raw(e) => match e {
                        LoadRawError::IoError(e) => {
                            log::error!("Error while reading controls: {}", e)
                        }
                        LoadRawError::NoSuchKey => (),
                    },
                }
                None
            }
            Ok(instance) => Some(instance),
        }
    }
}

fn new_game(
    rng_seed_source: &mut RngSeedSource,
    game_config: &GameConfig,
    victories: Vec<Victory>,
) -> (GameInstance, witness::Running) {
    let mut rng = Isaac64Rng::seed_from_u64(rng_seed_source.next_seed());
    GameInstance::new(game_config, victories, &mut rng)
}

pub struct GameLoopData {
    instance: Option<GameInstance>,
    controls: Controls,
    game_config: GameConfig,
    storage: AppStorage,
    rng_seed_source: RngSeedSource,
    config: Config,
    images: Images,
    cursor: Option<Coord>,
    music_state: MusicState,
}

impl GameLoopData {
    pub fn new(
        game_config: GameConfig,
        mut storage: AppStorage,
        initial_rng_seed: InitialRngSeed,
        force_new_game: bool,
        mute: bool,
    ) -> (Self, GameLoopState) {
        let mut rng_seed_source = RngSeedSource::new(initial_rng_seed);
        let config = storage.load_config().unwrap_or_default();
        let (instance, state) = match storage.load_game() {
            Some(instance) => {
                let (instance, running) = instance.into_game_instance();
                (
                    Some(instance),
                    GameLoopState::Playing(running.into_witness()),
                )
            }
            None => {
                if force_new_game {
                    let (instance, running) =
                        new_game(&mut rng_seed_source, &game_config, config.victories.clone());
                    (
                        Some(instance),
                        GameLoopState::Playing(running.into_witness()),
                    )
                } else {
                    (None, GameLoopState::MainMenu)
                }
            }
        };
        let controls = if let Some(controls) = storage.load_controls() {
            controls
        } else {
            let controls = Controls::default();
            storage.save_controls(&controls);
            controls
        };
        let music_state = MusicState::new();
        if mute {
            music_state.set_volume(0.0);
        } else {
            music_state.set_volume(0.5);
        }
        if instance.is_some() {
            music_state.set_track(Some(Track::Level));
        } else {
            music_state.set_track(Some(Track::Menu));
        };
        (
            Self {
                instance,
                controls,
                game_config,
                storage,
                rng_seed_source,
                config,
                images: Images::new(),
                cursor: None,
                music_state,
            },
            state,
        )
    }

    // XXX the witness system is overly restrictive
    fn try_save_instance_cheat(&mut self) {
        if let Some(instance) = self.instance.take() {
            let instance = instance.into_storable(witness::Running::cheat());
            self.storage.save_game(&instance);
            let (instance, _running) = instance.into_game_instance();
            self.instance = Some(instance);
        }
    }

    fn save_instance(&mut self, running: witness::Running) -> witness::Running {
        let instance = self.instance.take().unwrap().into_storable(running);
        self.storage.save_game(&instance);
        let (instance, running) = instance.into_game_instance();
        self.instance = Some(instance);
        running
    }

    fn clear_saved_game(&mut self) {
        self.music_state.set_track(Some(Track::Menu));
        self.storage.clear_game();
    }

    fn new_game(&mut self) -> witness::Running {
        let victories = self.config.victories.clone();
        let (instance, running) = new_game(&mut self.rng_seed_source, &self.game_config, victories);
        self.instance = Some(instance);
        self.music_state.set_track(Some(Track::Level));
        running
    }

    fn save_config(&mut self) {
        self.storage.save_config(&self.config);
    }

    fn render(&self, ctx: Ctx, fb: &mut FrameBuffer, mode: Mode) {
        let instance = self.instance.as_ref().unwrap();
        instance.render(ctx, fb, self.cursor, mode);
        match mode {
            Mode::Normal => {
                let colour = colours::NORMAL_MODE.to_rgba32(127);
                if let Some(cursor) = self.cursor {
                    let render_cell = RenderCell::default().with_background(colour);
                    fb.set_cell_relative_to_ctx(ctx, cursor, 50, render_cell);
                }
            }
            Mode::Aiming => {
                if let Some(cursor) = self.cursor {
                    let colour = colours::AIMING_MODE.to_rgba32(127);
                    let render_cell = RenderCell::default().with_background(colour);
                    let instance = self.instance.as_ref().unwrap();
                    for coord in
                        line_2d::coords_between(instance.game.inner_ref().player_coord(), cursor)
                    {
                        fb.set_cell_relative_to_ctx(ctx, coord, 50, render_cell);
                    }
                }
            }
        }
    }

    fn update(&mut self, event: Event, running: witness::Running) -> GameLoopState {
        let instance = self.instance.as_mut().unwrap();
        let witness = match event {
            Event::Input(input) => {
                self.cursor = None;
                if let Some(app_input) = self.controls.get(input) {
                    if instance.game.inner_ref().is_gameplay_blocked() {
                        running.into_witness()
                    } else {
                        let (witness, _action_result) = match app_input {
                            AppInput::Direction(direction) => {
                                running.walk(&mut instance.game, direction)
                            }
                            AppInput::Wait => running.wait(&mut instance.game),
                            AppInput::FireEquipped => {
                                self.cursor = Some(instance.game.inner_ref().player_coord());
                                (running.fire_equipped(), Ok(()))
                            }
                        };
                        witness
                    }
                } else {
                    if let Input::Mouse(MouseInput::MouseMove { coord, .. }) = input {
                        self.cursor = Some(coord);
                    }
                    if let Input::Mouse(MouseInput::MousePress { coord, .. }) = input {
                        self.cursor = Some(coord);
                    }
                    if let Input::Mouse(MouseInput::MouseRelease { coord, .. }) = input {
                        self.cursor = Some(coord);
                    }
                    if let Input::Keyboard(KeyboardInput::Char('?')) = input {
                        return GameLoopState::Help(running);
                    }
                    running.into_witness()
                }
            }
            Event::Tick(since_previous) => {
                running.tick(&mut instance.game, since_previous, &self.game_config)
            }
            _ => Witness::Running(running),
        };
        GameLoopState::Playing(witness)
    }
}

struct GameInstanceComponent(Option<witness::Running>);

impl GameInstanceComponent {
    fn new(running: witness::Running) -> Self {
        Self(Some(running))
    }
}

pub enum GameLoopState {
    Paused(witness::Running),
    Playing(Witness),
    MainMenu,
    Help(witness::Running),
}

impl Component for GameInstanceComponent {
    type Output = GameLoopState;
    type State = GameLoopData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        state.render(ctx, fb, Mode::Normal);
    }

    fn update(&mut self, state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        let running = witness::Running::cheat(); // XXX
        if event.is_escape() {
            GameLoopState::Paused(running)
        } else {
            state.update(event, running)
        }
    }

    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        ctx.bounding_box.size()
    }
}

struct GameInstanceFireEquippedComponent(Option<FireEquipped>);

struct Cancel;

impl Component for GameInstanceFireEquippedComponent {
    type Output = Option<(Result<Coord, Cancel>, FireEquipped)>;
    type State = GameLoopData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        state.render(ctx, fb, Mode::Aiming);
    }

    fn update(&mut self, state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        let instance = state.instance.as_mut().unwrap();
        if event.is_escape() {
            return Some((Err(Cancel), self.0.take().unwrap()));
        }
        match event {
            Event::Input(input) => {
                if let Input::Mouse(MouseInput::MouseMove { coord, .. }) = input {
                    if coord.is_valid(instance.game.inner_ref().world_size()) {
                        state.cursor = Some(coord);
                    }
                }
                if let Input::Mouse(MouseInput::MousePress { coord, .. }) = input {
                    return Some((Ok(coord), self.0.take().unwrap()));
                }
                if let Input::Keyboard(key) = input {
                    let delta = match key {
                        KeyboardInput::Left => Coord::new(-1, 0),
                        KeyboardInput::Right => Coord::new(1, 0),
                        KeyboardInput::Up => Coord::new(0, -1),
                        KeyboardInput::Down => Coord::new(0, 1),
                        _ => Coord::new(0, 0),
                    };
                    if let Some(cursor) = state.cursor {
                        let new_cursor = cursor + delta;
                        if new_cursor.is_valid(instance.game.inner_ref().world_size()) {
                            state.cursor = Some(new_cursor);
                        }
                    }
                }
            }
            Event::Tick(since_previous) => {
                Running::cheat().tick(&mut instance.game, since_previous, &state.game_config);
            }
            _ => (),
        }
        None
    }

    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        ctx.bounding_box.size()
    }
}

fn menu_style<T: 'static>(menu: AppCF<T>) -> AppCF<T> {
    menu.border(BorderStyle::default())
        .fill(MENU_BACKGROUND)
        .centre()
        .overlay_tint(
            render_state(|state: &State, ctx, fb| state.render(ctx, fb, Mode::Normal)),
            chargrid::core::TintDim(63),
            60,
        )
}

#[derive(Clone)]
enum MainMenuEntry {
    NewGame,
    Help,
    Quit,
}

fn title_decorate<T: 'static>(cf: AppCF<T>) -> AppCF<T> {
    let decoration = {
        let style = Style::plain_text();
        chargrid::many![styled_string(
            "Electric Organ".to_string(),
            style
                .with_bold(true)
                .with_foreground(colours::VAPORWAVE_BACKGROUND.to_rgba32(255))
                .with_background(
                    colours::VAPORWAVE_FOREGROUND
                        .to_rgba32(255)
                        .saturating_scalar_mul_div(4, 3)
                ),
        )]
    };
    cf.overlay(decoration.add_offset(Coord::new(31, 10)), 0)
}

fn main_menu() -> AppCF<MainMenuEntry> {
    use menu::builder::*;
    use MainMenuEntry::*;
    let mut builder = menu_builder().vi_keys();
    let mut add_item = |entry, name, ch: char| {
        let identifier =
            MENU_FADE_SPEC.identifier(move |b| write!(b, "({}) {}", ch, name).unwrap());
        builder.add_item_mut(item(entry, identifier).add_hotkey_char(ch));
    };
    add_item(NewGame, "New Game", 'n');
    add_item(Help, "Help", 'h');
    if !cfg!(feature = "web") {
        add_item(Quit, "Quit", 'q');
    }
    builder.build_cf()
}

enum MainMenuOutput {
    NewGame { new_running: witness::Running },
    Quit,
}

const MAIN_MENU_TEXT_WIDTH: u32 = 40;

fn background() -> CF<(), State> {
    unit()
}

struct MainMenuBackground {
    count: u64,
    rng_seed: u64,
    city_heights: Vec<u32>,
}

impl MainMenuBackground {
    fn new() -> Self {
        let mut rng = Isaac64Rng::from_entropy();
        let city_heights = (0..100).map(|_| rng.gen_range(3..8)).collect();
        Self {
            count: 0,
            rng_seed: rng.gen(),
            city_heights,
        }
    }
}

impl Component for MainMenuBackground {
    type Output = ();
    type State = GameLoopData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        let screen_size = ctx.bounding_box.size();
        let mut star_rng = Isaac64Rng::seed_from_u64(self.rng_seed);
        let mut star_brightness_rng = Isaac64Rng::seed_from_u64(self.count / 30);
        for i in 0..15 {
            for j in 0..(screen_size.width() as i32) {
                let coord = Coord::new(j, i);
                let render_cell = RenderCell {
                    character: None,
                    style: Style::default()
                        .with_background(colours::VAPORWAVE_FOREGROUND.to_rgba32(i as u8 * 10)),
                };
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
        }
        for i in 15..30 {
            for j in 0..(screen_size.width() as i32) {
                let coord = Coord::new(j, i);
                let render_cell = RenderCell {
                    character: None,
                    style: Style::default()
                        .with_background(colours::VAPORWAVE_BACKGROUND.to_rgba32(127)),
                };
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
        }
        for _ in 0..20 {
            let coord = Coord {
                x: star_rng.gen_range(0..screen_size.width() as i32),
                y: star_rng.gen_range(0..14),
            };
            let star_render_cell = RenderCell {
                character: Some('.'),
                style: Style::default()
                    .with_bold(true)
                    .with_foreground(Rgba32::new_grey(star_brightness_rng.gen_range(127..=255))),
            };
            fb.set_cell_relative_to_ctx(ctx, coord, 0, star_render_cell);
        }
        for i in 0..screen_size.width() {
            let city_height = self.city_heights
                [((i as usize + (self.count as usize / 30)) / 4) % self.city_heights.len()];
            for j in 0..city_height {
                let coord = Coord {
                    x: i as i32,
                    y: 14 - j as i32,
                };
                let render_cell = RenderCell {
                    character: Some(' '),
                    style: Style::default().with_background(Rgba32::new(0, 31, 127, 255)),
                };
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
        }
        let stride = 10;
        let virtual_width = 20;
        let offset = ((virtual_width * stride) / 2) as i32 - (screen_size.width() / 2) as i32;
        let end = Coord::new(screen_size.width() as i32 / 2, 5);
        let line_render_cell = |y| RenderCell {
            character: None,
            style: Style::default().with_background(
                colours::VAPORWAVE_FOREGROUND
                    .to_rgba32(255)
                    .linear_interpolate(
                        colours::VAPORWAVE_BACKGROUND.to_rgba32(255),
                        (y - 14) as u8 * 10,
                    ),
            ),
        };
        for i in 0..virtual_width {
            let x = (i * stride) - offset - ((self.count / 5) % stride as u64) as i32;
            let start = Coord::new(x as i32, screen_size.height() as i32);
            for coord in line_2d::coords_between(start, end) {
                if coord.x >= 0 && coord.x <= screen_size.width() as i32 {
                    fb.set_cell_relative_to_ctx(ctx, coord, 0, line_render_cell(coord.y));
                }
                if coord.y == screen_size.height() as i32 / 2 {
                    break;
                }
            }
        }
        let mut hline = |y| {
            for coord in
                line_2d::coords_between(Coord::new(0, y), Coord::new(screen_size.width() as i32, y))
            {
                fb.set_cell_relative_to_ctx(ctx, coord, 0, line_render_cell(y));
            }
        };
        hline(24);
        hline(20);
        hline(17);
        hline(15);
        let heart_image = if self.count % 120 < 110 {
            &state.images.heart
        } else {
            &state.images.heart_beat
        };
        let heart_width = 30;
        let heart_left = screen_size.width() as i32 / 2 - heart_width / 2;
        let heart_image_offset = Coord::new(12, 3);
        for i in 0..20 {
            for j in 0..heart_width {
                let coord = Coord::new(j, i);
                let screen_coord = Coord::new(j + heart_left, i);
                let heart_cell = heart_image.grid.get_checked(coord + heart_image_offset);
                if heart_cell.foreground().unwrap().r == 255 {
                    continue;
                }
                let mut render_cell = RenderCell {
                    character: Some(' '),
                    style: Style::default(),
                };
                let alpha = 50 + i as u8 * 10;
                if heart_cell.foreground().unwrap().g == 255 {
                    render_cell.style.background = Some(Rgb24::new(0, 255, 255).to_rgba32(alpha));
                }
                if heart_cell.foreground().unwrap().b == 255 {
                    render_cell.style.background = Some(Rgb24::new(0, 187, 127).to_rgba32(alpha));
                }
                fb.set_cell_relative_to_ctx(ctx, screen_coord, 0, render_cell);
            }
        }
    }
    fn update(&mut self, _state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        if let Event::Tick(_) = event {
            self.count += 1;
        }
    }
    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        ctx.bounding_box.size()
    }
}

fn help() -> AppCF<()> {
    text::help(MAIN_MENU_TEXT_WIDTH)
        .centre()
        .overlay(background(), 1)
}

fn main_menu_loop() -> AppCF<MainMenuOutput> {
    use MainMenuEntry::*;
    title_decorate(
        main_menu()
            .add_offset(Coord::new(32, 12))
            .overlay(MainMenuBackground::new(), 1),
    )
    .repeat_unit(move |entry| match entry {
        NewGame => text::loading(MAIN_MENU_TEXT_WIDTH)
            .centre()
            .overlay(background(), 1)
            .then(|| {
                on_state(|state: &mut State| MainMenuOutput::NewGame {
                    new_running: state.new_game(),
                })
            })
            .break_(),
        Help => help().continue_(),
        Quit => val_once(MainMenuOutput::Quit).break_(),
    })
}

#[derive(Clone)]
enum PauseMenuEntry {
    Resume,
    SaveQuit,
    Save,
    NewGame,
    Help,
    Clear,
}

fn pause_menu() -> AppCF<PauseMenuEntry> {
    use menu::builder::*;
    use PauseMenuEntry::*;
    let mut builder = menu_builder().vi_keys();
    let mut add_item = |entry, name, ch: char| {
        let identifier =
            MENU_FADE_SPEC.identifier(move |b| write!(b, "({}) {}", ch, name).unwrap());
        builder.add_item_mut(item(entry, identifier).add_hotkey_char(ch));
    };
    add_item(Resume, "Resume", 'r');
    if !cfg!(feature = "web") {
        add_item(SaveQuit, "Save and Quit", 'q');
        add_item(Save, "Save", 's');
    }
    add_item(NewGame, "New Game", 'n');
    add_item(Help, "Help", 'h');
    add_item(Clear, "Clear", 'c');
    builder.build_cf()
}

fn pause_menu_loop(running: witness::Running) -> AppCF<PauseOutput> {
    use PauseMenuEntry::*;
    let text_width = 64;
    pause_menu()
        .menu_harness()
        .repeat(
            running,
            move |running, entry_or_escape| match entry_or_escape {
                Ok(entry) => match entry {
                    Resume => break_(PauseOutput::ContinueGame { running }),
                    SaveQuit => text::saving(MAIN_MENU_TEXT_WIDTH)
                        .then(|| {
                            on_state(|state: &mut State| {
                                state.save_instance(running);
                                PauseOutput::Quit
                            })
                        })
                        .break_(),
                    Save => text::saving(MAIN_MENU_TEXT_WIDTH)
                        .then(|| {
                            on_state(|state: &mut State| PauseOutput::ContinueGame {
                                running: state.save_instance(running),
                            })
                        })
                        .break_(),
                    NewGame => text::loading(MAIN_MENU_TEXT_WIDTH)
                        .then(|| {
                            on_state(|state: &mut State| PauseOutput::ContinueGame {
                                running: state.new_game(),
                            })
                        })
                        .break_(),
                    Help => text::help(text_width).continue_with(running),
                    Clear => on_state(|state: &mut State| {
                        state.clear_saved_game();
                        PauseOutput::MainMenu
                    })
                    .break_(),
                },
                Err(_escape_or_start) => break_(PauseOutput::ContinueGame { running }),
            },
        )
}

enum PauseOutput {
    ContinueGame { running: witness::Running },
    MainMenu,
    Quit,
}

fn pause(running: witness::Running) -> AppCF<PauseOutput> {
    menu_style(pause_menu_loop(running))
}

fn game_instance_component(running: witness::Running) -> AppCF<GameLoopState> {
    cf(GameInstanceComponent::new(running)).some().no_peek()
}

fn fire_equipped(fire_equipped: FireEquipped) -> AppCF<Witness> {
    cf(GameInstanceFireEquippedComponent(Some(fire_equipped)))
        .no_peek()
        .map_side_effect(|(result, fire_equipped), state: &mut State| match result {
            Ok(coord) => {
                let instance = state.instance.as_mut().unwrap();
                let (witness, _) = fire_equipped.commit(&mut instance.game, coord);
                for external_event in instance.game.take_external_events() {
                    match external_event {
                        ExternalEvent::FirePistol => state.music_state.sfx_pistol(),
                        ExternalEvent::FireShotgun => state.music_state.sfx_shotgun(),
                        ExternalEvent::FireRocket => state.music_state.sfx_rocket(),
                        _ => (),
                    }
                }
                witness
            }
            Err(Cancel) => fire_equipped.cancel(),
        })
}

fn win() -> AppCF<()> {
    text::win(MAIN_MENU_TEXT_WIDTH)
}

fn game_over(reason: GameOverReason) -> AppCF<()> {
    on_state_then(move |state: &mut State| {
        state.clear_saved_game();
        state.save_config();
        text::game_over(MAIN_MENU_TEXT_WIDTH, reason)
    })
    .centre()
    .overlay(background(), 1)
}

fn game_menu(menu_witness: witness::Menu) -> AppCF<Witness> {
    use chargrid::align::*;
    use game::MenuChoice;
    use menu::builder::*;
    let mut builder = menu_builder();
    let mut add_item = |entry: MenuChoice, name: String, ch: char| {
        let identifier = MENU_FADE_SPEC.identifier(move |b| write!(b, "{}. {}", ch, name).unwrap());
        builder.add_item_mut(item(entry, identifier).add_hotkey_char(ch));
    };
    for (i, choice) in menu_witness.menu.choices.iter().enumerate() {
        let ch = std::char::from_digit(i as u32 + 1, 10).unwrap();
        match choice {
            MenuChoice::Dummy => add_item(choice.clone(), "Dummy".to_string(), ch),
        }
    }
    let title = {
        use chargrid::text::*;
        Text::new(vec![StyledString {
            string: menu_witness.menu.text.clone(),
            style: Style::plain_text(),
        }])
        .wrap_word()
        .cf::<State>()
        .set_width(36)
    };
    let menu_cf = builder
        .build_cf()
        .menu_harness()
        .add_x(2)
        .with_title_vertical(title, 2)
        .align(Alignment {
            x: AlignmentX::Left,
            y: AlignmentY::Centre,
        })
        .add_x(4)
        .overlay(
            render_state(move |state: &State, ctx, fb| {
                state
                    .images
                    .image_from_menu_image(menu_witness.menu.image)
                    .render(ctx, fb)
            }),
            1,
        );
    menu_cf.and_then_side_effect(|result, state: &mut State| {
        let witness = match result {
            Err(Close) => menu_witness.cancel(),
            Ok(choice) => {
                if let Some(instance) = state.instance.as_mut() {
                    menu_witness.commit(&mut instance.game, choice.clone())
                } else {
                    menu_witness.cancel()
                }
            }
        };
        val_once(witness)
    })
}

pub fn pre_game_screen() -> AppCF<()> {
    if cfg!(feature = "web") {
        text::press_any_key_to_begin(MAIN_MENU_TEXT_WIDTH).press_any_key()
    } else {
        unit().some()
    }
}

pub fn game_loop_component(initial_state: GameLoopState) -> AppCF<()> {
    use GameLoopState::*;
    pre_game_screen().then(|| {
        loop_(initial_state, |state| match state {
            Playing(witness) => match witness {
                Witness::Running(running) => game_instance_component(running).continue_(),
                Witness::GameOver(reason) => game_over(reason).map_val(|| MainMenu).continue_(),
                Witness::Win(_) => win().map_val(|| MainMenu).continue_(),
                Witness::Menu(menu_) => game_menu(menu_).map(Playing).continue_(),
                Witness::FireEquipped(fire_equipped_) => {
                    fire_equipped(fire_equipped_).map(Playing).continue_()
                }
            },
            Paused(running) => pause(running).map(|pause_output| match pause_output {
                PauseOutput::ContinueGame { running } => {
                    LoopControl::Continue(Playing(running.into_witness()))
                }
                PauseOutput::MainMenu => LoopControl::Continue(MainMenu),
                PauseOutput::Quit => LoopControl::Break(()),
            }),
            MainMenu => main_menu_loop().map(|main_menu_output| match main_menu_output {
                MainMenuOutput::NewGame { new_running } => {
                    LoopControl::Continue(Playing(new_running.into_witness()))
                }
                MainMenuOutput::Quit => LoopControl::Break(()),
            }),
            Help(running) => help()
                .map(|()| GameLoopState::Playing(running.into_witness()))
                .continue_(),
        })
        .bound_size(Size::new_u16(80, 30))
        .on_each_tick_with_state(|state| state.music_state.tick())
        .on_exit_with_state(|state| state.try_save_instance_cheat())
    })
}
