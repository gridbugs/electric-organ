pub use direction::{CardinalDirection, Direction};
pub use entity_table::{entity_data, entity_update, ComponentTable, Entity};
pub use grid_2d::{Coord, Grid, Size};
pub use grid_search_cardinal::distance_map;
pub use line_2d::{self, coords_between, coords_between_cardinal};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
pub use rgb_int::{Rgb24, Rgba32};
use serde::{Deserialize, Serialize};
pub use spatial_table::UpdateError;
use std::time::Duration;

pub use visible_area_detection::{
    vision_distance::Circle, CellVisibility, Light, VisibilityGrid, World as VisibleWorld,
};

mod terrain;
mod world;
use terrain::Terrain;
mod ai;
mod realtime;
pub mod witness;

use ai::{Agent, AiContext};
use realtime::AnimationContext;
use world::{
    data::{DoorState, EntityData, EntityUpdate},
    spatial::Layers,
    World,
};
pub use world::{
    data::{
        Item, Layer, Location, Meter, NpcType, Organ, OrganTrait, OrganTraits, OrganType, Tile,
    },
    spatial::LayerTable,
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
pub enum ExternalEvent {
    FirePistol,
    FireShotgun,
    FireRocket,
    Explosion(Coord),
    ChangeLevel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Wait,
    OpenDoor,
    CloseDoor,
    ActionError(ActionError),
    NpcHit {
        npc_type: NpcType,
        damage: u32,
    },
    NpcDies(NpcType),
    PlayerHit {
        attacker_npc_type: NpcType,
        damage: u32,
    },
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

pub struct PlayerStats {
    pub health: Meter,
    pub oxygen: Meter,
    pub food: Meter,
    pub poison: Meter,
    pub radiation: Meter,
    pub power: Option<Meter>,
}

#[derive(Clone, Copy, Debug)]
pub enum Input {
    Walk(CardinalDirection),
    Wait,
    FireEquipped(Coord),
    Get,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct VisibleEntity {
    pub tile: Option<Tile>,
    pub colour_hint: Option<Rgba32>,
    pub health: Option<Meter>,
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
                    let health = world.components.health.get(entity).cloned();
                    VisibleEntity {
                        tile,
                        colour_hint,
                        health,
                    }
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
        Self::get_opacity(self, coord)
    }

    fn for_each_light_by_coord<F: FnMut(Coord, &Light<Self::VisionDistance>)>(&self, mut f: F) {
        for (entity, light) in self.components.light.iter() {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                f(coord, light);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    InvalidMove,
    NothingToGet,
    InventoryIsFull,
}

#[derive(Serialize, Deserialize)]
struct Level {
    world: World,
    visibility_grid: VisibilityGrid<VisibleCellData>,
    agents: ComponentTable<Agent>,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    current_level_index: usize,
    other_levels: Vec<Option<Level>>,
    world: World,
    visibility_grid: VisibilityGrid<VisibleCellData>,
    agents: ComponentTable<Agent>,
    rng: Isaac64Rng,
    animation_rng: Isaac64Rng,
    player_entity: Entity,
    message_log: Vec<Message>,
    ai_context: AiContext,
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
                    agents: Default::default(),
                })
            })
            .collect::<Vec<_>>();
        let current_level_index = 0;
        let Level {
            mut world,
            visibility_grid,
            agents,
        } = other_levels[current_level_index].take().unwrap();
        let player_spawn = world.stairs_up_or_exit_coord().unwrap();
        let player_data = world::spawn::make_player();
        let player_location = Location {
            coord: player_spawn,
            layer: Some(Layer::Character),
        };
        let player_entity = world.insert_entity_data(player_location, player_data);
        let mut game = Self {
            ai_context: AiContext::new(world.size()),
            current_level_index,
            other_levels,
            world,
            visibility_grid,
            agents,
            rng,
            animation_rng,
            player_entity,
            message_log: Vec::new(),
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
        mem::swap(&mut self.agents, &mut level.agents);
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
        self.external_events.push(ExternalEvent::ChangeLevel);
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

    fn player_walk(
        &mut self,
        direction: CardinalDirection,
    ) -> Result<Option<GameControlFlow>, ActionError> {
        let player_coord = self.player_coord();
        let new_player_coord = player_coord + direction.coord();
        if !new_player_coord.is_valid(self.world.size()) {
            // player would walk outside bounds of map
            return Err(ActionError::InvalidMove);
        }
        if let Some(&Layers {
            feature: Some(feature_entity),
            ..
        }) = self.world.spatial_table.layers_at(new_player_coord)
        {
            // If the player bumps into a door, open the door
            if let Some(DoorState::Closed) = self.world.components.door_state.get(feature_entity) {
                self.open_door(feature_entity);
                self.message_log.push(Message::OpenDoor);
                return Ok(None);
            }
            // Don't let the player walk through solid entities
            if self.world.components.solid.contains(feature_entity) {
                if let Some(open_door_entity) =
                    self.open_door_entity_adjacent_to_coord(player_coord, new_player_coord)
                {
                    self.close_door(open_door_entity);
                    self.message_log.push(Message::CloseDoor);
                    return Ok(None);
                }
                return Err(ActionError::InvalidMove);
            }
        }
        self.world
            .spatial_table
            .update_coord(self.player_entity, new_player_coord)
            .unwrap();
        self.change_level_if_player_is_on_stairs();
        Ok(None)
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

    fn npc_walk(
        &mut self,
        entity: Entity,
        direction: CardinalDirection,
    ) -> Option<GameControlFlow> {
        let current_coord = self
            .world
            .entity_coord(entity)
            .expect("Entity tried to walk but it doesn't have a coord");
        let new_coord = current_coord + direction.coord();
        if !new_coord.is_valid(self.world.size()) {
            // would walk outside bounds of map
            return None;
        }
        if let Some(&Layers {
            feature, character, ..
        }) = self.world.spatial_table.layers_at(new_coord)
        {
            if let Some(feature_entity) = feature {
                // If the npc bumps into a door, open the door
                if let Some(DoorState::Closed) =
                    self.world.components.door_state.get(feature_entity)
                {
                    self.open_door(feature_entity);
                    return None;
                }
            }
            // Don't let them walk into other characters
            if let Some(character_entity) = character {
                if self.world.components.player.contains(character_entity) {
                    self.world.damage_player(
                        entity,
                        1,
                        &mut self.rng,
                        &mut self.external_events,
                        &mut self.message_log,
                    );
                }
                return None;
            }
        }
        if !self
            .world
            .can_npc_traverse_feature_at_coord_with_entity(new_coord, entity)
        {
            return None;
        }
        self.world
            .spatial_table
            .update_coord(entity, new_coord)
            .unwrap();
        None
    }

    // Create agents for npcs that lack agents and remove agents for agents whose npcs have been
    // removed.
    fn npc_setup_agents(&mut self) {
        for entity in self.world.components.npc.entities() {
            if !self.agents.contains(entity) {
                self.agents.insert(entity, Agent::new(self.world.size()));
            }
        }
        let mut agents_to_remove = Vec::new();
        for entity in self.agents.entities() {
            if !self.world.components.npc.contains(entity) {
                agents_to_remove.push(entity);
            }
        }
        for entity in agents_to_remove {
            self.agents.remove(entity);
        }
    }

    fn npc_turn(&mut self) -> Option<GameControlFlow> {
        self.npc_setup_agents();
        self.ai_context.update(self.player_entity, &self.world);
        let agent_entities = self.agents.entities().collect::<Vec<_>>();
        for agent_entity in agent_entities {
            let ai_input = self.agents.get_mut(agent_entity).unwrap().act(
                agent_entity,
                &self.world,
                self.player_entity,
                &mut self.ai_context,
                &mut self.rng,
            );
            if let Some(input) = ai_input {
                match input {
                    Input::Wait => (),
                    Input::Walk(direction) => {
                        if let Some(control_flow) = self.npc_walk(agent_entity, direction) {
                            return Some(control_flow);
                        }
                    }
                    _ => (),
                }
            }
        }
        None
    }

    fn cleanup(&mut self) {
        let to_remove = self
            .world
            .components
            .to_remove
            .entities()
            .collect::<Vec<_>>();
        for entity in to_remove {
            self.world.remove_entity(entity);
        }
    }

    #[must_use]
    pub(crate) fn handle_tick(
        &mut self,
        _since_last_tick: Duration,
        _config: &Config,
    ) -> Option<GameControlFlow> {
        let initially_blockd = self.is_gameplay_blocked();
        self.animation_context.tick(
            &mut self.world,
            &mut self.external_events,
            &mut self.message_log,
            &mut self.animation_rng,
        );
        if initially_blockd && !self.is_gameplay_blocked() {
            let result = self.npc_turn();
            if result.is_some() {
                return result;
            }
        }
        self.cleanup();
        self.update_visibility();
        None
    }

    fn pass_time(&mut self) {}

    pub fn is_gameplay_blocked(&self) -> bool {
        !self.world.components.blocks_gameplay.is_empty()
    }

    #[must_use]
    pub(crate) fn handle_input(
        &mut self,
        input: Input,
    ) -> Result<Option<GameControlFlow>, ActionError> {
        let game_control_flow = match input {
            Input::Walk(direction) => {
                let result = self.player_walk(direction);
                match result {
                    Ok(x) => x,
                    Err(action_error) => {
                        self.message_log.push(Message::ActionError(action_error));
                        return Err(action_error);
                    }
                }
            }
            Input::Wait => {
                self.message_log.push(Message::Wait);
                self.pass_time();
                None
            }
            Input::FireEquipped(target) => {
                let start = self.player_coord();
                self.external_events.push(ExternalEvent::FirePistol);
                self.world
                    .spawn_bullet(start, target, &mut self.animation_rng);
                None
            }
            Input::Get => {
                if let Err(e) = self.player_get_item() {
                    self.message_log.push(Message::ActionError(e));
                }
                None
            }
        };
        if game_control_flow.is_some() {
            return Ok(game_control_flow);
        }
        if !self.is_gameplay_blocked() {
            let game_control_flow = self.npc_turn();
            if game_control_flow.is_some() {
                return Ok(game_control_flow);
            }
        }
        self.update_visibility();
        Ok(None)
    }

    fn player_get_item(&mut self) -> Result<(), ActionError> {
        let player_coord = self.player_coord();
        let layers = self.world.spatial_table.layers_at_checked(player_coord);
        if let Some(item_entity) = layers.item {
            if self.world.components.money_item.contains(item_entity) {
                *self
                    .world
                    .components
                    .money
                    .get_mut(self.player_entity)
                    .unwrap() += 1;
                self.world.remove_entity(item_entity);
            }
            Ok(())
        } else {
            Err(ActionError::NothingToGet)
        }
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
                        health: self.world.components.health.get(entity).cloned(),
                    };
                    f(coord, visible_entity, light_colour);
                }
            }
        }
    }

    pub fn world_size(&self) -> Size {
        self.world.spatial_table.grid_size()
    }

    pub fn take_external_events(&mut self) -> Vec<ExternalEvent> {
        use std::mem;
        mem::replace(&mut self.external_events, Vec::new())
    }

    pub fn player_stats(&self) -> PlayerStats {
        PlayerStats {
            health: *self
                .world
                .components
                .health
                .get(self.player_entity)
                .unwrap(),
            oxygen: Meter::new(4, 10),
            food: Meter::new(8, 10),
            poison: Meter::new(3, 10),
            radiation: Meter::new(4, 10),
            power: None,
        }
    }

    pub fn player_money(&self) -> u32 {
        *self.world.components.money.get(self.player_entity).unwrap()
    }
}
