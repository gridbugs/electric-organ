use crate::colours;
use chargrid::{
    prelude::*,
    text::{self, Text},
};
use game::{
    witness::{self, Game, RunningGame},
    ActionError, CellVisibility, Config, Item, Layer, LayerTable, Message, Meter, NpcType, Organ,
    OrganTrait, OrganTraits, OrganType, Tile, Victory, VisibleEntity,
};
use rand::Rng;
use rgb_int::Rgb24;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Normal,
    Aiming,
}

#[derive(Clone, Copy)]
struct LightBlend {
    light_colour: Rgb24,
}

impl Tint for LightBlend {
    fn tint(&self, rgba32: Rgba32) -> Rgba32 {
        rgba32
            .to_rgb24()
            .normalised_mul(self.light_colour)
            .saturating_add(self.light_colour.saturating_scalar_mul_div(1, 5))
            .to_rgba32(255)
    }
}

pub struct GameInstance {
    pub game: Game,
}

fn visible_entity_on_top(layers: &LayerTable<VisibleEntity>) -> Option<(&VisibleEntity, Layer)> {
    if layers.character.tile.is_some() {
        return Some((&layers.character, Layer::Character));
    }
    if layers.item.tile.is_some() {
        return Some((&layers.item, Layer::Item));
    }
    if layers.feature.tile.is_some() {
        return Some((&layers.feature, Layer::Feature));
    }
    if layers.floor.tile.is_some() {
        return Some((&layers.floor, Layer::Floor));
    }
    None
}

fn visible_entity_on_top_excluding_character(
    layers: &LayerTable<VisibleEntity>,
) -> Option<(&VisibleEntity, Layer)> {
    if layers.item.tile.is_some() {
        return Some((&layers.item, Layer::Item));
    }
    if layers.feature.tile.is_some() {
        return Some((&layers.feature, Layer::Feature));
    }
    if layers.floor.tile.is_some() {
        return Some((&layers.floor, Layer::Floor));
    }
    None
}

fn render_meter(meter: Meter, colour: Rgb24, ctx: Ctx, fb: &mut FrameBuffer) {
    use text::*;
    let width = 15;
    let string = format!("{}/{}", meter.current(), meter.max());
    let style = Style::plain_text()
        .with_bold(true)
        .with_foreground(Rgb24::new_grey(255).to_rgba32(187));
    let centre_offset = (width / 2) - ((string.len() + 1) / 2);
    let filled_width = (meter.current() * width as u32) / meter.max();
    let filled_width = if filled_width == 0 && meter.current() > 0 {
        1
    } else {
        filled_width
    };
    for i in 0..width {
        let coord = Coord::new(i as i32, 0);
        let alpha = if i < filled_width as usize { 255 } else { 63 };
        let rc = RenderCell::default().with_background(colour.to_rgba32(alpha));
        fb.set_cell_relative_to_ctx(ctx, coord, 0, rc);
    }
    StyledString { string, style }.render(&(), ctx.add_x(centre_offset as i32), fb);
}

fn render_meter_disabled(ctx: Ctx, fb: &mut FrameBuffer) {
    use text::*;
    let width = 15;
    let string = format!("N/A");
    let centre_offset = (width / 2) - ((string.len() + 1) / 2);
    let style = Style::plain_text()
        .with_bold(true)
        .with_foreground(Rgb24::new_grey(255).to_rgba32(187));
    for i in 0..width {
        let coord = Coord::new(i as i32, 0);
        let rc = RenderCell::default().with_background(Rgba32::new_grey(63));
        fb.set_cell_relative_to_ctx(ctx, coord, 0, rc);
    }
    StyledString { string, style }.render(&(), ctx.add_x(centre_offset as i32), fb);
}

impl GameInstance {
    pub fn new<R: Rng>(
        config: &Config,
        victories: Vec<Victory>,
        rng: &mut R,
    ) -> (Self, witness::Running) {
        let (game, running) = witness::new_game(config, victories, rng);
        (GameInstance { game }, running)
    }

    pub fn into_storable(self, running: witness::Running) -> GameInstanceStorable {
        let Self { game } = self;
        let running_game = game.into_running_game(running);
        GameInstanceStorable { running_game }
    }

    fn layer_to_depth(layer: Layer) -> i8 {
        match layer {
            Layer::Character => 3,
            Layer::Item => 2,
            Layer::Feature => 1,
            Layer::Floor => 0,
        }
    }

