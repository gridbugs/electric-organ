#![windows_subsystem = "windows"]
use app::{app, AppArgs, NAME};
use chargrid_sdl2::*;
use native::{meap, NativeCommon};

fn main() {
    use meap::Parser;
    env_logger::init();
    let NativeCommon {
        storage,
        initial_rng_seed,
        omniscient,
        new_game,
        mute,
    } = NativeCommon::parser()
        .with_help_default()
        .parse_env_or_exit();
    let context = Context::new(Config {
        font_bytes: FontBytes {
            normal: include_bytes!("./fonts/PxPlus_IBM_CGAthin-2y.ttf").to_vec(),
            bold: include_bytes!("./fonts/PxPlus_IBM_CGA-2y.ttf").to_vec(),
        },
        title: NAME.to_string(),
        window_dimensions_px: Dimensions {
            width: 960.,
            height: 720.,
        },
        cell_dimensions_px: Dimensions {
            width: 12.,
            height: 24.,
        },
        font_point_size: 24,
        character_cell_offset: Dimensions {
            width: 0.,
            height: 0.,
        },
        underline_width_cell_ratio: 0.1,
        underline_top_offset_cell_ratio: 0.8,
        resizable: false,
    });
    context.run(app(AppArgs {
        storage,
        initial_rng_seed,
        omniscient,
        new_game,
        mute,
    }));
}
