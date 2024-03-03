use app::{app, AppArgs, AppStorage, InitialRngSeed};
use chargrid_web::{Context, LoopMethod, Size};
use general_storage_static::StaticStorage;
use general_storage_web::LocalStorage;
use wasm_bindgen::prelude::*;

const SAVE_KEY: &str = "save";
const CONFIG_KEY: &str = "config";
const CONTROLS_KEY: &str = "controls";

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();
    let mut storage = StaticStorage::new(LocalStorage::new());
    let _ = storage.remove(CONFIG_KEY);
    let _ = storage.remove(CONTROLS_KEY);
    let context = Context::new(Size::new(80, 30), "content");
    let args = AppArgs {
        storage: AppStorage {
            handle: storage,
            save_game_key: SAVE_KEY.to_string(),
            config_key: CONFIG_KEY.to_string(),
            controls_key: CONTROLS_KEY.to_string(),
        },
        initial_rng_seed: InitialRngSeed::Random,
        omniscient: false,
        new_game: false,
        mute: false,
    };
    context.run_with_loop_method(app(args), LoopMethod::SetTimeoutMs(1000 / 60));
    Ok(())
}