    fn tile_to_render_cell(tile: Tile) -> RenderCell {
        match tile {
            Tile::Player => {
                return RenderCell {
                    character: Some('@'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::Street => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(Rgba32::new_grey(127)),
                };
            }
            Tile::Alley => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(Rgba32::new_grey(127)),
                };
            }
            Tile::Footpath => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FLOOR.to_rgba32(255)),
                };
            }
            Tile::Floor => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FLOOR.to_rgba32(255)),
                };
            }
            Tile::FloorBloody => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FLOOR_BLOODY.to_rgba32(255)),
                };
            }
            Tile::Wall => {
                return RenderCell {
                    character: Some('#'),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(colours::VAPORWAVE_FOREGROUND.to_rgba32(255)),
                };
            }
            Tile::Debris => {
                return RenderCell {
                    character: Some('%'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::DebrisBurning => {
                return RenderCell {
                    character: Some('%'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }

            Tile::DoorClosed => {
                return RenderCell {
                    character: Some('+'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::DoorOpen => {
                return RenderCell {
                    character: Some('-'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::StairsDown => {
                return RenderCell {
                    character: Some('>'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::StairsUp => {
                return RenderCell {
                    character: Some('<'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255)),
                };
            }
            Tile::Tentacle => {
                return RenderCell {
                    character: Some('▓'),
                    style: Style::new().with_foreground(colours::BIO.to_rgba32(255)),
                };
            }
            Tile::TentacleGlow => {
                return RenderCell {
                    character: Some('▒'),
                    style: Style::new().with_foreground(colours::BIO.to_rgba32(255)),
                };
            }
            Tile::Exit => {
                return RenderCell {
                    character: Some('Ω'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgb24::new(255, 0, 0).to_rgba32(255)),
                };
            }
            Tile::Bullet => {
                return RenderCell {
                    character: Some('●'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgb24::new(187, 187, 187).to_rgba32(255)),
                };
            }
            Tile::Money => {
                return RenderCell {
                    character: Some('$'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::MONEY.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Stimpack) => {
                return RenderCell {
                    character: Some('{'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::STIMPACK.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Antidote) => {
                return RenderCell {
                    character: Some('}'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ANTIDOTE.to_rgba32(255)),
                };
            }
            Tile::Item(Item::BloodVialEmpty) => {
                return RenderCell {
                    character: Some('['),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(colours::BLOOD_VIAL_EMPTY.to_rgba32(255)),
                };
            }
            Tile::Item(Item::BloodVialFull) => {
                return RenderCell {
                    character: Some('['),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BLOOD_VIAL_FULL.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Battery) => {
                return RenderCell {
                    character: Some('ϟ'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BATTERY.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Food) => {
                return RenderCell {
                    character: Some('*'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FOOD.to_rgba32(255)),
                };
            }
            Tile::Item(Item::AntiRads) => {
                return RenderCell {
                    character: Some(']'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ANTIRADS.to_rgba32(255)),
                };
            }

            Tile::Item(Item::OrganContainer(_)) => {
                return RenderCell {
                    character: Some('ɸ'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ORGAN_CONTAINER.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Pistol) => {
                return RenderCell {
                    character: Some('!'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::PISTOL.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Shotgun) => {
                return RenderCell {
                    character: Some('!'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SHOTGUN.to_rgba32(255)),
                };
            }
            Tile::Item(Item::RocketLauncher) => {
                return RenderCell {
                    character: Some('!'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
                };
            }
            Tile::Item(Item::PistolAmmo) => {
                return RenderCell {
                    character: Some('"'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::PISTOL.to_rgba32(255)),
                };
            }
            Tile::Item(Item::ShotgunAmmo) => {
                return RenderCell {
                    character: Some('"'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SHOTGUN.to_rgba32(255)),
                };
            }
            Tile::Item(Item::Rocket) => {
                return RenderCell {
                    character: Some('"'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
                };
            }
            Tile::Zombie => {
                return RenderCell {
                    character: Some('z'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ZOMBIE.to_rgba32(255)),
                };
            }
            Tile::Climber => {
                return RenderCell {
                    character: Some('c'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::CLIMBER.to_rgba32(255)),
                };
            }
            Tile::Trespasser => {
                return RenderCell {
                    character: Some('t'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::TRESPASSER.to_rgba32(255)),
                };
            }
            Tile::Boomer => {
                return RenderCell {
                    character: Some('b'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BOOMER.to_rgba32(255)),
                };
            }
            Tile::Snatcher => {
                return RenderCell {
                    character: Some('s'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SNATCHER.to_rgba32(255)),
                };
            }
            Tile::Corpse(npc_type) => {
                let colour = match npc_type {
                    NpcType::Zombie => colours::ZOMBIE,
                    NpcType::Climber => colours::CLIMBER,
                    NpcType::Boomer => colours::BOOMER,
                    NpcType::Trespasser => colours::TRESPASSER,
                    NpcType::Snatcher => colours::SNATCHER,
                };
                return RenderCell {
                    character: Some('?'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colour.to_rgba32(255)),
                };
            }
        };
    }

    pub fn render_game(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        let centre_coord_delta = Coord::new(0, 0);
        for coord in ctx.bounding_box.size().coord_iter_row_major() {
            let cell = self
                .game
                .inner_ref()
                .cell_visibility_at_coord(coord + centre_coord_delta);
            let unseen_background = Rgba32::new(0, 0, 0, 255);
            match cell {
                CellVisibility::Never => {
                    let render_cell = RenderCell {
                        character: None,
                        style: Style::new().with_background(unseen_background),
                    };
                    fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
                }
                CellVisibility::Previous(data) => {
                    let background = Rgba32::new(0, 0, 0, 255);
                    data.tiles.for_each_enumerate(|visible_entity, layer| {
                        if let Some(&tile) = visible_entity.tile.as_ref() {
                            let depth = Self::layer_to_depth(layer);
                            let mut render_cell = Self::tile_to_render_cell(tile);
                            render_cell.style.background = Some(background);
                            render_cell.style.foreground = Some(colours::STAIRS.to_rgba32(127));
                            fb.set_cell_relative_to_ctx(ctx, coord, depth, render_cell);
                        }
                    });
                }
                CellVisibility::Current { data, light_colour } => {
                    let light_colour = light_colour.unwrap_or(Rgb24::new_grey(0));
                    let tint = LightBlend { light_colour };
                    data.tiles.for_each_enumerate(|visible_entity, layer| {
                        if let Some(tile) = visible_entity.tile {
                            let depth = Self::layer_to_depth(layer);
                            let mut render_cell = Self::tile_to_render_cell(tile);
                            if let Some(colour_hint) = visible_entity.colour_hint {
                                render_cell = render_cell.with_foreground(colour_hint);
                            }
                            fb.set_cell_relative_to_ctx(
                                ctx_tint!(ctx, tint),
                                coord,
                                depth,
                                render_cell,
                            );
                        }
                    });
                }
            }
        }
        self.game
            .inner_ref()
            .for_each_visible_particle(|coord, visible_entity, _light_colour| {
                let mut render_cell = if let Some(tile) = visible_entity.tile {
                    Self::tile_to_render_cell(tile)
                } else {
                    RenderCell::default()
                };
                if let Some(colour_hint) = visible_entity.colour_hint {
                    render_cell = render_cell.with_background(colour_hint);
                }
                fb.set_cell_relative_to_ctx(ctx, coord, 10, render_cell);
            });
    }

    fn render_messages(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        use text::*;
        let max = 4;
        let mut messages: Vec<(usize, Message)> = Vec::new();
        for m in self.game.inner_ref().message_log().iter().rev() {
            if messages.len() >= max {
                break;
            }
            if let Some((ref mut count, last)) = messages.last_mut() {
                if last == m {
                    *count += 1;
                    continue;
                }
            }
            messages.push((1, m.clone()));
        }
        for (i, (count, m)) in messages.into_iter().enumerate() {
            let text = message_to_text(m);
            let mut text = if count == 1 {
                text
            } else {
                let mut text = text;
                text.parts
                    .push(StyledString::plain_text(format!(" (x{})", count)));
                text
            };
            let alpha = 255 - (i as u8 * 50);
            let offset = max as i32 - i as i32 - 1;
            for part in &mut text.parts {
                part.style = part.style.with_foreground(
                    part.style
                        .foreground
                        .unwrap_or_else(|| Rgba32::new(255, 255, 255, 255))
                        .with_a(alpha),
                );
            }
            text.render(&(), ctx.add_y(offset), fb);
        }
    }

    fn render_description(&self, ctx: Ctx, fb: &mut FrameBuffer, cursor: Option<Coord>) {
        use text::*;
        let (cursor, player) = if let Some(cursor) = cursor {
            if self.game.inner_ref().world_size().is_valid(cursor) {
                (cursor, false)
            } else {
                (self.game.inner_ref().player_coord(), true)
            }
        } else {
            (self.game.inner_ref().player_coord(), true)
        };
        let (visible_entity, verb, end, currently_visible) =
            match self.game.inner_ref().cell_visibility_at_coord(cursor) {
                CellVisibility::Never => {
                    Text::new(vec![StyledString {
                        string: "UNDISCOVERED LOCATION".to_string(),
                        style: Style::new().with_foreground(Rgb24::new_grey(255).to_rgba32(127)),
                    }])
                    .wrap_word()
                    .render(&(), ctx, fb);
                    return;
                }
                CellVisibility::Previous(data) => (
                    visible_entity_on_top(&data.tiles),
                    "remember seeing",
                    Some("here"),
                    false,
                ),
                CellVisibility::Current { data, .. } => {
                    if player {
                        (
                            visible_entity_on_top_excluding_character(&data.tiles),
                            "see",
                            None,
                            true,
                        )
                    } else {
                        (visible_entity_on_top(&data.tiles), "see", None, true)
                    }
                }
            };
        if let Some((visible_entity, layer)) = visible_entity {
            if let Some(tile) = visible_entity.tile {
                if player {
                    let Description {
                        mut name,
                        description,
                    } = describe_tile(tile);
                    let mut text = Text {
                        parts: vec![StyledString::plain_text(format!("There is "))],
                    };
                    text.parts.append(&mut name.parts);
                    if let Some(end) = end {
                        text.parts.push(StyledString::plain_text(format!(" {end}")));
                    }
                    text.parts
                        .push(StyledString::plain_text(" here.".to_string()));
                    if let Some(mut description) = description {
                        text.parts
                            .push(StyledString::plain_text("\n\n".to_string()));
                        text.parts.append(&mut description.parts);
                    }
                    match layer {
                        Layer::Floor => text.parts.push(StyledString {
                            string: "\n\n(Move the cursor over a tile to see a description.)"
                                .to_string(),
                            style: Style::new()
                                .with_foreground(Rgb24::new_grey(255).to_rgba32(127)),
                        }),
                        Layer::Item => text.parts.push(StyledString {
                            string: "\n\n(Press g to pick it up.)".to_string(),
                            style: Style::new()
                                .with_foreground(Rgb24::new_grey(255).to_rgba32(127)),
                        }),
                        _ => (),
                    }
                    text.wrap_word().render(&(), ctx, fb);
                } else {
                    let Description {
                        mut name,
                        description,
                    } = describe_tile(tile);
                    let mut text = Text {
                        parts: vec![StyledString::plain_text(format!("You {verb} "))],
                    };
                    text.parts.append(&mut name.parts);
                    if let Some(end) = end {
                        text.parts.push(StyledString::plain_text(format!(" {end}")));
                    }
                    text.parts.push(StyledString::plain_text(".".to_string()));
                    if currently_visible {
                        if let Some(health) = visible_entity.health {
                            if tile != Tile::Player {
                                text.parts
                                    .push(StyledString::plain_text("\n\n".to_string()));
                                text.parts
                                    .push(StyledString::plain_text(format!("Its health is ")));
                                text.parts.push(StyledString {
                                    string: format!("{}/{}", health.current(), health.max()),
                                    style: Style::default().with_bold(true).with_foreground(
                                        colours::HEALTH
                                            .to_rgba32(255)
                                            .saturating_scalar_mul_div(3, 2),
                                    ),
                                });
                            }
                        }
                    }
                    if let Some(mut description) = description {
                        text.parts
                            .push(StyledString::plain_text("\n\n".to_string()));
                        text.parts.append(&mut description.parts);
                    }
                    text.wrap_word().render(&(), ctx, fb);
                }
            }
        }
    }

    fn render_mode(&self, ctx: Ctx, fb: &mut FrameBuffer, mode: Mode) {
        use text::*;
        let text = match mode {
            Mode::Normal => Text::new(vec![StyledString::plain_text(format!(
                "Move with ←↑→↓.\nPress ? for more info."
            ))]),
            Mode::Aiming => Text::new(vec![StyledString::plain_text(format!(
                "Aim with the mouse or ←↑→↓. Click or ENTER to fire."
            ))]),
        };
        text.wrap_word().render(&(), ctx, fb);
    }

    fn render_stats(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        use text::*;
        let stats = self.game.inner_ref().player_stats();
        let x_offset = 11;
        StyledString {
            string: "Health:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        render_meter(stats.health, colours::HEALTH, ctx.add_x(x_offset), fb);
        let ctx = ctx.add_y(1);
        StyledString {
            string: "Oxygen:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        render_meter(stats.oxygen, colours::OXYGEN, ctx.add_x(x_offset), fb);
        let ctx = ctx.add_y(1);
        StyledString {
            string: "Food:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        render_meter(stats.food, colours::FOOD, ctx.add_x(x_offset), fb);
        let ctx = ctx.add_y(1);
        StyledString {
            string: "Poison:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        render_meter(stats.poison, colours::POISON, ctx.add_x(x_offset), fb);
        let ctx = ctx.add_y(1);
        StyledString {
            string: "Radiation:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        render_meter(stats.radiation, colours::RADIATION, ctx.add_x(x_offset), fb);
        let ctx = ctx.add_y(1);
        StyledString {
            string: "Power:".to_string(),
            style: Style::plain_text(),
        }
        .render(&(), ctx, fb);
        if let Some(power) = stats.power {
            render_meter(power, colours::POWER, ctx.add_x(x_offset), fb);
        } else {
            render_meter_disabled(ctx.add_x(x_offset), fb);
        }
        if let Some(satiation) = stats.satiation {
            let ctx = ctx.add_y(1);
            StyledString {
                string: "Satiation:".to_string(),
                style: Style::plain_text(),
            }
            .render(&(), ctx, fb);
            render_meter(satiation, colours::SATIATION, ctx.add_x(x_offset), fb);
        } else {
            //render_meter_disabled(ctx.add_x(x_offset), fb);
        }
    }

    fn render_info(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        use text::*;
        let (left_hand, right_hand) = self.game.inner_ref().player_hand_contents();
        Text::new(vec![
            StyledString {
                string: "L. Hand: ".to_string(),
                style: Style::plain_text(),
            },
            StyledString {
                string: left_hand,
                style: Style::plain_text().with_bold(true),
            },
        ])
        .render(&(), ctx, fb);
        let ctx = ctx.add_y(1);
        Text::new(vec![
            StyledString {
                string: "R. Hand: ".to_string(),
                style: Style::plain_text(),
            },
            StyledString {
                string: right_hand,
                style: Style::plain_text().with_bold(true),
            },
        ])
        .render(&(), ctx, fb);
        let ctx = ctx.add_y(2);
        Text::new(vec![
            StyledString {
                string: "CyberCoinz™: ".to_string(),
                style: Style::plain_text(),
            },
            StyledString {
                string: format!("{}", self.game.inner_ref().player_money()),
                style: Style::plain_text()
                    .with_bold(true)
                    .with_foreground(colours::MONEY.to_rgba32(255)),
            },
        ])
        .render(&(), ctx, fb);
    }

    pub fn render(
        &self,
        ctx: Ctx,
        fb: &mut FrameBuffer,
        cursor: Option<Coord>,
        mode: Mode,
        offset: Coord,
    ) {
        use text::*;
        self.render_game(ctx.add_offset(offset), fb);
        self.render_messages(
            ctx.add_xy(1, ctx.bounding_box.size().height() as i32 - 4)
                .add_depth(20),
            fb,
        );
        let border_style = Style::new()
            .with_bold(true)
            .with_foreground(colours::VAPORWAVE_BACKGROUND.to_rgba32(255));
        let border_text_style = Style::new()
            .with_bold(true)
            .with_foreground(colours::VAPORWAVE_FOREGROUND.to_rgba32(255));
        let game_size = self.game.inner_ref().world_size();
        let box_render_cell = RenderCell::default().with_style(border_style);
        // line to the right of game
        {
            let render_cell = box_render_cell.with_character('║');
            for i in 0..ctx.bounding_box.size().height() {
                let coord = Coord::new(game_size.width() as i32, i as i32);
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
        }
        // line under game
        {
            let render_cell = box_render_cell.with_character('═');
            for i in 0..game_size.width() {
                let coord = Coord::new(i as i32, game_size.height() as i32);
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
            Text::new(vec![
                StyledString {
                    string: "╡".to_string(),
                    style: border_style,
                },
                StyledString {
                    string: "Message Log".to_string(),
                    style: border_text_style,
                },
                StyledString {
                    string: "╞".to_string(),
                    style: border_style,
                },
            ])
            .render(&(), ctx.add_xy(2, game_size.height() as i32), fb);
        }
        fb.set_cell_relative_to_ctx(
            ctx,
            game_size.to_coord().unwrap(),
            0,
            box_render_cell.with_character('╣'),
        );
        // description
        {
            let offset_y = 21;
            let render_cell = box_render_cell.with_character('═');
            for i in (game_size.width() + 1)..ctx.bounding_box.size().width() {
                let coord = Coord::new(i as i32, offset_y);
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
            Text::new(vec![
                StyledString {
                    string: "╡".to_string(),
                    style: border_style,
                },
                StyledString {
                    string: format!("Description: "),
                    style: border_text_style,
                },
                if cursor.is_some() {
                    match mode {
                        Mode::Normal => StyledString {
                            string: format!("AT CURSOR"),
                            style: border_text_style
                                .with_foreground(colours::NORMAL_MODE.to_rgba32(255)),
                        },
                        Mode::Aiming => StyledString {
                            string: format!("AT TARGET"),
                            style: border_text_style
                                .with_foreground(colours::AIMING_MODE.to_rgba32(255)),
                        },
                    }
                } else {
                    StyledString {
                        string: format!("AT PLAYER"),
                        style: border_text_style.with_foreground(Rgba32::new_grey(255)),
                    }
                },
                StyledString {
                    string: "╞".to_string(),
                    style: border_style,
                },
            ])
            .render(&(), ctx.add_xy(game_size.width() as i32 + 1, offset_y), fb);
            fb.set_cell_relative_to_ctx(
                ctx,
                game_size.to_coord().unwrap().set_y(offset_y),
                0,
                box_render_cell.with_character('╠'),
            );
            self.render_description(
                ctx.add_offset(game_size.to_coord().unwrap().set_y(offset_y + 1))
                    .add_xy(2, 1),
                fb,
                cursor,
            );
        }
        // mode
        {
            let offset_y = 16;
            let render_cell = box_render_cell.with_character('═');
            for i in (game_size.width() + 1)..ctx.bounding_box.size().width() {
                let coord = Coord::new(i as i32, offset_y);
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
            Text::new(vec![
                StyledString {
                    string: "╡".to_string(),
                    style: border_style,
                },
                StyledString {
                    string: "Mode: ".to_string(),
                    style: border_text_style,
                },
                match mode {
                    Mode::Normal => StyledString {
                        string: "NORMAL".to_string(),
                        style: border_text_style
                            .with_foreground(colours::NORMAL_MODE.to_rgba32(255)),
                    },
                    Mode::Aiming => StyledString {
                        string: "AIMING".to_string(),
                        style: border_text_style
                            .with_foreground(colours::AIMING_MODE.to_rgba32(255)),
                    },
                },
                StyledString {
                    string: "╞".to_string(),
                    style: border_style,
                },
            ])
            .render(&(), ctx.add_xy(game_size.width() as i32 + 1, offset_y), fb);
            fb.set_cell_relative_to_ctx(
                ctx,
                game_size.to_coord().unwrap().set_y(offset_y),
                0,
                box_render_cell.with_character('╠'),
            );
            self.render_mode(
                ctx.add_offset(game_size.to_coord().unwrap().set_y(offset_y + 1))
                    .add_xy(2, 1),
                fb,
                mode,
            );
        }
        // stats
        {
            let offset_y = 6;
            let render_cell = box_render_cell.with_character('═');
            for i in (game_size.width() + 1)..ctx.bounding_box.size().width() {
                let coord = Coord::new(i as i32, offset_y);
                fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
            }
            Text::new(vec![
                StyledString {
                    string: "╡".to_string(),
                    style: border_style,
                },
                StyledString {
                    string: "Stats".to_string(),
                    style: border_text_style,
                },
                StyledString {
                    string: "╞".to_string(),
                    style: border_style,
                },
            ])
            .render(&(), ctx.add_xy(game_size.width() as i32 + 1, offset_y), fb);
            fb.set_cell_relative_to_ctx(
                ctx,
                game_size.to_coord().unwrap().set_y(offset_y),
                0,
                box_render_cell.with_character('╠'),
            );
            self.render_stats(
                ctx.add_offset(game_size.to_coord().unwrap().set_y(offset_y + 1))
                    .add_xy(2, 1),
                fb,
            );
        }
        // info
        {
            let offset_y = 0;
            self.render_info(
                ctx.add_offset(game_size.to_coord().unwrap().set_y(offset_y + 1))
                    .add_xy(2, 0),
                fb,
            );
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameInstanceStorable {
    running_game: RunningGame,
}

impl GameInstanceStorable {
    pub fn into_game_instance(self) -> (GameInstance, witness::Running) {
        let Self { running_game } = self;
        let (game, running) = running_game.into_game();
        (GameInstance { game }, running)
    }
}

struct Description {
    name: Text,
    description: Option<Text>,
}

fn describe_tile(tile: Tile) -> Description {
    use text::*;
    match tile {
        Tile::Player => Description {
            name: Text::new(vec![StyledString::plain_text("yourself".to_string())]),
            description: None,
        },
        Tile::Floor => Description {
            name: Text::new(vec![StyledString::plain_text("the floor".to_string())]),
            description: None,
        },
        Tile::FloorBloody => Description {
            name: Text::new(vec![StyledString::plain_text(
                "the floor (bloody)".to_string(),
            )]),
            description: None,
        },
        Tile::Wall => Description {
            name: Text::new(vec![StyledString::plain_text("a wall".to_string())]),
            description: None,
        },
        Tile::Street => Description {
            name: Text::new(vec![StyledString::plain_text("the street".to_string())]),
            description: None,
        },
        Tile::Alley => Description {
            name: Text::new(vec![StyledString::plain_text("an alley".to_string())]),
            description: None,
        },
        Tile::Footpath => Description {
            name: Text::new(vec![StyledString::plain_text("a sidewalk".to_string())]),
            description: None,
        },
        Tile::DoorClosed => Description {
            name: Text::new(vec![StyledString::plain_text("a closed door".to_string())]),
            description: None,
        },
        Tile::DoorOpen => Description {
            name: Text::new(vec![StyledString::plain_text("an open door".to_string())]),
            description: None,
        },
        Tile::StairsDown => Description {
            name: Text::new(vec![StyledString::plain_text(
                "a downwards elevator shaft".to_string(),
            )]),
            description: None,
        },
        Tile::StairsUp => Description {
            name: Text::new(vec![StyledString::plain_text(
                "an upwards elevator shaft".to_string(),
            )]),
            description: None,
        },
        Tile::Debris => Description {
            name: Text::new(vec![StyledString::plain_text("some debris".to_string())]),
            description: None,
        },
        Tile::DebrisBurning => Description {
            name: Text::new(vec![StyledString::plain_text(
                "some burning debris".to_string(),
            )]),
            description: None,
        },
        Tile::Tentacle | Tile::TentacleGlow => Description {
            name: Text::new(vec![StyledString::plain_text(
                "<ENTRY CORRUPT>".to_string(),
            )]),
            description: None,
        },
        Tile::Exit => Description {
            name: Text::new(vec![StyledString::plain_text("the evac zone".to_string())]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "Return here once your mission is complete!".to_string(),
            )])),
        },
        Tile::Bullet => Description {
            name: Text::new(vec![StyledString::plain_text("a bullet".to_string())]),
            description: None,
        },
        Tile::Money => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "CyberCoin™".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::MONEY.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "A unit of cybernetically-secure decentralized currency.".to_string(),
            )])),
        },
        Tile::Item(Item::Stimpack) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "stimpack".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::STIMPACK.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text("Consume to increase ".to_string()),
                StyledString {
                    string: "health".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::HEALTH.to_rgba32(255)),
                },
                StyledString::plain_text(".".to_string()),
            ])),
        },
        Tile::Item(Item::Antidote) => Description {
            name: Text::new(vec![
                StyledString::plain_text("an ".to_string()),
                StyledString {
                    string: "antidote".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ANTIDOTE.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text("Consume to decrease ".to_string()),
                StyledString {
                    string: "poison".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::POISON.to_rgba32(255)),
                },
                StyledString::plain_text(".".to_string()),
            ])),
        },
        Tile::Item(Item::BloodVialEmpty) => Description {
            name: Text::new(vec![
                StyledString::plain_text("an ".to_string()),
                StyledString {
                    string: "empty blood vial".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BLOOD_VIAL_EMPTY.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "Can be filled with blood from a corpse for later consumption.".to_string(),
            )])),
        },
        Tile::Item(Item::BloodVialFull) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "full blood vial".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BLOOD_VIAL_FULL.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text(
                    "The blood is oxygenated. Consume to increase ".to_string(),
                ),
                StyledString {
                    string: "oxygen".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::OXYGEN.to_rgba32(255)),
                },
                StyledString::plain_text(".".to_string()),
            ])),
        },
        Tile::Item(Item::Food) => Description {
            name: Text::new(vec![
                StyledString::plain_text("some ".to_string()),
                StyledString {
                    string: "food".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FOOD.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text("Consume to increase ".to_string()),
                StyledString {
                    string: "food".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::FOOD.to_rgba32(255)),
                },
                StyledString::plain_text(".".to_string()),
            ])),
        },
        Tile::Item(Item::AntiRads) => Description {
            name: Text::new(vec![
                StyledString::plain_text("some ".to_string()),
                StyledString {
                    string: "AntiRads™".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ANTIRADS.to_rgba32(255)),
                },
                StyledString::plain_text(" medication.".to_string()),
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text("Consume to decrease ".to_string()),
                StyledString {
                    string: "radiation".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::RADIATION.to_rgba32(255)),
                },
                StyledString::plain_text(".".to_string()),
            ])),
        },
        Tile::Item(Item::Battery) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "battery".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BATTERY.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![
                StyledString::plain_text("Consume to increase ".to_string()),
                StyledString {
                    string: "power".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::POWER.to_rgba32(255)),
                },
                StyledString::plain_text(" (requires CyberCore™).".to_string()),
            ])),
        },
        Tile::Item(Item::OrganContainer(organ)) => Description {
            name: Text::new(vec![
                StyledString::plain_text("an ".to_string()),
                StyledString {
                    string: "organ container".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ORGAN_CONTAINER.to_rgba32(255)),
                },
            ]),
            description: Some(if let Some(ref organ) = organ {
                Text::new(vec![StyledString::plain_text(format!(
                    "Contains a {}.",
                    organ_string_for_description(organ)
                ))])
            } else {
                Text::new(vec![StyledString::plain_text(
                    "Empty. Can be filled with an organ from a corpse.".to_string(),
                )])
            }),
        },
        Tile::Item(Item::Pistol) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "pistol".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::PISTOL.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Item(Item::Shotgun) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "shotgun".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SHOTGUN.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Item(Item::RocketLauncher) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "rocket launcher".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Item(Item::PistolAmmo) => Description {
            name: Text::new(vec![
                StyledString::plain_text("some ".to_string()),
                StyledString {
                    string: "pistol bullets".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::PISTOL.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Item(Item::ShotgunAmmo) => Description {
            name: Text::new(vec![
                StyledString::plain_text("some ".to_string()),
                StyledString {
                    string: "shotgun shells".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SHOTGUN.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Item(Item::Rocket) => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "rocket".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
                },
            ]),
            description: None,
        },
        Tile::Zombie => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "zombie".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::ZOMBIE.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "Resurrects unless its corpse is destroyed.".to_string(),
            )])),
        },
        Tile::Climber => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "climber".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::CLIMBER.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "It can climb over debris.".to_string(),
            )])),
        },
        Tile::Trespasser => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "trespasser".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::TRESPASSER.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "It knows how to open doors.".to_string(),
            )])),
        },
        Tile::Boomer => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "boomer".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::BOOMER.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "Explodes on when it dies.".to_string(),
            )])),
        },
        Tile::Snatcher => Description {
            name: Text::new(vec![
                StyledString::plain_text("a ".to_string()),
                StyledString {
                    string: "snatcher".to_string(),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(colours::SNATCHER.to_rgba32(255)),
                },
            ]),
            description: Some(Text::new(vec![StyledString::plain_text(
                "Ignores you but steals items. Drops them on death.".to_string(),
            )])),
        },
        Tile::Corpse(npc_type) => match npc_type {
            NpcType::Zombie => Description {
                name: Text::new(vec![
                    StyledString::plain_text("the corpse of a ".to_string()),
                    StyledString {
                        string: "zombie".to_string(),
                        style: Style::new()
                            .with_bold(true)
                            .with_foreground(colours::ZOMBIE.to_rgba32(255)),
                    },
                ]),
                description: Some(Text::new(vec![StyledString::plain_text(
                    "Remove the heart to prevent resurrection.".to_string(),
                )])),
            },
            NpcType::Climber => Description {
                name: Text::new(vec![
                    StyledString::plain_text("the corpse of a ".to_string()),
                    StyledString {
                        string: "climber".to_string(),
                        style: Style::new()
                            .with_bold(true)
                            .with_foreground(colours::CLIMBER.to_rgba32(255)),
                    },
                ]),
                description: None,
            },
            NpcType::Trespasser => Description {
                name: Text::new(vec![
                    StyledString::plain_text("the corpse of a ".to_string()),
                    StyledString {
                        string: "trespasser".to_string(),
                        style: Style::new()
                            .with_bold(true)
                            .with_foreground(colours::TRESPASSER.to_rgba32(255)),
                    },
                ]),
                description: None,
            },
            NpcType::Boomer => Description {
                name: Text::new(vec![
                    StyledString::plain_text("the corpse of a ".to_string()),
                    StyledString {
                        string: "boomer".to_string(),
                        style: Style::new()
                            .with_bold(true)
                            .with_foreground(colours::BOOMER.to_rgba32(255)),
                    },
                ]),
                description: None,
            },
            NpcType::Snatcher => Description {
                name: Text::new(vec![
                    StyledString::plain_text("the corpse of a ".to_string()),
                    StyledString {
                        string: "snatcher".to_string(),
                        style: Style::new()
                            .with_bold(true)
                            .with_foreground(colours::SNATCHER.to_rgba32(255)),
                    },
                ]),
                description: None,
            },
        },
    }
}

