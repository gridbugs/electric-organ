use crate::game_loop::{AppCF, State};
use chargrid::{
    control_flow::*,
    prelude::*,
    text::{StyledString, Text},
};
use game::GameOverReason;

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
            b("Controls:\n\n\n"),
            t("Walk: ←↑→↓\n\n"),
            t("Wait: Space\n\n"),
            t("Fire Equipped Weapon: f\n\n"),
            t("Fire Body Weapon: b\n\n"),
            t("Get item: g\n\n"),
            t("Apply item: a\n\n"),
            t("Drop item: d\n\n"),
            t("Unequip item: u\n\n"),
            t("Reload equipped weapon: r\n\n"),
            t("Display message log: m\n\n"),
            t("Display list of organs: o\n\n"),
            t("Display this help message: ?\n\n"),
        ],
    )
    .press_any_key()
}

pub fn press_any_key_to_begin(width: u32) -> CF<(), State> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("Press any key to begin...")])
}

pub fn loading(width: u32) -> AppCF<()> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("Generating...")]).delay(Duration::from_millis(100))
}

pub fn saving(width: u32) -> AppCF<()> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("Saving...")]).delay(Duration::from_millis(100))
}

fn game_over_text(width: u32, _reason: GameOverReason) -> CF<(), State> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    let text = vec![t("You have died...")];
    text_component(width, text)
}

pub fn game_over(width: u32, reason: GameOverReason) -> AppCF<()> {
    game_over_text(width, reason)
        .delay(Duration::from_secs(1))
        .then(move || game_over_text(width, reason).press_any_key())
}

fn win_text(width: u32) -> CF<(), State> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(width, vec![t("You win!")])
}
pub fn win(width: u32) -> AppCF<()> {
    // TODO: this is not ergonomic
    win_text(width)
        .delay(Duration::from_secs(1))
        .then(move || win_text(width).press_any_key())
}
