use crate::{
    colours,
    controls::{AppInput, Controls},
    game_instance::{
        item_string_for_menu, message_to_text, organ_string_for_menu, GameInstance,
        GameInstanceStorable, Mode,
    },
    image::Images,
    music::{MusicState, Track},
    text,
};
use chargrid::{self, border::BorderStyle, control_flow::*, menu, prelude::*};
use game::{
    witness::{self, FireBody, FireEquipped, Running, Witness},
    Config as GameConfig, ExternalEvent, GameOverReason, Item, Menu as GameMenu,
    MenuChoice as GameMenuChoice, Victory, WhichHand,
};
use general_storage_static::{self as storage, format, StaticStorage as Storage};
use line_2d;
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
use rgb_int::Rgb24;
use serde::{Deserialize, Serialize};

const LEVEL_TRACKS: &[Track] = &[Track::Level1, Track::Level2];

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

const MENU_BACKGROUND: Rgba32 = colours::VAPORWAVE_BACKGROUND
    .to_rgba32(255)
    .saturating_scalar_mul_div(1, 4);
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
                    background: MENU_BACKGROUND,
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

#[derive(Clone, Copy)]
struct ScreenShake {
    countdown: u32,
    offset: Coord,
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
    screen_shake: Option<ScreenShake>,
    level_track_index: usize,
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
            music_state.set_track(Some(Track::Level1));
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
                screen_shake: None,
                level_track_index: 0,
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
        self.instance = None;
    }

    fn new_game(&mut self) -> witness::Running {
        let victories = self.config.victories.clone();
        let (instance, running) = new_game(&mut self.rng_seed_source, &self.game_config, victories);
        self.instance = Some(instance);
        self.music_state.set_track(Some(Track::Level1));
        running
    }

    fn save_config(&mut self) {
        self.storage.save_config(&self.config);
    }

    fn render(&self, ctx: Ctx, fb: &mut FrameBuffer, mode: Mode) {
        if let Some(instance) = self.instance.as_ref() {
            let offset = self
                .screen_shake
                .map(|s| s.offset)
                .unwrap_or(Coord::new(0, 0));
            instance.render(ctx, fb, self.cursor, mode, offset);
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
                        for coord in line_2d::coords_between(
                            instance.game.inner_ref().player_coord(),
                            cursor,
                        ) {
                            fb.set_cell_relative_to_ctx(ctx, coord, 50, render_cell);
                        }
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
                                let witness = running.walk(&mut instance.game, direction);
                                for external_event in instance.game.take_external_events() {
                                    match external_event {
                                        ExternalEvent::ChangeLevel => {
                                            self.level_track_index += 1;
                                            self.music_state.set_track(Some(
                                                LEVEL_TRACKS
                                                    [self.level_track_index % LEVEL_TRACKS.len()],
                                            ));
                                        }
                                        _ => (),
                                    }
                                }
                                witness
                            }
                            AppInput::Wait => running.wait(&mut instance.game),
                            AppInput::Get => running.get(&mut instance.game),
                            AppInput::FireEquipped => {
                                self.cursor = Some(instance.game.inner_ref().player_coord());
                                (running.fire_equipped(), Ok(()))
                            }
                            AppInput::FireBody => {
                                self.cursor = Some(instance.game.inner_ref().player_coord());
                                (running.fire_body(), Ok(()))
                            }
                            AppInput::MessageLog => {
                                return GameLoopState::MessageLog(running);
                            }
                            AppInput::ViewOrgans => {
                                return GameLoopState::ViewOrgans(running);
                            }
                            AppInput::DropItem => (
                                drop_menu_witness(instance.game.inner_ref(), running),
                                Ok(()),
                            ),
                            AppInput::ApplyItem => (
                                apply_menu_witness(instance.game.inner_ref(), running),
                                Ok(()),
                            ),
                            AppInput::UnequipItem => running.unequip(&mut instance.game),
                            AppInput::Reload => running.reload(&mut instance.game),
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
                let witness = running.tick(&mut instance.game, since_previous, &self.game_config);
                self.screen_shake = self.screen_shake.and_then(|mut screen_shake| {
                    if screen_shake.countdown == 0 {
                        None
                    } else {
                        screen_shake.countdown -= 1;
                        Some(screen_shake)
                    }
                });
                for external_event in instance.game.take_external_events() {
                    match external_event {
                        ExternalEvent::Explosion(_) => {
                            self.music_state.sfx_explosion();
                            let mut rng = Isaac64Rng::from_entropy();
                            let screen_shake = ScreenShake {
                                countdown: 2,
                                offset: if rng.gen() {
                                    Coord::new(-1, 0)
                                } else {
                                    Coord::new(1, 0)
                                },
                            };
                            self.screen_shake = Some(screen_shake);
                        }
                        _ => (),
                    }
                }
                witness
            }
            _ => Witness::Running(running),
        };
        GameLoopState::Playing(witness)
    }
}

