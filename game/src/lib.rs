pub use direction::{CardinalDirection, Direction};
pub use entity_table::{entity_data, entity_update, ComponentTable, Entity};
pub use grid_2d::{Coord, Grid, Size};
pub use grid_search_cardinal_distance_map as distance_map;
pub use line_2d::{self, coords_between, coords_between_cardinal};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
pub use rgb_int::{Rgb24, Rgba32};
use serde::{Deserialize, Serialize};
pub use shadowcast::Context as ShadowcastContext;
pub use spatial_table::UpdateError;
use std::time::Duration;

pub use visible_area_detection::{
    vision_distance::Circle, CellVisibility, Light, VisibilityGrid, World as VisibleWorld,
};

mod terrain;
mod world;
use terrain::Terrain;
mod realtime;
pub mod witness;

use realtime::AnimationContext;
pub use world::data::{Layer, Location, Meter, Tile};
use world::{
    data::{Components, DoorState, EntityData, EntityUpdate},
    spatial::{LayerTable, Layers, SpatialTable},
    World,
};

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

/// Events which the game can report back to the io layer so it can
/// respond with a sound/visual effect.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ExternalEvent {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Dummy,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuImage {}

#[derive(Debug, Clone, Copy)]
pub enum MenuChoice {
    Dummy,
}

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
pub struct VisibleEntity {
    pub tile: Option<Tile>,
    pub colour_hint: Option<Rgba32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VisibleCellData {
    pub tiles: LayerTable<VisibleEntity>,
}
impl Default for VisibleCellData {
    fn default() -> Self {
        Self {
            tiles: LayerTable {
                floor: Default::default(),
                feature: Default::default(),
                character: Default::default(),
                item: Default::default(),
            },
        }
    }
}

impl VisibleCellData {
    fn update(&mut self, world: &World, coord: Coord) {
        let layers = world.spatial_table.layers_at_checked(coord);
        self.tiles = layers.map(|&entity| {
            entity
                .map(|entity| {
                    let tile = world.components.tile.get(entity).cloned();
                    let colour_hint = world.components.colour_hint.get(entity).cloned();
                    VisibleEntity { tile, colour_hint }
                })
                .unwrap_or_default()
        });
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

    fn for_each_light_by_coord<F: FnMut(Coord, &Light<Self::VisionDistance>)>(&self, mut f: F) {
        for (entity, light) in self.components.light.iter() {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                f(coord, light);
            }
        }
    }
}

pub enum ActionError {}

#[derive(Serialize, Deserialize, Default)]
struct AiCtx {
    distance_map: distance_map::PopulateContext,
}

#[derive(Serialize, Deserialize)]
struct Level {
    world: World,
    visibility_grid: VisibilityGrid<VisibleCellData>,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    current_level_index: usize,
    other_levels: Vec<Option<Level>>,
    world: World,
    visibility_grid: VisibilityGrid<VisibleCellData>,
    rng: Isaac64Rng,
    animation_rng: Isaac64Rng,
    player_entity: Entity,
    message_log: Vec<Message>,
    ai_ctx: AiCtx,
    animation_context: AnimationContext,
    omniscient: bool,
    external_events: Vec<ExternalEvent>,
}

pub const NUM_LEVELS: usize = 6;

impl Game {
    pub fn new<R: Rng>(config: &Config, _victories: Vec<Victory>, base_rng: &mut R) -> Self {
        let mut rng = Isaac64Rng::seed_from_u64(base_rng.gen());
        let animation_rng = Isaac64Rng::seed_from_u64(base_rng.gen());
        let mut other_levels = (0..NUM_LEVELS)
            .map(|i| {
                let Terrain { world } = Terrain::generate(i, &mut rng);
                let visibility_grid = VisibilityGrid::new(world.spatial_table.grid_size());
                Some(Level {
                    world,
                    visibility_grid,
                })
            })
            .collect::<Vec<_>>();
        let current_level_index = 0;
        let Level {
            mut world,
            visibility_grid,
        } = other_levels[current_level_index].take().unwrap();
        let player_spawn = world.stairs_up_or_exit_coord().unwrap();
        let player_data = world::spawn::make_player();
        let player_location = Location {
            coord: player_spawn,
            layer: Some(Layer::Character),
        };
        let player_entity = world.insert_entity_data(player_location, player_data);
        let mut game = Self {
            current_level_index,
            other_levels,
            world,
            visibility_grid,
            rng,
            animation_rng,
            player_entity,
            message_log: Vec::new(),
            ai_ctx: Default::default(),
            animation_context: Default::default(),
            omniscient: config.omniscient.is_some(),
            external_events: Default::default(),
        };
        game.update_visibility();
        game
    }

