use crate::colours;
use chargrid::{prelude::*, text};
use game::{
    witness::{self, Game, RunningGame},
    CellVisibility, Config, Layer, Tile, Victory,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub struct GameInstance {
    pub game: Game,
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
                        .with_foreground(Rgba32::new_grey(127))
                        .with_background(colours::STREET_BACKGROUND.to_rgba32(255)),
                };
            }
            Tile::Alley => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(Rgba32::new_grey(127))
                        .with_background(colours::STREET_BACKGROUND.to_rgba32(255)),
                };
            }
            Tile::Footpath => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::VAPORWAVE_BACKGROUND.to_rgba32(255)),
                };
            }
            Tile::Floor => {
                return RenderCell {
                    character: Some('.'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::VAPORWAVE_BACKGROUND.to_rgba32(255)),
                };
            }
            Tile::Wall => {
                return RenderCell {
                    character: Some('#'),
                    style: Style::new()
                        .with_bold(false)
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::VAPORWAVE_BACKGROUND.to_rgba32(255)),
                };
            }
            Tile::Debris => {
                return RenderCell {
                    character: Some('%'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::DAMAGED_BACKGROUND.to_rgba32(255)),
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
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::STAIRS.to_rgba32(255)),
                };
            }
            Tile::StairsUp => {
                return RenderCell {
                    character: Some('<'),
                    style: Style::new()
                        .with_bold(true)
                        .with_foreground(Rgba32::new_grey(255))
                        .with_background(colours::STAIRS.to_rgba32(255)),
                };
            }
            Tile::Tentacle => {
                return RenderCell {
                    character: Some('█'),
                    style: Style::new().with_foreground(colours::BIO.to_rgba32(255)),
                };
            }
            Tile::TentacleGlow => {
                return RenderCell {
                    character: Some('▒'),
                    style: Style::new().with_foreground(colours::BIO.to_rgba32(255)),
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
                    data.tiles.for_each_enumerate(|tile, layer| {
                        if let Some(&tile) = tile.as_ref() {
                            let depth = Self::layer_to_depth(layer);
                            let mut render_cell = Self::tile_to_render_cell(tile);
                            render_cell.style.background = Some(background);
                            render_cell.style.foreground = Some(Rgba32::new_grey(63));
                            fb.set_cell_relative_to_ctx(ctx, coord, depth, render_cell);
                        }
                    });
                }
                CellVisibility::Current { data, .. } => {
                    data.tiles.for_each_enumerate(|tile, layer| {
                        if let Some(&tile) = tile.as_ref() {
                            let depth = Self::layer_to_depth(layer);
                            let render_cell = Self::tile_to_render_cell(tile);
                            fb.set_cell_relative_to_ctx(ctx, coord, depth, render_cell);
                        }
                    });
                }
            }
        }
    }

    fn render_messages(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        use text::*;
        let max = 4;
        let mut messages: Vec<(usize, String)> = Vec::new();
        for m in self.game.inner_ref().messages().iter().rev() {
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
            let string = if count == 1 {
                m
            } else {
                format!("{} (x{})", m, count)
            };
            let alpha = 255 - (i as u8 * 50);
            let styled_string = StyledString {
                string,
                style: Style::plain_text().with_foreground(Rgba32::new_grey(255).with_a(alpha)),
            };
            let offset = max as i32 - i as i32 - 1;
            styled_string.render(&(), ctx.add_y(offset), fb);
        }
    }

    pub fn render(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        self.render_game(ctx, fb);
        self.render_messages(
            ctx.add_xy(1, ctx.bounding_box.size().height() as i32 - 7)
                .add_depth(20),
            fb,
        );
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
