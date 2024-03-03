use chargrid::{control_flow::*, core::*};
use game::Config;

mod colours;
mod controls;
mod game_instance;
mod game_loop;
mod image;
mod music;
mod text;

pub use game_loop::{AppStorage, InitialRngSeed};

pub const NAME: &'static str = "Electric Organ";

struct AppState {
    game_loop_data: game_loop::GameLoopData,
}

pub struct AppArgs {
    pub storage: AppStorage,
    pub initial_rng_seed: InitialRngSeed,
    pub omniscient: bool,
    pub new_game: bool,
    pub mute: bool,
}

pub fn app(
    AppArgs {
        storage,
        initial_rng_seed,
        omniscient,
        new_game,
        mute,
    }: AppArgs,
) -> impl Component<Output = app::Output, State = ()> {
    let config = Config {
        omniscient: if omniscient { Config::OMNISCIENT } else { None },
        demo: false,
        debug: false,
    };
    let (game_loop_data, initial_state) =
        game_loop::GameLoopData::new(config, storage, initial_rng_seed, new_game, mute);
    let state = AppState { game_loop_data };
    game_loop::game_loop_component(initial_state)
        .lens_state(lens!(AppState[game_loop_data]: game_loop::GameLoopData))
        .map(|_| app::Exit)
        .with_state(state)
        .clear_each_frame()
        .exit_on_close()
}
