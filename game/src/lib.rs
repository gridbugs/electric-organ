pub use direction::{CardinalDirection, Direction};
pub use entity_table::{entity_data, entity_update, ComponentTable, Entity};
pub use grid_2d::{Coord, Grid, Size};
pub use grid_search_cardinal_distance_map as distance_map;
pub use line_2d::{self, coords_between, coords_between_cardinal};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
pub use rgb_int::Rgb24;
use serde::{Deserialize, Serialize};
pub use shadowcast::Context as ShadowcastContext;
pub use spatial_table::UpdateError;
use std::time::Duration;

pub mod witness;
mod world;

pub use visible_area_detection::{
    vision_distance::Circle, CellVisibility, VisibilityGrid, World as VisibleWorld,
};
pub use world::data::{Layer, Location, Meter, Tile};
use world::{
    data::{Components, DoorState, EntityData, EntityUpdate},
    spatial::{LayerTable, Layers, SpatialTable},
    World,
};

mod terrain;
use terrain::Terrain;

#[derive(Debug, Clone, Copy)]
pub struct Omniscient;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub omniscient: Option<Omniscient>,
    pub demo: bool,
    pub debug: bool,
}
impl Config {
    pub const OMNISCIENT: Option<Omniscient> = Some(Omniscient);
}
impl Default for Config {
    fn default() -> Self {
        Self {
            omniscient: None,
            demo: false,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuImage {}

#[derive(Debug, Clone, Copy)]
pub enum MenuChoice {}

#[derive(Debug, Clone)]
pub struct Menu {
    pub choices: Vec<MenuChoice>,
    pub text: String,
    pub image: MenuImage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Victory {}

#[derive(Debug, Clone, Copy)]
pub enum GameOverReason {}

#[derive(Debug)]
pub enum GameControlFlow {
    GameOver(GameOverReason),
    Win,
    Menu(Menu),
}

#[derive(Clone, Copy, Debug)]
pub enum Input {
    Walk(CardinalDirection),
    Wait,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct VisibleCellData {
    pub tiles: LayerTable<Option<Tile>>,
}

impl VisibleCellData {
    fn update(&mut self, world: &World, coord: Coord) {
        let layers = world.spatial_table.layers_at_checked(coord);
        self.tiles = layers.option_and_then(|&entity| world.components.tile.get(entity).cloned());
    }
}

impl VisibleWorld for World {
    type VisionDistance = Circle;

    fn size(&self) -> Size {
        self.spatial_table.grid_size()
    }

    fn get_opacity(&self, coord: Coord) -> u8 {
        if let Some(&Layers {
            feature: Some(feature_entity),
            ..
        }) = self.spatial_table.layers_at(coord)
        {
            self.components
                .opacity
                .get(feature_entity)
                .cloned()
                .unwrap_or(0)
        } else {
            0
        }
    }
}

pub enum ActionError {}

#[derive(Serialize, Deserialize, Default)]
struct AiCtx {
    distance_map: distance_map::PopulateContext,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    world: World,
    rng: Isaac64Rng,
    player_entity: Entity,
    visibility_grid: VisibilityGrid<VisibleCellData>,
    messages: Vec<String>,
    ai_ctx: AiCtx,
}

impl Game {
    pub fn new<R: Rng>(_config: &Config, _victories: Vec<Victory>, base_rng: &mut R) -> Self {
        let rng = Isaac64Rng::seed_from_u64(base_rng.gen());
        let Terrain {
            world,
            player_entity,
        } = Terrain::generate_text(world::spawn::make_player());
        let mut game = Self {
            rng,
            visibility_grid: VisibilityGrid::new(world.spatial_table.grid_size()),
            world,
            player_entity,
            messages: Vec::new(),
            ai_ctx: Default::default(),
        };
        game.update_visibility();
        game
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    pub fn update_visibility(&mut self) {
        let update_fn = |data: &mut VisibleCellData, coord| {
            data.update(&self.world, coord);
        };
        let distance = Circle::new_squared(150);
        self.visibility_grid.update_custom(
            Rgb24::new_grey(255),
            &self.world,
            distance,
            self.player_coord(),
            update_fn,
        );
    }

    pub fn cell_visibility_at_coord(&self, coord: Coord) -> CellVisibility<&VisibleCellData> {
        self.visibility_grid.get_visibility(coord)
    }

    /// Returns the coordinate of the player character
    pub fn player_coord(&self) -> Coord {
        self.world
            .spatial_table
            .coord_of(self.player_entity)
            .expect("player does not have coord")
    }

    fn open_door(&mut self, entity: Entity) {
        self.world.components.apply_entity_update(
            entity,
            entity_update! {
                door_state: Some(DoorState::Open),
                tile: Some(Tile::DoorOpen),
                solid: None,
                opacity: None,
            },
        );
    }

    fn open_door_entity_adjacent_to_coord(&self, coord: Coord) -> Option<Entity> {
        for direction in Direction::all() {
            let potential_door_coord = coord + direction.coord();
            if let Some(&Layers {
                feature: Some(feature_entity),
                ..
            }) = self.world.spatial_table.layers_at(potential_door_coord)
            {
                if let Some(DoorState::Open) = self.world.components.door_state.get(feature_entity)
                {
                    return Some(feature_entity);
                }
            }
        }
        None
    }

    fn close_door(&mut self, entity: Entity) {
        self.world.components.insert_entity_data(
            entity,
            entity_data! {
                door_state: DoorState::Closed,
                tile: Tile::DoorClosed,
                solid: (),
                opacity: 255,
            },
        );
    }

    fn player_walk(&mut self, direction: CardinalDirection) -> Option<GameControlFlow> {
        let player_coord = self.player_coord();
        let new_player_coord = player_coord + direction.coord();
        if !new_player_coord.is_valid(self.world.size()) {
            // player would walk outside bounds of map
            return None;
        }
        if let Some(&Layers {
            feature: Some(feature_entity),
            ..
        }) = self.world.spatial_table.layers_at(new_player_coord)
        {
            // If the player bumps into a door, open the door
            if let Some(DoorState::Closed) = self.world.components.door_state.get(feature_entity) {
                self.open_door(feature_entity);
                return None;
            }
            // Don't let the player walk through solid entities
            if self.world.components.solid.contains(feature_entity) {
                if let Some(open_door_entity) =
                    self.open_door_entity_adjacent_to_coord(player_coord)
                {
                    self.close_door(open_door_entity);
                }
                return None;
            }
            // Exercise win logic
            if self.world.components.stairs_down.contains(feature_entity) {
                return Some(GameControlFlow::Win);
            }
        }
        self.world
            .spatial_table
            .update_coord(self.player_entity, new_player_coord)
            .unwrap();
        None
    }

    fn npc_turn(&mut self) -> Option<GameControlFlow> {
        {
            struct C<'a> {
                components: &'a Components,
                spatial_table: &'a SpatialTable,
            }
            impl<'a> distance_map::CanEnter for C<'a> {
                fn can_enter(&self, coord: Coord) -> bool {
                    if let Some(&layers) = self.spatial_table.layers_at(coord) {
                        if let Layers {
                            feature: Some(feature),
                            ..
                        } = layers
                        {
                            if self.components.solid.contains(feature) {
                                return false;
                            }
                        }
                        if let Layers { floor: None, .. } = layers {
                            return false;
                        }
                    }
                    true
                }
            }
            self.ai_ctx.distance_map.clear();
            self.ai_ctx.distance_map.add(self.player_coord());
            let c = C {
                components: &self.world.components,
                spatial_table: &self.world.spatial_table,
            };
            self.ai_ctx
                .distance_map
                .populate_approach(&c, 12, &mut self.world.distance_map);
        }
        None
    }

    #[must_use]
    pub(crate) fn handle_tick(
        &mut self,
        _since_last_tick: Duration,
        _config: &Config,
    ) -> Option<GameControlFlow> {
        None
    }

    fn pass_time(&mut self) {}

    #[must_use]
    pub(crate) fn handle_input(
        &mut self,
        input: Input,
        _config: &Config,
    ) -> Result<Option<GameControlFlow>, ActionError> {
        let game_control_flow = match input {
            Input::Walk(direction) => self.player_walk(direction),
            Input::Wait => {
                self.pass_time();
                None
            }
        };
        if game_control_flow.is_some() {
            return Ok(game_control_flow);
        }
        let game_control_flow = self.npc_turn();
        if game_control_flow.is_some() {
            return Ok(game_control_flow);
        }
        self.update_visibility();
        Ok(None)
    }

    fn is_coord_visible(&self, coord: Coord) -> bool {
        match self.cell_visibility_at_coord(coord) {
            CellVisibility::Current { .. } => true,
            CellVisibility::Previous(_) => true,
            _ => false,
        }
    }

    pub(crate) fn handle_choice(&mut self, choice: MenuChoice) -> Option<GameControlFlow> {
        self.update_visibility();
        None
    }
}
