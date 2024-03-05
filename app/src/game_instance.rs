use crate::colours;
use chargrid::{
    prelude::*,
    text::{self, Text},
};
use game::{
    witness::{self, Game, RunningGame},
    ActionError, CellVisibility, Config, Layer, LayerTable, Message, Meter, Tile, Victory,
    VisibleEntity,
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

fn visible_entity_on_top(layers: &LayerTable<VisibleEntity>) -> Option<&VisibleEntity> {
    if layers.character.tile.is_some() {
        return Some(&layers.character);
    }
    if layers.item.tile.is_some() {
        return Some(&layers.item);
    }
    if layers.feature.tile.is_some() {
        return Some(&layers.feature);
    }
    if layers.floor.tile.is_some() {
        return Some(&layers.floor);
    }
    None
}

fn render_meter(meter: Meter, colour: Rgb24, ctx: Ctx, fb: &mut FrameBuffer) {
    use text::*;
    let width = 10;
    let text = format!("{}/{}", meter.current(), meter.max());
    let centre_offset = (width / 2) - (text.len() / 2);
    StyledString::plain_text(text).render(&(), ctx.add_y(centre_offset as i32), fb);
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
            Tile::Bullet => {
                return RenderCell {
                    character: Some('●'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgb24::new(187, 187, 187).to_rgba32(255)),
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
        if let Some(cursor) = cursor {
            if self.game.inner_ref().world_size().is_valid(cursor) {
                let (visible_entity, verb, end, currently_visible) =
                    match self.game.inner_ref().cell_visibility_at_coord(cursor) {
                        CellVisibility::Never => {
                            Text::new(vec![StyledString {
                                string: "UNDISCOVERED LOCATION".to_string(),
                                style: Style::new()
                                    .with_foreground(Rgb24::new_grey(255).to_rgba32(127)),
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
                            (visible_entity_on_top(&data.tiles), "see", None, true)
                        }
                    };
                if let Some(visible_entity) = visible_entity {
                    if let Some(tile) = visible_entity.tile {
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
                                text.parts
                                    .push(StyledString::plain_text("\n\n".to_string()));
                                text.parts
                                    .push(StyledString::plain_text(format!("Its health is ")));
                                text.parts.push(StyledString {
                                    string: format!("{}/{}", health.current(), health.max()),
                                    style: Style::default()
                                        .with_bold(true)
                                        .with_foreground(colours::HEALTH.to_rgba32(255)),
                                });
                            }
                        }
                        if let Some(mut description) = description {
                            text.parts
                                .push(StyledString::plain_text("\n\n".to_string()));
                            text.parts.append(&mut description.parts);
                        }
                        text.wrap_word().render(&(), ctx, fb);
                        return;
                    }
                }
            }
        }
        Text::new(vec![StyledString {
            string: "(Move the cursor over a tile to see a description.)".to_string(),
            style: Style::new().with_foreground(Rgb24::new_grey(255).to_rgba32(127)),
        }])
        .wrap_word()
        .render(&(), ctx, fb);
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

    pub fn render(&self, ctx: Ctx, fb: &mut FrameBuffer, cursor: Option<Coord>, mode: Mode) {
        use text::*;
        self.render_game(ctx, fb);
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
            let offset_y = 20;
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
                    string: "Description".to_string(),
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
            self.render_description(
                ctx.add_offset(game_size.to_coord().unwrap().set_y(offset_y + 1))
                    .add_xy(2, 1),
                fb,
                cursor,
            );
        }
        // mode
        {
            let offset_y = 15;
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
            description: None,
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
    }
}

fn message_to_text(message: Message) -> Text {
    use text::*;
    match message {
        Message::OpenDoor => Text::new(vec![StyledString::plain_text(
            "You open the door.".to_string(),
        )]),
        Message::CloseDoor => Text::new(vec![StyledString::plain_text(
            "You close the door.".to_string(),
        )]),
        Message::ActionError(ActionError::InvalidMove) => {
            Text::new(vec![StyledString::plain_text(
                "You can't walk there.".to_string(),
            )])
        }
    }
}
