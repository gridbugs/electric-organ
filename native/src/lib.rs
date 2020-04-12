use general_audio_static::{
    backend::{Error as NativeAudioError, NativeAudioPlayer},
    StaticAudioPlayer,
};
use general_storage_static::backend::{FileStorage, IfDirectoryMissing};
pub use general_storage_static::StaticStorage;
pub use simon;
use simon::*;
use slime99_app::{AppAudioPlayer, Controls, GameConfig, Omniscient, RngSeed};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

const DEFAULT_SAVE_FILE: &str = "save";
const DEFAULT_NEXT_TO_EXE_SAVE_DIR: &str = "save";
const DEFAULT_NEXT_TO_EXE_CONTROLS_FILE: &str = "controls.json";

pub struct NativeCommon {
    pub rng_seed: RngSeed,
    pub save_file: String,
    pub file_storage: StaticStorage,
    pub controls: Controls,
    pub audio_player: AppAudioPlayer,
    pub game_config: GameConfig,
}

fn read_controls_file(path: &PathBuf) -> Option<Controls> {
    let mut buf = Vec::new();
    let mut f = File::open(path).ok()?;
    f.read_to_end(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

impl NativeCommon {
    pub fn arg() -> impl Arg<Item = Self> {
        args_map! {
            let {
                rng_seed = opt::<u64>("r", "rng-seed", "rng seed to use for first new game", "INT")
                    .option_map(|seed| RngSeed::U64(seed))
                    .with_default(RngSeed::Random);
                save_file = opt("s", "save-file", "save file", "PATH")
                    .with_default(DEFAULT_SAVE_FILE.to_string());
                save_dir = opt("d", "save-dir", "save dir", "PATH")
                    .with_default(DEFAULT_NEXT_TO_EXE_SAVE_DIR.to_string());
                controls_file = opt::<String>("c", "controls-file", "controls file", "PATH");
                delete_save = flag("", "delete-save", "delete save game file");
                omniscient = flag("", "omniscient", "enable omniscience").some_if(Omniscient);
                audio = flag("a", "audio", "enable audio (may crash the game after a few minutes)");
            } in {{
                let controls_file = if let Some(controls_file) = controls_file {
                    controls_file.into()
                } else {
                    env::current_exe().unwrap().parent().unwrap().join(DEFAULT_NEXT_TO_EXE_CONTROLS_FILE)
                        .to_path_buf()
                };
                let controls = read_controls_file(&controls_file).unwrap_or_else(Controls::default);
                let mut file_storage = StaticStorage::new(FileStorage::next_to_exe(
                    &save_dir,
                    IfDirectoryMissing::Create,
                ).expect("failed to open directory"));
                if delete_save {
                    let result = file_storage.remove(&save_file);
                    if result.is_err() {
                        log::warn!("couldn't find save file to delete");
                    }
                }
                let audio_player = if audio {
                    match NativeAudioPlayer::try_new_default_device() {
                        Ok(audio_player) => Some(StaticAudioPlayer::new(audio_player)),
                        Err(NativeAudioError::NoOutputDevice) => {
                            log::warn!("no output audio device - continuing without audio");
                            None
                        }
                    }
                } else {
                    None
                };
                let game_config = GameConfig { omniscient };
                Self {
                    rng_seed,
                    save_file,
                    file_storage,
                    controls,
                    audio_player,
                    game_config,
                }
            }}
        }
    }
}
