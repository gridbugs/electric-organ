use crate::game_loop::{AppCF, State};
use boat_journey_game::GameOverReason;
use chargrid::{
    control_flow::*,
    prelude::*,
    text::{StyledString, Text},
};

fn text_component(width: u32, text: Vec<StyledString>) -> CF<(), State> {
    Text::new(text).wrap_word().cf().set_width(width)
}

pub fn help(width: u32) -> AppCF<()> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    let b = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text().with_bold(true),
    };
    text_component(
        width,
        vec![
            b("Controls:\n\n"),
            b("General\n"),
            t("Wait: Space\n"),
            t("Ability: 1-9\n"),
            t("\n"),
            b("On Foot\n"),
            t("Walk: Arrow Keys\n"),
            t("Drive Boat: e\n"),
            t("\n"),
            b("Driving Boat\n"),
            t("Move: Forward/Backward\n"),
            t("Turn: Left/Right\n"),
            t("Leave Boat: e\n"),
            b("\n\nTips:\n\n"),
            t("- Walk into a door (+) to open it\n"),
            t("- Walk into the wall next to a door to close the door\n"),
            t("- Head to the inn when it gets dark\n"),
        ],
    )
    .press_any_key()
}

pub fn loading(width: u32) -> AppCF<()> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("Generating...")]).delay(Duration::from_millis(32))
}

pub fn saving(width: u32) -> AppCF<()> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("Saving...")]).delay(Duration::from_millis(32))
}

fn game_over_text(width: u32, _reason: GameOverReason) -> CF<(), State> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    let text = vec![t("TODO")];
    text_component(width, text)
}

pub fn game_over(width: u32, reason: GameOverReason) -> AppCF<()> {
    game_over_text(width, reason)
        .delay(Duration::from_secs(2))
        .then(move || game_over_text(width, reason).press_any_key())
}