struct GameInstanceComponent(Option<witness::Running>);

fn drop_menu_witness(game: &game::Game, running: witness::Running) -> Witness {
    let choices = (0..game.inventory_size())
        .map(|i| GameMenuChoice::DropItem(i))
        .collect::<Vec<_>>();
    let menu = GameMenu {
        text: format!("Select an item to drop (escape to cancel):"),
        choices,
        image: None,
    };
    running.menu(menu)
}

fn apply_menu_witness(game: &game::Game, running: witness::Running) -> Witness {
    let choices = (0..game.inventory_size())
        .map(|i| GameMenuChoice::ApplyItem(i))
        .collect::<Vec<_>>();
    let menu = GameMenu {
        text: format!("Select an item to apply (escape to cancel):"),
        choices,
        image: None,
    };
    running.menu(menu)
}

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
    MessageLog(witness::Running),
    ViewOrgans(witness::Running),
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
                if let Input::Keyboard(input::keys::RETURN) = input {
                    if let Some(coord) = state.cursor {
                        return Some((Ok(coord), self.0.take().unwrap()));
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

struct GameInstanceFireBodyComponent(Option<FireBody>);

impl Component for GameInstanceFireBodyComponent {
    type Output = Option<(Result<Coord, Cancel>, FireBody)>;
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
                if let Input::Keyboard(input::keys::RETURN) = input {
                    if let Some(coord) = state.cursor {
                        return Some((Ok(coord), self.0.take().unwrap()));
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
    let mut border_style = BorderStyle::default();
    border_style.foreground = colours::VAPORWAVE_FOREGROUND.to_rgba32(255);
    menu.border(border_style)
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
    use chargrid::pad_by::Padding;
    menu_style(
        text::help(60)
            .pad_by(Padding {
                left: 1,
                right: 4,
                top: 1,
                bottom: 1,
            })
            .overlay(background(), 1),
    )
}

struct MessageLog {
    scroll_from_bottom: usize,
}

impl Component for MessageLog {
    type Output = Option<()>;
    type State = GameLoopData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        use chargrid::text::*;
        let ctx = ctx.set_size(self.size(state, ctx));
        Text::new(vec![StyledString {
            string: format!("Scroll with ↑↓. Press any other key to return to the game."),
            style: Style::plain_text().with_foreground(Rgba32::new_grey(127)),
        }])
        .wrap_word()
        .render(&(), ctx, fb);
        let ctx = ctx.add_xy(0, 3);
        let instance = state.instance.as_ref().unwrap();
        let message_log = instance.game.inner_ref().message_log();
        let num_messages = message_log.len();
        if num_messages == 0 {
            StyledString {
                string: format!("(No messages in log.)"),
                style: Style::plain_text(),
            }
            .render(&(), ctx, fb);
        } else {
            let message_log_start = message_log
                .len()
                .saturating_sub(ctx.bounding_box.size().height() as usize)
                - self.scroll_from_bottom;
            for (i, &ref message) in message_log[message_log_start..].into_iter().enumerate() {
                message_to_text(message.clone()).render(&(), ctx.add_y(i as i32), fb);
            }
        }
    }

    fn update(&mut self, state: &mut Self::State, ctx: Ctx, event: Event) -> Self::Output {
        let ctx = ctx.set_size(self.size(state, ctx));
        let ctx = ctx.add_xy(0, 3);
        let instance = state.instance.as_ref().unwrap();
        let message_log = instance.game.inner_ref().message_log();
        let num_messages = message_log.len();
        match event {
            Event::Input(Input::Keyboard(key)) => match key {
                KeyboardInput::Up => {
                    if num_messages > ctx.bounding_box.size().height() as usize {
                        let offset = num_messages - ctx.bounding_box.size().height() as usize;
                        let new_scroll = self.scroll_from_bottom + 1;
                        if new_scroll < offset {
                            self.scroll_from_bottom = new_scroll;
                        }
                    }
                }
                KeyboardInput::Down => {
                    self.scroll_from_bottom = self.scroll_from_bottom.saturating_sub(1);
                }
                _ => return Some(()),
            },
            _ => (),
        }
        None
    }

    fn size(&self, _state: &Self::State, _ctx: Ctx) -> Size {
        Size::new(60, 25)
    }
}

fn message_log() -> AppCF<()> {
    menu_style(cf(MessageLog {
        scroll_from_bottom: 0,
    }))
}

struct ViewOrgans;
impl ViewOrgans {
    const SIZE: Size = Size::new_u16(40, 14);
}
impl Component for ViewOrgans {
    type Output = Option<()>;
    type State = GameLoopData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        use chargrid::text::*;
        let ctx = ctx.set_size(Self::SIZE).add_xy(1, 1);
        Text::new(vec![StyledString {
            string: format!("Viewing your organs. Press any key to return to the game."),
            style: Style::plain_text().with_foreground(Rgba32::new_grey(127)),
        }])
        .wrap_word()
        .render(&(), ctx, fb);
        let instance = state.instance.as_ref().unwrap();
        let ctx = ctx.add_y(4);
        for (i, slot) in instance
            .game
            .inner_ref()
            .player_organs()
            .into_iter()
            .enumerate()
        {
            let s = if let Some(organ) = slot {
                let string = organ_string_for_menu(&organ);
                StyledString {
                    string,
                    style: Style::plain_text(),
                }
            } else {
                StyledString {
                    string: "(empty)".to_string(),
                    style: Style::plain_text().with_foreground(Rgb24::new_grey(127).to_rgba32(255)),
                }
            };
            s.render(&(), ctx.add_y(i as i32), fb);
        }
    }

    fn update(&mut self, _state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        if event.keyboard_input().is_some() {
            Some(())
        } else {
            None
        }
    }

    fn size(&self, _state: &Self::State, _ctx: Ctx) -> Size {
        Self::SIZE
    }
}

fn view_organs() -> AppCF<()> {
    menu_style(cf(ViewOrgans))
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

fn fire_body(fire_body: FireBody) -> AppCF<Witness> {
    cf(GameInstanceFireBodyComponent(Some(fire_body)))
        .no_peek()
        .map_side_effect(|(result, fire_body), state: &mut State| match result {
            Ok(coord) => {
                let instance = state.instance.as_mut().unwrap();
                let (witness, _) = fire_body.commit(&mut instance.game, coord);
                for external_event in instance.game.take_external_events() {
                    match external_event {
                        ExternalEvent::FirePistol => state.music_state.sfx_pistol(),
                        ExternalEvent::FireShotgun => state.music_state.sfx_shotgun(),
                        _ => (),
                    }
                }
                witness
            }
            Err(Cancel) => fire_body.cancel(),
        })
}

fn win() -> AppCF<()> {
    text::win(MAIN_MENU_TEXT_WIDTH)
}

fn game_over(reason: GameOverReason) -> AppCF<()> {
    menu_style(on_state_then(move |_state: &mut State| {
        text::game_over(MAIN_MENU_TEXT_WIDTH, reason)
    }))
    .map_side_effect(|_, state: &mut State| {
        state.clear_saved_game();
        state.save_config();
    })
}

fn apply_item_description(item: Item) -> String {
    use Item::*;
    match item {
        Stimpack => "Consume to increase health".to_string(),
        Antidote => "Consume to decrease poison".to_string(),
        BloodVialEmpty => "Fill with blood (must be standing on corpse)".to_string(),
        BloodVialFull => "Consume to increase oxygen".to_string(),
        Battery => "Consume to increase power (requires CyberCore™)".to_string(),
        Food => "Consume to gain food".to_string(),
        AntiRads => "Consume to reduce radiation".to_string(),
        OrganContainer(Some(_)) => "Dump contents".to_string(),
        OrganContainer(None) => "Harvest organ (must be standing on corpse)".to_string(),
        Pistol => "Equip weapon (requires non-claw hand)".to_string(),
        Shotgun | RocketLauncher => "Equip weapon (requires two non-claw hands)".to_string(),
        PistolAmmo | ShotgunAmmo | Rocket => "Load into current weapon".to_string(),
    }
}

fn menu_choice_string(game: &game::Game, choice: GameMenuChoice) -> String {
    match choice {
        GameMenuChoice::Dummy => panic!(),
        GameMenuChoice::DropItem(i) => {
            if let Some(item) = game.inventory_item(i) {
                item_string_for_menu(item)
            } else {
                format!("(empty)")
            }
        }
        GameMenuChoice::ApplyItem(i) => {
            if let Some(item) = game.inventory_item(i) {
                format!(
                    "{} - {}",
                    item_string_for_menu(item),
                    apply_item_description(item)
                )
            } else {
                format!("(empty)")
            }
        }
        GameMenuChoice::HarvestOrgan { organ, .. } => organ_string_for_menu(&organ),
        GameMenuChoice::EquipWeaponInHand { which_hand, .. } => match which_hand {
            WhichHand::Left => "Left Hand".to_string(),
            WhichHand::Right => "Right Hand".to_string(),
        },
        GameMenuChoice::UnequipWhichHand(which_hand) => match which_hand {
            WhichHand::Left => "Left Hand".to_string(),
            WhichHand::Right => "Right Hand".to_string(),
        },
    }
}

const ALPHABET: &'static str = "abcdefghijklmnopqrstuvwxyz";

fn game_menu(menu_witness: witness::Menu) -> AppCF<Witness> {
    use chargrid::align::*;
    use game::MenuChoice;
    use menu::builder::*;
    let game_menu = menu_witness.menu.clone();
    let menu_cf = on_state_then(move |state: &mut State| {
        let instance = state.instance.as_ref().unwrap();
        let mut builder = menu_builder();
        let mut add_item = |entry: MenuChoice, name: String, ch: char| {
            let identifier =
                MENU_FADE_SPEC.identifier(move |b| write!(b, "{}) {}", ch, name).unwrap());
            builder.add_item_mut(item(entry, identifier).add_hotkey_char(ch));
        };
        for (choice, ch) in game_menu.choices.iter().zip(ALPHABET.chars()) {
            add_item(
                choice.clone(),
                menu_choice_string(instance.game.inner_ref(), choice.clone()),
                ch,
            );
        }
        let title = {
            use chargrid::text::*;
            Text::new(vec![StyledString {
                string: game_menu.text.clone(),
                style: Style::plain_text(),
            }])
            .wrap_word()
            .cf::<State>()
            .set_width(50)
        };
        let menu = builder
            .build_cf()
            .menu_harness()
            .with_title_vertical(title, 2);
        if let Some(menu_image) = menu_witness.menu.image {
            menu.add_x(2)
                .align(Alignment {
                    x: AlignmentX::Left,
                    y: AlignmentY::Centre,
                })
                .add_x(4)
                .overlay(
                    render_state(move |state: &State, ctx, fb| {
                        state
                            .images
                            .image_from_menu_image(menu_image)
                            .render(ctx, fb);
                    }),
                    1,
                )
        } else {
            menu_style(menu)
        }
    });
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
                Witness::FireBody(fire_body_) => fire_body(fire_body_).map(Playing).continue_(),
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
            MessageLog(running) => message_log()
                .map(|()| GameLoopState::Playing(running.into_witness()))
                .continue_(),
            ViewOrgans(running) => view_organs()
                .map(|()| GameLoopState::Playing(running.into_witness()))
                .continue_(),
        })
        .bound_size(Size::new_u16(80, 30))
        .on_each_tick_with_state(|state| state.music_state.tick())
        .on_exit_with_state(|state| state.try_save_instance_cheat())
    })
}