fn npc_type_to_styled_string(npc_type: NpcType) -> text::StyledString {
    use text::*;
    match npc_type {
        NpcType::Zombie => StyledString {
            string: "zombie".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ZOMBIE.to_rgba32(255)),
        },
        NpcType::Climber => StyledString {
            string: "climber".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::CLIMBER.to_rgba32(255)),
        },
        NpcType::Trespasser => StyledString {
            string: "trespasser".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::TRESPASSER.to_rgba32(255)),
        },
        NpcType::Boomer => StyledString {
            string: "boomer".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::BOOMER.to_rgba32(255)),
        },
        NpcType::Snatcher => StyledString {
            string: "snatcher".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::SNATCHER.to_rgba32(255)),
        },
    }
}

pub fn message_to_text(message: Message) -> Text {
    use text::*;
    match message {
        Message::Wait => Text::new(vec![StyledString::plain_text(
            "You wait for a turn.".to_string(),
        )]),
        Message::OpenDoor => Text::new(vec![StyledString::plain_text(
            "You open the door.".to_string(),
        )]),
        Message::CloseDoor => Text::new(vec![StyledString::plain_text(
            "You close the door.".to_string(),
        )]),
        Message::ActionError(e) => Text::new(vec![StyledString::plain_text(match e {
            ActionError::InvalidMove => format!("You can't walk there."),
            ActionError::NothingToGet => format!("There is nothing here to pick up."),
            ActionError::InventoryIsFull => {
                return Text::new(vec![
                    StyledString {
                        string: format!("Inv. is full. "),
                        style: Style::plain_text(),
                    },
                    StyledString {
                        string: format!("(Press d to drop items.)"),
                        style: Style::plain_text()
                            .with_foreground(Rgb24::new_grey(127).to_rgba32(255)),
                    },
                ]);
            }
            ActionError::NoCorpseHere => "There is no corpse here.".to_string(),
            ActionError::NoCyberCore => "You don't have a CyberCore™.".to_string(),
            ActionError::NeedsTwoHands => "Weapon requires two non-claw hands.".to_string(),
            ActionError::NeedsOneHand => "Weapon requires at least one non-claw hand.".to_string(),
            ActionError::NothingToUnequip => "No equipped item to unequip.".to_string(),
            ActionError::NothingToReload => "No suitable non-full gun equipped.".to_string(),
            ActionError::NoGun => "No gun is equippped.".to_string(),
            ActionError::OutOfLoadedAmmo => "Your equipped gun is empty.".to_string(),
            ActionError::OutOfAmmo => "No held ammo for equipped gun.".to_string(),
        })]),
        Message::NpcHit { npc_type, damage } => Text::new(vec![
            StyledString::plain_text("The ".to_string()),
            npc_type_to_styled_string(npc_type),
            StyledString::plain_text(" is hit for ".to_string()),
            StyledString {
                string: format!("{damage}"),
                style: Style::plain_text().with_bold(true),
            },
            StyledString::plain_text(" damage.".to_string()),
        ]),
        Message::NpcDies(npc_type) => Text::new(vec![
            StyledString::plain_text("The ".to_string()),
            npc_type_to_styled_string(npc_type),
            StyledString::plain_text(" dies.".to_string()),
        ]),
        Message::PlayerHit {
            attacker_npc_type,
            damage,
        } => Text::new(vec![
            StyledString::plain_text("The ".to_string()),
            npc_type_to_styled_string(attacker_npc_type),
            StyledString::plain_text(" hits you for ".to_string()),
            StyledString {
                string: format!("{damage}"),
                style: Style::plain_text().with_bold(true),
            },
            StyledString::plain_text(" damage.".to_string()),
        ]),
        Message::GetMoney => Text::new(vec![
            StyledString::plain_text("You pick up the ".to_string()),
            StyledString {
                string: format!("CyberCoin™"),
                style: Style::plain_text()
                    .with_bold(true)
                    .with_foreground(colours::MONEY.to_rgba32(255)),
            },
            StyledString::plain_text(".".to_string()),
        ]),
        Message::GetItem(item) => Text::new(vec![
            StyledString::plain_text("You pick up the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
        Message::DropItem(item) => Text::new(vec![
            StyledString::plain_text("You drop the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
        Message::UnequipItem(item) => Text::new(vec![
            StyledString::plain_text("You unequip the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
        Message::DropUnequipItem(item) => Text::new(vec![
            StyledString::plain_text("You drop the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(" (inventory full).".to_string()),
        ]),
        Message::EquipItem(item) => Text::new(vec![
            StyledString::plain_text("You equip the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
        Message::ReloadGun(item) => Text::new(vec![
            StyledString::plain_text("You reload the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
        Message::FireGun(item) => Text::new(vec![
            StyledString::plain_text("You fire the ".to_string()),
            item_styled_string_for_message(item),
            StyledString::plain_text(".".to_string()),
        ]),
    }
}

pub fn organ_type_name(organ_type: OrganType) -> &'static str {
    use OrganType::*;
    match organ_type {
        Heart => "heart",
        Liver => "liver",
        Lung => "lung",
        Stomach => "stomach",
        Appendix => "appendix",
        Tumour => "tumour",
        CronenbergPistol => "cronenberg pistol",
        CronenbergShotgun => "cronenberg shotgun",
        CyberCore => "CyberCore™",
        Claw => "claw™",
    }
}

pub fn organ_type_name_cap(organ_type: OrganType) -> &'static str {
    use OrganType::*;
    match organ_type {
        Heart => "Heart",
        Liver => "Liver",
        Lung => "Lung",
        Stomach => "Stomach",
        Appendix => "Appendix",
        Tumour => "tumour",
        CronenbergPistol => "Cronenberg pistol",
        CronenbergShotgun => "Cronenberg shotgun",
        CyberCore => "CyberCore™",
        Claw => "Claw™",
    }
}

pub fn organ_trait_name(organ_trait: OrganTrait) -> &'static str {
    use OrganTrait::*;
    match organ_trait {
        Prolific => "prolific",
        Vampiric => "vampiric",
        Radioactitve => "radioactive",
        Damaged => "damaged",
        Transient => "transient",
        Embedded => "embedded",
    }
}

pub fn organ_traits_string(organ_traits: OrganTraits) -> String {
    let traits = organ_traits.traits();
    if traits.is_empty() {
        "".to_string()
    } else {
        let num_traits = traits.len();
        let mut string = " (".to_string();
        for (i, trait_) in traits.into_iter().enumerate() {
            string.push_str(organ_trait_name(trait_));
            if i == num_traits - 1 {
                string.push(')');
            } else {
                string.push_str(", ");
            }
        }
        string
    }
}

fn organ_string_for_description(organ: &Organ) -> String {
    let article = match organ.type_ {
        OrganType::Appendix => "an",
        _ => "a",
    };
    let cybernetic = if organ.cybernetic { " cybernetic" } else { "" };
    format!(
        "{article}{cybernetic} {}{}",
        organ_type_name(organ.type_),
        organ_traits_string(organ.traits)
    )
}

pub fn organ_string_for_menu(organ: &Organ) -> String {
    let cybernetic = if organ.cybernetic { "Cybernetic " } else { "" };
    format!(
        "{cybernetic}{}{}",
        organ_type_name_cap(organ.type_),
        organ_traits_string(organ.traits)
    )
}

fn item_styled_string_for_message(item: Item) -> text::StyledString {
    use text::*;
    match item {
        Item::Stimpack => StyledString {
            string: "stimpack".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::STIMPACK.to_rgba32(255)),
        },
        Item::Antidote => StyledString {
            string: "antidote".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ANTIDOTE.to_rgba32(255)),
        },
        Item::BloodVialEmpty => StyledString {
            string: "empty blood vial".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::BLOOD_VIAL_EMPTY.to_rgba32(255)),
        },
        Item::BloodVialFull => StyledString {
            string: "full blood vial".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::BLOOD_VIAL_FULL.to_rgba32(255)),
        },
        Item::Food => StyledString {
            string: "food".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::FOOD.to_rgba32(255)),
        },
        Item::AntiRads => StyledString {
            string: "AntiRads™".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ANTIRADS.to_rgba32(255)),
        },
        Item::Battery => StyledString {
            string: "battery".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::BATTERY.to_rgba32(255)),
        },
        Item::OrganContainer(None) => StyledString {
            string: "empty organ container".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ORGAN_CONTAINER.to_rgba32(255)),
        },
        Item::OrganContainer(Some(_)) => StyledString {
            string: "full organ container".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ORGAN_CONTAINER.to_rgba32(255)),
        },
        Item::Pistol => StyledString {
            string: "pistol".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::PISTOL.to_rgba32(255)),
        },
        Item::Shotgun => StyledString {
            string: "shotgun".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::SHOTGUN.to_rgba32(255)),
        },
        Item::RocketLauncher => StyledString {
            string: "rocket launcher".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
        },
        Item::PistolAmmo => StyledString {
            string: "pistol bullets".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::PISTOL.to_rgba32(255)),
        },
        Item::ShotgunAmmo => StyledString {
            string: "shotgun shells".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::SHOTGUN.to_rgba32(255)),
        },
        Item::Rocket => StyledString {
            string: "rocket".to_string(),
            style: Style::new()
                .with_bold(true)
                .with_foreground(colours::ROCKET_LAUNCHER.to_rgba32(255)),
        },
    }
}

pub fn item_string_for_menu(item: Item) -> String {
    match item {
        Item::Stimpack => "Stimpack".to_string(),
        Item::Antidote => "Antidote".to_string(),
        Item::BloodVialEmpty => "Blood Vial (empty)".to_string(),
        Item::BloodVialFull => "Blood Vial (full)".to_string(),
        Item::Food => "Food".to_string(),
        Item::AntiRads => "AntiRads™".to_string(),
        Item::Battery => "Battery".to_string(),
        Item::OrganContainer(None) => "Organ Container (empty)".to_string(),
        Item::OrganContainer(Some(organ)) => format!(
            "Organ Container with {}",
            organ_string_for_description(&organ)
        ),
        Item::Pistol => "Pistol".to_string(),
        Item::Shotgun => "Shotgun".to_string(),
        Item::RocketLauncher => "Rocket Launcher".to_string(),
        Item::PistolAmmo => "Pistol Bullets".to_string(),
        Item::ShotgunAmmo => "Shotgun Shells".to_string(),
        Item::Rocket => "Rocket".to_string(),
    }
}
