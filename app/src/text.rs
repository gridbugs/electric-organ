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
            b("Controls\n\n\n"),
            t("Walk: ←↑→↓\n\n"),
            t("Wait: Space\n\n"),
            t("Fire Equipped Weapon: f\n\n"),
            t("Fire all Cronenberg Weapons (costs health): c\n\n"),
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
    .then(move || help_1_5(width))
}

pub fn help_1_5(width: u32) -> AppCF<()> {
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
            b("Tips\n\n\n"),
            t("- Walk into doors to open them.\n\n"),
            t("- Walk into the wall adjacent to a door to close it.\n\n"),
            t("- Walk into enemies to perform a melee attack.\n\n"),
        ],
    )
    .press_any_key()
    .then(move || help2(width))
}

pub fn help2(width: u32) -> AppCF<()> {
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
            b("Organs\n"),
            b("\nHeart: "),
            t("Each heart increases your max health. If you have no hearts then you die."),
            b("\nLiver: "),
            t("Each liver speeds up poison recovery."),
            b("\nLung: "),
            t("Each lung increases the amount of oxygen you recover each turn."),
            b("\nStomach: "),
            t("Your food will decrease over time and increase your health by an amount determined by how many stomachs you have."),
            b("\nAppendix: "),
            t("Does nothing. You start with one."),
            b("\nTumour: "),
            t("Does nothing. Can be caused by radiation. Often has the \"Prolific\" trait causing it to periodically duplicate."),
            b("\nCronenberg Pistol: "),
            t("A biological pistol attached to your body. Costs health to fire."),
            b("\nCronenberg Shotgun: "),
            t("A biological shotgun attached to your body. Costs health to fire."),
            b("\nClaw: "),
            t("Greatly increases melee damage. Replaces a hand. \
                With one claw you can still hold a pistol. With two claws you can't hold any guns."),
            b("\nCyberCore™: "),
            t("Allows cybernetic organs to operate."),
        ],
    )
    .press_any_key()
    .then(move || help3(width))
}

pub fn help3(width: u32) -> AppCF<()> {
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
            b("Organ Traits\n"),
            b("\n\nCybernetic: "),
            t("More effective but requires CyberCore™."),
            b("\n\nProlific: "),
            t("Periodically makes copies of itself."),
            b("\n\nVampiric: "),
            t("Only functions if you have recently consumed a blood vial."),
            b("\n\nRadioactive: "),
            t("Increases rate of radiation buildup. When radiation fills up you gain a mutation."),
            b("\n\nDamaged: "),
            t("The organ is less effective. Damaged Cronenberg guns cost more health to fire."),
            b("\n\nEmbedded: "),
            t("Costs more money to remove."),
            b("\n\nTransient: "),
            t("May disappear at any time."),
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
    let text = vec![t("You have died... (press any key to continue)")];
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
    text_component(
        width,
        vec![t(
            "You defeated the CORRUPTOR and saved the city. Congratulations hero!",
        )],
    )
}
pub fn win(width: u32) -> AppCF<()> {
    // TODO: this is not ergonomic
    win_text(width)
        .delay(Duration::from_secs(1))
        .then(move || win_text(width).press_any_key())
}

fn bad_win_text(width: u32) -> CF<(), State> {
    let t = |s: &str| StyledString {
        string: s.to_string(),
        style: Style::plain_text(),
    };
    text_component(
        width,
        vec![t(
            "With the CORRUPTED HEART beating in your chest you finally take your rightful place as the god of this world.",
        )],
    )
}
pub fn bad_win(width: u32) -> AppCF<()> {
    // TODO: this is not ergonomic
    bad_win_text(width)
        .delay(Duration::from_secs(1))
        .then(move || bad_win_text(width).press_any_key())
}