    pub fn enter_level(&mut self, level_index: usize) {
        use std::mem;
        assert!(
            level_index == self.current_level_index + 1
                || level_index == self.current_level_index - 1
        );
        let down = level_index == self.current_level_index + 1;
        let player_data = self.world.remove_entity(self.player_entity);
        let mut level = self.other_levels[level_index].take().unwrap();
        mem::swap(&mut self.world, &mut level.world);
        mem::swap(&mut self.visibility_grid, &mut level.visibility_grid);
        self.other_levels[self.current_level_index] = Some(level);
        self.current_level_index = level_index;
        let player_coord = if down {
            self.world.stairs_up_or_exit_coord().unwrap()
        } else {
            self.world.stairs_down_coord().unwrap()
        };
        self.player_entity = self.world.insert_entity_data(
            Location {
                layer: Some(Layer::Character),
                coord: player_coord,
            },
            player_data,
        );
        self.update_visibility();
    }

    pub fn message_log(&self) -> &[Message] {
        &self.message_log
    }

    pub fn update_visibility(&mut self) {
        let update_fn = |data: &mut VisibleCellData, coord| {
            data.update(&self.world, coord);
        };
        if self.omniscient {
            self.visibility_grid.update_omniscient_custom(
                Rgb24::new_grey(255),
                &self.world,
                update_fn,
            );
        } else {
            let distance = Circle::new_squared(200);
            self.visibility_grid.update_custom(
                Rgb24::new_grey(0),
                &self.world,
                distance,
                self.player_coord(),
                update_fn,
            );
        }
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
                solid_for_particles: None,
                opacity: None,
            },
        );
    }

    fn open_door_entity_adjacent_to_coord(
        &self,
        coord: Coord,
        dest_coord: Coord,
    ) -> Option<Entity> {
        for direction in Direction::all() {
            let potential_door_coord = coord + direction.coord();
            let delta = dest_coord - potential_door_coord;
            if delta.x.abs() <= 1 && delta.y.abs() <= 1 {
                if let Some(&Layers {
                    feature: Some(feature_entity),
                    ..
                }) = self.world.spatial_table.layers_at(potential_door_coord)
                {
                    if let Some(DoorState::Open) =
                        self.world.components.door_state.get(feature_entity)
                    {
                        return Some(feature_entity);
                    }
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
                solid_for_particles: (),
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
                    self.open_door_entity_adjacent_to_coord(player_coord, new_player_coord)
                {
                    self.close_door(open_door_entity);
                }
                return None;
            }
        }
        self.world
            .spatial_table
            .update_coord(self.player_entity, new_player_coord)
            .unwrap();
        self.change_level_if_player_is_on_stairs();
        None
    }

    fn change_level_if_player_is_on_stairs(&mut self) {
        let player_coord = self.player_coord();
        if let Some(feature_entity) = self
            .world
            .spatial_table
            .layers_at_checked(player_coord)
            .feature
        {
            if self.world.components.stairs_down.contains(feature_entity) {
                self.enter_level(self.current_level_index + 1);
            } else if self.world.components.stairs_up.contains(feature_entity) {
                self.enter_level(self.current_level_index - 1);
            }
        }
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
        self.animation_context.tick(
            &mut self.world,
            &mut self.external_events,
            &mut self.message_log,
            &mut self.animation_rng,
        );
        self.update_visibility();
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

    pub(crate) fn handle_choice(&mut self, _choice: MenuChoice) -> Option<GameControlFlow> {
        None
    }

    pub fn for_each_visible_particle<F: FnMut(Coord, VisibleEntity, Option<Rgb24>)>(
        &self,
        mut f: F,
    ) {
        for entity in self.world.components.particle.entities() {
            if let Some(coord) = self.world.spatial_table.coord_of(entity) {
                if let CellVisibility::Current { light_colour, .. } =
                    self.cell_visibility_at_coord(coord)
                {
                    let visible_entity = VisibleEntity {
                        tile: self.world.components.tile.get(entity).cloned(),
                        colour_hint: self.world.components.colour_hint.get(entity).cloned(),
                    };
                    f(coord, visible_entity, light_colour);
                }
            }
        }
    }
}
