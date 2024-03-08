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
use vector::{Radial, Radians};

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
    data::{DoorState, EntityData, EntityUpdate, GunType, Hand, ProjectileDamage},
    spatial::Layers,
    World,
};
pub use world::{
    data::{
        Item, Layer, Location, Meter, NpcType, Organ, OrganTrait, OrganTraits, OrganType, Tile,
    },
    query::PlayerOrgan,
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

pub const MAX_ORGANS: usize = 8;

/// Events which the game can report back to the io layer so it can
/// respond with a sound/visual effect.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ExternalEvent {
    FirePistol,
    FireShotgun,
    FireRocket,
    Explosion(Coord),
    ChangeLevel,
    Melee,
    Death,
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
    GetMoney(u32),
    GetItem(Item),
    DropItem(Item),
    UnequipItem(Item),
    DropUnequipItem(Item),
    EquipItem(Item),
    ReloadGun(Item),
    FireGun(Item),
    FireOrgan(Organ),
    FireOrganDamage(u32),
    YouDie,
    IrradiatedByOrgan(Organ),
    OrganDuplication(Organ),
    OrganDisappear(Organ),
    OrganDamagedByPoison(Organ),
    OrganDestroyedByPoison(Organ),
    GrowTumor,
    OrganGainsTrait {
        organ: Organ,
        trait_: OrganTrait,
    },
    OrganLosesTrait {
        organ: Organ,
        trait_: OrganTrait,
    },
    AmbientRadiation,
    DigestFood {
        health_gain: u32,
    },
    DigestFoodNoHealthIncrease,
    ClawDrop(Item),
    LackOfOxygen,
    Smoke,
    RadiationClose,
    RadiationVeryClose,
    Poison,
    BecomesHostile(NpcType),
    CantAfford(Item),
    Buy(Item),
    FillBloodVial,
    EatFood,
    ApplyAntidote,
    ApplyAntiRads,
    ApplyStimpack,
    ApplyFullBlodVial,
    ApplyBattery,
    DumpOrgan(Organ),
    CantAffordGeneral,
    NoSpaceForOrgan(Organ),
    InstallOrgan(Organ),
    RemoveOrgan(Organ),
    CorruptorTeleport,
    HarvestOrgan(Organ),
    BossKill,
    GetToTheEvacZone,
    Escape,
    HungerDamage,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuImage {}

#[derive(Debug, Clone, Copy)]
pub enum WhichHand {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuChoice {
    Empty,
    DropItem(usize),
    ApplyItem(usize),
    Dummy,
    HarvestOrgan {
        inventory_index: usize,
        organ: Organ,
    },
    EquipWeaponInHand {
        which_hand: WhichHand,
        inventory_index: usize,
    },
    UnequipWhichHand(WhichHand),
    BuyItem {
        item: Item,
        shop_entity: Entity,
        item_entity: Entity,
        shop_inventory_index: usize,
    },
    ClinicBuy {
        clinic_entity: Entity,
    },
    ClinicRemove,
    ClinicInstallFromContainer,
    ClinicRemoveOrgan {
        organ: Organ,
        index: usize,
    },
    ClinicBuyOrgan {
        clinic_entity: Entity,
        index: usize,
        organ: Organ,
    },
    ClinicInstallFromContainerOrgan {
        inventory_index: usize,
        organ: Organ,
    },
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub choices: Vec<MenuChoice>,
    pub text: String,
    pub image: Option<MenuImage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Victory {}

#[derive(Debug, Clone, Copy)]
pub enum GameOverReason {
    YouDied,
}

#[derive(Debug, Clone, Copy)]
pub enum Win {
    Good,
    Bad,
}

#[derive(Debug)]
pub enum GameControlFlow {
    GameOver(GameOverReason),
    Win(Win),
    Menu(Menu),
}

pub struct PlayerStats {
    pub health: Meter,
    pub oxygen: Meter,
    pub food: Meter,
    pub poison: Meter,
    pub radiation: Meter,
    pub power: Option<Meter>,
    pub satiation: Option<Meter>,
}

#[derive(Clone, Copy, Debug)]
pub enum Input {
    Walk(CardinalDirection),
    Wait,
    FireEquipped(Coord),
    FireBody(Coord),
    Get,
    Unequip,
    Reload,
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
    NoCorpseHere,
    NoCyberCore,
    NeedsTwoHands,
    NeedsOneHand,
    NothingToUnequip,
    NothingToReload,
    OutOfLoadedAmmo,
    OutOfAmmo,
    NoGun,
    HealthIsFull,
    OxygenIsFull,
    PoisonIsEmpty,
    RadiationIsEmpty,
    PowerIsFull,
    FoodIsFull,
    RefusingToTargetSelf,
    NoBodyGuns,
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
    turn_count: u64,
    game_over: bool,
    boss_dead: bool,
}

pub const NUM_LEVELS: usize = 4;

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
        world.add_player_initial_items();
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
            turn_count: 0,
            game_over: false,
            boss_dead: false,
        };
        game.systems();
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
        let mut level = self.other_levels[level_index].take().unwrap();
        {
            let mut inventory = self
                .world
                .components
                .inventory
                .get(self.player_entity)
                .unwrap()
                .clone();
            for slot in inventory.items.iter_mut() {
                if let Some(item_entity) = slot.as_mut() {
                    let data = self.world.components.remove_entity_data(*item_entity);
                    let new_item_entity = level.world.entity_allocator.alloc();
                    level
                        .world
                        .components
                        .insert_entity_data(new_item_entity, data);
                    *item_entity = new_item_entity;
                }
            }
            self.world
                .components
                .inventory
                .insert(self.player_entity, inventory);
        }
        {
            let mut hands = self
                .world
                .components
                .hands
                .get(self.player_entity)
                .unwrap()
                .clone();
            if let Hand::Holding(ref mut held_entity) = &mut hands.left {
                let data = self.world.components.remove_entity_data(*held_entity);
                let new_item_entity = level.world.entity_allocator.alloc();
                level
                    .world
                    .components
                    .insert_entity_data(new_item_entity, data);
                *held_entity = new_item_entity;
            }
            if let Hand::Holding(ref mut held_entity) = &mut hands.right {
                let data = self.world.components.remove_entity_data(*held_entity);
                let new_item_entity = level.world.entity_allocator.alloc();
                level
                    .world
                    .components
                    .insert_entity_data(new_item_entity, data);
                *held_entity = new_item_entity;
            }
            self.world
                .components
                .hands
                .insert(self.player_entity, hands);
        }
        let player_data = self.world.remove_entity(self.player_entity);
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
        if let Some(layers) = self.world.spatial_table.layers_at(new_player_coord) {
            if let Some(feature_entity) = layers.feature {
                // If the player bumps into a door, open the door
                if let Some(DoorState::Closed) =
                    self.world.components.door_state.get(feature_entity)
                {
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
            if let Some(character_entity) = layers.character {
                if self.world.components.shop.contains(character_entity) {
                    return Ok(Some(GameControlFlow::Menu(
                        self.shop_menu(character_entity),
                    )));
                }
                self.world.player_bump_combat(
                    character_entity,
                    &mut self.rng,
                    &mut self.external_events,
                    &mut self.message_log,
                );
                return Ok(None);
            }
            self.world
                .spatial_table
                .update_coord(self.player_entity, new_player_coord)
                .unwrap();
            self.change_level_if_player_is_on_stairs();
        }
        Ok(None)
    }

    fn clinic_menu(&self, shop_entity: Entity) -> Menu {
        let shop = self.world.components.shop.get(shop_entity).unwrap();
        Menu {
            image: None,
            text: shop.message.clone(),
            choices: vec![
                MenuChoice::ClinicRemove,
                MenuChoice::ClinicBuy {
                    clinic_entity: shop_entity,
                },
                MenuChoice::ClinicInstallFromContainer,
            ],
        }
    }

    fn shop_menu(&self, shop_entity: Entity) -> Menu {
        if self.world.components.organ_clinic.contains(shop_entity) {
            return self.clinic_menu(shop_entity);
        }
        let shop = self.world.components.shop.get(shop_entity).unwrap();
        let inventory = self
            .world
            .components
            .simple_inventory
            .get(shop_entity)
            .unwrap();
        let choices = inventory
            .into_iter()
            .enumerate()
            .map(|(i, &item_entity)| {
                let item = *self.world.components.item.get(item_entity).unwrap();
                MenuChoice::BuyItem {
                    item,
                    shop_entity,
                    item_entity,
                    shop_inventory_index: i,
                }
            })
            .collect::<Vec<_>>();
        Menu {
            image: None,
            text: shop.message.clone(),
            choices,
        }
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
                    let damage_range = self
                        .world
                        .components
                        .bump_damage
                        .get(entity)
                        .cloned()
                        .unwrap_or_else(|| 1..=1);
                    self.world.damage_player(
                        entity,
                        self.rng.gen_range(damage_range),
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
            if self.world.components.corpse.contains(agent_entity) {
                continue;
            }
            if let Some(slow) = self.world.components.slow.get(agent_entity) {
                if self.turn_count % slow != 0 {
                    continue;
                }
            }
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
        self.systems();
        self.turn_count += 1;
        if let Some(win) = self.win() {
            self.message_log.push(Message::Escape);
            self.update_visibility();
            return Some(GameControlFlow::Win(win));
        }
        self.check_game_over()
    }

    fn systems(&mut self) {
        self.world.handle_poison(&mut self.message_log);
        self.world.handle_radiation(&mut self.message_log);
        self.world.handle_smoke(&mut self.message_log);
        self.world.handle_asphyxiation(&mut self.message_log);
        self.world.handle_resurrection();
        self.world.handle_get_on_touch();
        self.world.handle_spread_poison();
        self.world
            .handle_full_poison(&mut self.rng, &mut self.message_log);
        self.world
            .handle_full_radiation(&mut self.rng, &mut self.message_log);
        self.world
            .handle_player_organ_traits(&mut self.rng, &mut self.message_log);
        self.world
            .handle_player_organs(&mut self.rng, &mut self.message_log);
        if self.world.is_boss_dead() {
            if !self.boss_dead {
                self.message_log.push(Message::BossKill);
                self.message_log.push(Message::GetToTheEvacZone);
                self.remove_corruption();
            }
            self.boss_dead = true;
        }
    }

    fn remove_corruption(&mut self) {
        self.world.remove_corrpution();
        for level in self.other_levels.iter_mut() {
            if let Some(level) = level.as_mut() {
                level.world.remove_corrpution();
            }
        }
    }

    fn check_game_over(&mut self) -> Option<GameControlFlow> {
        if self.game_over {
            return Some(GameControlFlow::GameOver(GameOverReason::YouDied));
        }
        if self.world.is_game_over() {
            self.game_over = true;
            self.world
                .components
                .tile
                .insert(self.player_entity, Tile::DeadPlayer);
            self.player_drop_all_items();
            self.update_visibility();
            self.message_log.push(Message::YouDie);
            self.external_events.push(ExternalEvent::Death);
            Some(GameControlFlow::GameOver(GameOverReason::YouDied))
        } else {
            None
        }
    }

    fn win(&self) -> Option<Win> {
        if self.current_level_index == 0 {
            if let Some(Layers {
                feature: Some(feature),
                ..
            }) = self.world.spatial_table.layers_at(self.player_coord())
            {
                if self.world.components.exit.contains(*feature) {
                    if self.boss_dead {
                        for po in self.world.player_organs() {
                            if po.organ.type_ == OrganType::CorruptedHeart {
                                return Some(Win::Bad);
                            }
                        }
                        return Some(Win::Good);
                    }
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
        self.check_game_over()
    }

    fn pass_time(&mut self) {}

    pub fn is_gameplay_blocked(&self) -> bool {
        !self.world.components.blocks_gameplay.is_empty()
    }

    fn fire_pistol(&mut self, target: Coord) {
        let start = self.player_coord();
        let target = line_2d::LineSegment::new(start, target)
            .infinite_iter()
            .nth(20)
            .unwrap();
        self.external_events.push(ExternalEvent::FirePistol);
        self.world.spawn_bullet(
            start,
            target,
            ProjectileDamage { hit_points: 1..=2 },
            &mut self.animation_rng,
        );
        self.message_log.push(Message::FireGun(Item::Pistol));
    }

    fn fire_shotgun(&mut self, target: Coord) {
        let start = self.player_coord();
        let target = line_2d::LineSegment::new(start, target)
            .infinite_iter()
            .nth(20)
            .unwrap();
        self.external_events.push(ExternalEvent::FireShotgun);
        for _ in 0..8 {
            let angle = Radians::random(&mut self.rng);
            let target = Radial { angle, length: 3.0 }
                .to_cartesian()
                .to_coord_round_nearest()
                + target;
            self.world.spawn_bullet(
                start,
                target,
                ProjectileDamage { hit_points: 2..=3 },
                &mut self.animation_rng,
            );
        }
        self.message_log.push(Message::FireGun(Item::Shotgun));
    }

    fn fire_rocket(&mut self, target: Coord) {
        let start = self.player_coord();
        self.external_events.push(ExternalEvent::FireRocket);
        self.world
            .spawn_rocket(start, target, &mut self.animation_rng);
        self.message_log
            .push(Message::FireGun(Item::RocketLauncher));
    }

    fn fire_equipped(&mut self, target: Coord) -> Result<(), ActionError> {
        let mut has_gun = false;
        let mut has_ammo = false;
        let player_hands = self.world.components.hands.get(self.player_entity).unwrap();
        if let Some(e) = player_hands.left.holding() {
            if let Some(gun) = self.world.components.gun.get_mut(e) {
                has_gun = true;
                if !gun.ammo.is_empty() {
                    has_ammo = true;
                    gun.ammo.decrease(1);
                    match gun.type_ {
                        GunType::Pistol => self.fire_pistol(target),
                        GunType::Shotgun => self.fire_shotgun(target),
                        GunType::RocketLauncher => self.fire_rocket(target),
                    }
                }
            }
        }
        let player_hands = self.world.components.hands.get(self.player_entity).unwrap();
        if let Some(e) = player_hands.right.holding() {
            if let Some(gun) = self.world.components.gun.get_mut(e) {
                has_gun = true;
                if !gun.ammo.is_empty() {
                    has_ammo = true;
                    gun.ammo.decrease(1);
                    match gun.type_ {
                        GunType::Pistol => self.fire_pistol(target),
                        GunType::Shotgun => self.fire_shotgun(target),
                        GunType::RocketLauncher => self.fire_rocket(target),
                    }
                }
            }
        }
        if !has_gun {
            Err(ActionError::NoGun)
        } else if !has_ammo {
            Err(ActionError::OutOfLoadedAmmo)
        } else {
            Ok(())
        }
    }

    fn fire_body_pistol(&mut self, target: Coord) {
        let start = self.player_coord();
        let target = line_2d::LineSegment::new(start, target)
            .infinite_iter()
            .nth(20)
            .unwrap();
        self.external_events.push(ExternalEvent::FirePistol);
        self.world.spawn_bullet(
            start,
            target,
            ProjectileDamage { hit_points: 1..=2 },
            &mut self.animation_rng,
        );
    }

    fn fire_body_shotgun(&mut self, target: Coord) {
        let start = self.player_coord();
        self.external_events.push(ExternalEvent::FireShotgun);
        for _ in 0..8 {
            let angle = Radians::random(&mut self.rng);
            let target = Radial { angle, length: 3.0 }
                .to_cartesian()
                .to_coord_round_nearest()
                + target;
            self.world.spawn_bullet(
                start,
                target,
                ProjectileDamage { hit_points: 2..=3 },
                &mut self.animation_rng,
            );
        }
    }

    fn fire_body(&mut self, target: Coord) -> Result<(), ActionError> {
        let organs = self.world.active_player_organs();
        let mut health_cost = 0;
        let mut count = 0;
        for organ in organs {
            match organ.type_ {
                OrganType::CronenbergPistol => {
                    count += 1;
                    self.message_log.push(Message::FireOrgan(organ));
                    self.fire_body_pistol(target);
                    if organ.cybernetic {
                        self.fire_body_pistol(target);
                    }
                    health_cost += 2;
                    if organ.traits.damaged {
                        health_cost += 2;
                    }
                }
                OrganType::CronenbergShotgun => {
                    count += 1;
                    self.message_log.push(Message::FireOrgan(organ));
                    self.fire_body_shotgun(target);
                    if organ.cybernetic {
                        self.fire_body_shotgun(target);
                    }
                    health_cost += 2;
                    if organ.traits.damaged {
                        health_cost += 2;
                    }
                }
                _ => (),
            }
        }
        if count == 0 {
            return Err(ActionError::NoBodyGuns);
        }
        self.message_log.push(Message::FireOrganDamage(health_cost));
        self.world.damage_character(
            self.player_entity,
            health_cost,
            &mut self.rng,
            &mut self.external_events,
            &mut self.message_log,
        );
        Ok(())
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
                if target == self.player_coord() {
                    self.message_log
                        .push(Message::ActionError(ActionError::RefusingToTargetSelf));
                    return Err(ActionError::RefusingToTargetSelf);
                }
                if let Err(e) = self.fire_equipped(target) {
                    self.message_log.push(Message::ActionError(e));
                    return Err(e);
                }
                None
            }
            Input::FireBody(target) => {
                if target == self.player_coord() {
                    self.message_log
                        .push(Message::ActionError(ActionError::RefusingToTargetSelf));
                    return Err(ActionError::RefusingToTargetSelf);
                }
                if let Err(e) = self.fire_body(target) {
                    self.message_log.push(Message::ActionError(e));
                    return Err(e);
                }
                None
            }
            Input::Get => {
                if let Err(e) = self.player_get_item() {
                    self.message_log.push(Message::ActionError(e));
                    return Err(e);
                }
                None
            }
            Input::Unequip => {
                let player_hands = self
                    .world
                    .components
                    .hands
                    .get_mut(self.player_entity)
                    .unwrap();
                if player_hands.left.is_holding() && player_hands.right.is_holding() {
                    return Ok(Some(GameControlFlow::Menu(Menu {
                        image: None,
                        text: "Unequip from which hand? (escape to cancel)".to_string(),
                        choices: vec![
                            MenuChoice::UnequipWhichHand(WhichHand::Left),
                            MenuChoice::UnequipWhichHand(WhichHand::Right),
                        ],
                    })));
                } else if player_hands.left.is_holding() {
                    self.unequip_from_hand(WhichHand::Left)
                } else if player_hands.right.is_holding() {
                    self.unequip_from_hand(WhichHand::Right)
                } else {
                    let e = ActionError::NothingToUnequip;
                    self.message_log.push(Message::ActionError(e));
                    return Err(e);
                }
                None
            }
            Input::Reload => {
                if let Err(e) = self.player_reload() {
                    self.message_log.push(Message::ActionError(e));
                    return Err(e);
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
        Ok(self.check_game_over())
    }

    fn player_reload_pistol(&mut self) -> Result<(), ActionError> {
        let left = self.get_appropriate_gun_ammo(WhichHand::Left, GunType::Pistol);
        let right = self.get_appropriate_gun_ammo(WhichHand::Left, GunType::Pistol);
        let which_hand = match (left, right) {
            (Some(left), Some(right)) => {
                if left <= right {
                    WhichHand::Left
                } else {
                    WhichHand::Right
                }
            }
            (Some(_), None) => WhichHand::Left,
            (None, Some(_)) => WhichHand::Right,
            (None, None) => return Err(ActionError::NothingToReload),
        };
        if let Some(index) = self.world.player_inventory_item_index(Item::PistolAmmo) {
            self.message_log.push(Message::ReloadGun(Item::Pistol));
            self.reload_gun_in_hand(which_hand);
            if let Some(entity) = self
                .world
                .components
                .inventory
                .get_mut(self.player_entity)
                .unwrap()
                .remove(index)
            {
                self.world.remove_entity(entity);
            }
            Ok(())
        } else {
            Err(ActionError::OutOfAmmo)
        }
    }

    fn player_reload_shotgun(&mut self) -> Result<(), ActionError> {
        if self
            .get_appropriate_gun_ammo(WhichHand::Left, GunType::Shotgun)
            .is_some()
        {
            if let Some(index) = self.world.player_inventory_item_index(Item::ShotgunAmmo) {
                self.message_log.push(Message::ReloadGun(Item::Shotgun));
                self.reload_gun_in_hand(WhichHand::Left);
                if let Some(entity) = self
                    .world
                    .components
                    .inventory
                    .get_mut(self.player_entity)
                    .unwrap()
                    .remove(index)
                {
                    self.world.remove_entity(entity);
                }
                Ok(())
            } else {
                Err(ActionError::OutOfAmmo)
            }
        } else {
            Err(ActionError::NothingToReload)
        }
    }

    fn player_reload_rocket_launcher(&mut self) -> Result<(), ActionError> {
        if self
            .get_appropriate_gun_ammo(WhichHand::Left, GunType::RocketLauncher)
            .is_some()
        {
            if let Some(index) = self.world.player_inventory_item_index(Item::Rocket) {
                self.message_log
                    .push(Message::ReloadGun(Item::RocketLauncher));
                self.reload_gun_in_hand(WhichHand::Left);
                if let Some(entity) = self
                    .world
                    .components
                    .inventory
                    .get_mut(self.player_entity)
                    .unwrap()
                    .remove(index)
                {
                    self.world.remove_entity(entity);
                }
                Ok(())
            } else {
                Err(ActionError::OutOfAmmo)
            }
        } else {
            Err(ActionError::NothingToReload)
        }
    }

    fn player_reload(&mut self) -> Result<(), ActionError> {
        if self
            .get_appropriate_gun_ammo(WhichHand::Left, GunType::Pistol)
            .is_some()
            || self
                .get_appropriate_gun_ammo(WhichHand::Left, GunType::Pistol)
                .is_some()
        {
            return self.player_reload_pistol();
        } else if self
            .get_appropriate_gun_ammo(WhichHand::Left, GunType::Shotgun)
            .is_some()
        {
            return self.player_reload_shotgun();
        } else if self
            .get_appropriate_gun_ammo(WhichHand::Left, GunType::RocketLauncher)
            .is_some()
        {
            return self.player_reload_rocket_launcher();
        }
        Err(ActionError::NothingToReload)
    }

    fn player_get_item(&mut self) -> Result<(), ActionError> {
        let player_coord = self.player_coord();
        let layers = self.world.spatial_table.layers_at_checked(player_coord);
        if let Some(item_entity) = layers.item {
            if let Some(money) = self.world.components.money_item.get(item_entity).cloned() {
                *self
                    .world
                    .components
                    .money
                    .get_mut(self.player_entity)
                    .unwrap() += money;
                self.world.remove_entity(item_entity);
                self.message_log.push(Message::GetMoney(money));
            }
            if let Some(&item) = self.world.components.item.get(item_entity) {
                let inventry = self
                    .world
                    .components
                    .inventory
                    .get_mut(self.player_entity)
                    .unwrap();
                if let Some(slot) = inventry.first_free_slot() {
                    *slot = Some(item_entity);
                    self.world.spatial_table.remove(item_entity);
                    self.message_log.push(Message::GetItem(item));
                } else {
                    return Err(ActionError::InventoryIsFull);
                }
            }
            Ok(())
        } else {
            Err(ActionError::NothingToGet)
        }
    }

    pub(crate) fn handle_choice(&mut self, choice: MenuChoice) -> Option<GameControlFlow> {
        match choice {
            MenuChoice::Empty => (),
            MenuChoice::Dummy => panic!(),
            MenuChoice::DropItem(i) => self.player_drop_item(i),
            MenuChoice::ApplyItem(i) => return self.player_apply_item(i),
            MenuChoice::HarvestOrgan {
                inventory_index,
                organ,
            } => self.player_harvest_organ(inventory_index, organ),
            MenuChoice::EquipWeaponInHand {
                which_hand,
                inventory_index,
            } => self.player_equip_weapon_in_hand(which_hand, inventory_index),
            MenuChoice::UnequipWhichHand(which_hand) => self.unequip_from_hand(which_hand),
            MenuChoice::BuyItem {
                item,
                shop_entity,
                item_entity,
                shop_inventory_index,
                ..
            } => self.player_buy_item(item, item_entity, shop_entity, shop_inventory_index),
            MenuChoice::ClinicBuy { clinic_entity } => {
                return Some(GameControlFlow::Menu(self.clinic_buy_menu(clinic_entity)))
            }
            MenuChoice::ClinicRemove => {
                return Some(GameControlFlow::Menu(self.clinic_remove_menu()))
            }
            MenuChoice::ClinicInstallFromContainer => {
                return Some(GameControlFlow::Menu(
                    self.clinic_install_from_container_menu(),
                ))
            }
            MenuChoice::ClinicBuyOrgan {
                clinic_entity,
                index,
                organ,
            } => self.clinic_buy_organ(clinic_entity, index, organ),
            MenuChoice::ClinicRemoveOrgan { organ, index } => {
                self.clinic_remove_organ(organ, index)
            }
            MenuChoice::ClinicInstallFromContainerOrgan {
                inventory_index,
                organ,
            } => self.clinic_install_from_container(inventory_index, organ),
        }
        self.npc_turn();
        None
    }

    fn clinic_install_from_container(&mut self, inventory_index: usize, organ: Organ) {
        let price = organ.container_install_cost();
        let money = self
            .world
            .components
            .money
            .get_mut(self.player_entity)
            .unwrap();
        if *money < price {
            self.message_log.push(Message::CantAffordGeneral);
            return;
        }
        let organs = self
            .world
            .components
            .organs
            .get_mut(self.player_entity)
            .unwrap();
        if organs.num_free_slots() == 0 {
            self.message_log.push(Message::NoSpaceForOrgan(organ));
            return;
        }
        *money -= price;
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        let container = inventory.get(inventory_index).unwrap();
        let item = self.world.components.item.get_mut(container).unwrap();
        if let Item::OrganContainer(Some(organ)) = item {
            *organs.first_free_slot().unwrap() = Some(*organ);
        }
        *item = Item::OrganContainer(None);
        self.world
            .components
            .tile
            .insert(container, Tile::Item(Item::OrganContainer(None)));
        self.message_log.push(Message::InstallOrgan(organ));
    }

    fn clinic_remove_organ(&mut self, organ: Organ, index: usize) {
        let price = organ.remove_price();
        let money = self
            .world
            .components
            .money
            .get_mut(self.player_entity)
            .unwrap();
        if price < 0 {
            *money += (-price) as u32;
        } else {
            let price = price as u32;
            if *money < price {
                self.message_log.push(Message::CantAffordGeneral);
                return;
            }
            *money -= price;
        }
        self.message_log.push(Message::RemoveOrgan(organ));
        let organs = self
            .world
            .components
            .organs
            .get_mut(self.player_entity)
            .unwrap();
        *organs.get_slot_mut(index) = None;
    }

    fn clinic_buy_organ(&mut self, clinic_entity: Entity, index: usize, organ: Organ) {
        let price = organ.player_buy_price();
        let money = self
            .world
            .components
            .money
            .get_mut(self.player_entity)
            .unwrap();
        if *money < price {
            self.message_log.push(Message::CantAffordGeneral);
            return;
        }
        let organs = self
            .world
            .components
            .organs
            .get_mut(self.player_entity)
            .unwrap();
        if organs.num_free_slots() == 0 {
            self.message_log.push(Message::NoSpaceForOrgan(organ));
            return;
        }
        *money -= price;
        *organs.first_free_slot().unwrap() = Some(organ);
        self.message_log.push(Message::InstallOrgan(organ));
        let clinic_organs = self
            .world
            .components
            .simple_organs
            .get_mut(clinic_entity)
            .unwrap();
        clinic_organs.remove(index);
    }

    fn clinic_install_from_container_menu(&self) -> Menu {
        let mut choices = Vec::new();
        let inventory = self
            .world
            .components
            .inventory
            .get(self.player_entity)
            .unwrap();
        for (i, slot) in inventory.items().into_iter().enumerate() {
            if let Some(item_entity) = slot {
                if let Some(Item::OrganContainer(Some(organ))) =
                    self.world.components.item.get(*item_entity)
                {
                    choices.push(MenuChoice::ClinicInstallFromContainerOrgan {
                        inventory_index: i,
                        organ: *organ,
                    })
                }
            }
        }
        if choices.is_empty() {
            choices.push(MenuChoice::Empty);
        }
        Menu {
            text: "Choose an organ to install from your organ containers: (escape to cancel)"
                .to_string(),
            choices,
            image: None,
        }
    }

    fn clinic_remove_menu(&self) -> Menu {
        let mut choices = Vec::new();
        let organs = self
            .world
            .components
            .organs
            .get(self.player_entity)
            .unwrap();
        for (i, organ) in organs.organs().into_iter().enumerate() {
            if let Some(organ) = organ {
                choices.push(MenuChoice::ClinicRemoveOrgan {
                    organ: *organ,
                    index: i,
                });
            }
        }
        Menu {
            text: "Choose an organ to remove. I'll pay for any original organs in good condition (other than appendices). (escape to cancel)".to_string(),
            choices,
            image: None,
        }
    }

    fn clinic_buy_menu(&self, clinic_entity: Entity) -> Menu {
        let organs = self
            .world
            .components
            .simple_organs
            .get(clinic_entity)
            .unwrap();
        let choices = organs
            .into_iter()
            .enumerate()
            .map(|(i, organ)| MenuChoice::ClinicBuyOrgan {
                clinic_entity,
                index: i,
                organ: *organ,
            })
            .collect::<Vec<_>>();
        Menu {
            text: "Choose an organ to buy: (escape to cancel)".to_string(),
            choices,
            image: None,
        }
    }

    fn player_buy_item(
        &mut self,
        item: Item,
        item_entity: Entity,
        shop_entity: Entity,
        shop_inventory_index: usize,
    ) {
        let money = self
            .world
            .components
            .money
            .get_mut(self.player_entity)
            .unwrap();
        if item.price() > *money {
            self.message_log.push(Message::CantAfford(item));
            return;
        }
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(first_free_slot) = inventory.first_free_slot() {
            *money -= item.price();
            *first_free_slot = Some(item_entity);
            let shop_inventory = self
                .world
                .components
                .simple_inventory
                .get_mut(shop_entity)
                .unwrap();
            shop_inventory.remove(shop_inventory_index);
            self.message_log.push(Message::Buy(item));
        } else {
            self.message_log
                .push(Message::ActionError(ActionError::InventoryIsFull));
        }
    }

    fn player_equip_weapon_in_hand(&mut self, which_hand: WhichHand, inventory_index: usize) {
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(entity) = inventory.remove(inventory_index) {
            let player_hands = self.world.components.hands.get(self.player_entity).unwrap();
            if let Hand::Holding(e) = player_hands.left {
                // assume that any 2 handed weapon is just in the left hand
                if let Some(gun) = self.world.components.gun.get(e) {
                    if gun.hands_required >= 2 {
                        self.unequip_from_hand(WhichHand::Left);
                    }
                }
            }
            let player_hands = self
                .world
                .components
                .hands
                .get_mut(self.player_entity)
                .unwrap();
            let hand = match which_hand {
                WhichHand::Left => &mut player_hands.left,
                WhichHand::Right => &mut player_hands.right,
            };
            if let Some(&item) = self.world.components.item.get(entity) {
                self.message_log.push(Message::EquipItem(item));
            }
            match hand {
                Hand::Claw => (),
                Hand::Empty => *hand = Hand::Holding(entity),
                Hand::Holding(current_item) => {
                    let current_item = *current_item;
                    *hand = Hand::Holding(entity);
                    let inventory = self
                        .world
                        .components
                        .inventory
                        .get_mut(self.player_entity)
                        .unwrap();
                    if let Some(slot) = inventory.first_free_slot() {
                        if let Some(&item) = self.world.components.item.get(current_item) {
                            self.message_log.push(Message::UnequipItem(item));
                        }
                        *slot = Some(current_item);
                    }
                }
            }
        }
    }

    fn player_harvest_organ(&mut self, inventory_index: usize, organ: Organ) {
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(entity) = inventory.get(inventory_index) {
            self.world
                .components
                .item
                .insert(entity, Item::OrganContainer(Some(organ)));
            self.message_log.push(Message::HarvestOrgan(organ));
        }
        let player_coord = self.player_coord();
        if let Some(Layers {
            item: Some(entity), ..
        }) = self.world.spatial_table.layers_at(player_coord)
        {
            self.world.remove_entity(*entity);
            self.world.make_floor_bloody(player_coord);
        }
    }

    fn player_apply_item(&mut self, i: usize) -> Option<GameControlFlow> {
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(item_entity) = inventory.get(i) {
            if let Some(&item) = self.world.components.item.get(item_entity) {
                match item {
                    Item::OrganContainer(None) => {
                        if let Some(organs) = self.organs_of_corpse_at_player() {
                            return Some(GameControlFlow::Menu(Menu {
                                text: format!(
                                    "Choose an organ to harvest. Corpse will be destroyed. (escape to cancel):"
                                ),
                                image: None,
                                choices: organs
                                    .into_iter()
                                    .map(|organ| MenuChoice::HarvestOrgan {
                                        inventory_index: i,
                                        organ,
                                    })
                                    .collect(),
                            }));
                        } else {
                            self.message_log
                                .push(Message::ActionError(ActionError::NoCorpseHere));
                        }
                    }
                    Item::BloodVialEmpty => {
                        self.player_fill_blood_vial(item_entity);
                        self.message_log.push(Message::FillBloodVial);
                        let player_coord = self.player_coord();
                        if let Some(Layers {
                            item: Some(entity), ..
                        }) = self.world.spatial_table.layers_at(player_coord)
                        {
                            self.world.remove_entity(*entity);
                            self.world.make_floor_bloody(player_coord);
                        }
                    }
                    Item::Antidote => {
                        let poison = self
                            .world
                            .components
                            .poison
                            .get_mut(self.player_entity)
                            .unwrap();
                        if poison.is_empty() {
                            self.message_log
                                .push(Message::ActionError(ActionError::PoisonIsEmpty));
                        } else {
                            poison.decrease(10);
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::ApplyAntidote);
                        }
                    }
                    Item::AntiRads => {
                        let radiation = self
                            .world
                            .components
                            .radiation
                            .get_mut(self.player_entity)
                            .unwrap();
                        if radiation.is_empty() {
                            self.message_log
                                .push(Message::ActionError(ActionError::RadiationIsEmpty));
                        } else {
                            radiation.decrease(50);
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::ApplyAntiRads);
                        }
                    }
                    Item::Stimpack => {
                        let health = self
                            .world
                            .components
                            .health
                            .get_mut(self.player_entity)
                            .unwrap();
                        if health.is_full() {
                            self.message_log
                                .push(Message::ActionError(ActionError::HealthIsFull));
                        } else {
                            health.increase(10);
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::ApplyStimpack);
                        }
                    }
                    Item::Food => {
                        let food = self
                            .world
                            .components
                            .food
                            .get_mut(self.player_entity)
                            .unwrap();
                        if food.is_full() {
                            self.message_log
                                .push(Message::ActionError(ActionError::FoodIsFull));
                        } else {
                            food.increase(25);
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::EatFood);
                        }
                    }
                    Item::BloodVialFull => {
                        let vampiric = self.world.player_has_vampiric_organ();
                        let oxygen = self
                            .world
                            .components
                            .oxygen
                            .get_mut(self.player_entity)
                            .unwrap();
                        if oxygen.is_full() && !vampiric {
                            self.message_log
                                .push(Message::ActionError(ActionError::OxygenIsFull));
                        } else {
                            oxygen.increase(10);
                            self.world
                                .components
                                .item
                                .insert(item_entity, Item::BloodVialEmpty);
                            self.world
                                .components
                                .tile
                                .insert(item_entity, Tile::Item(Item::BloodVialEmpty));
                            self.world
                                .components
                                .satiation
                                .get_mut(self.player_entity)
                                .unwrap()
                                .fill();
                            self.message_log.push(Message::ApplyFullBlodVial);
                        }
                    }
                    Item::Battery => {
                        if self.world.player_has_cyber_core() {
                            let power = self
                                .world
                                .components
                                .power
                                .get_mut(self.player_entity)
                                .unwrap();
                            if power.is_full() {
                                self.message_log
                                    .push(Message::ActionError(ActionError::PowerIsFull));
                            } else {
                                power.increase(50);
                                self.message_log.push(Message::ApplyBattery);
                            }
                        } else {
                            self.message_log
                                .push(Message::ActionError(ActionError::NoCyberCore));
                        }
                    }
                    Item::OrganContainer(Some(organ)) => {
                        self.world.make_floor_bloody(self.player_coord());
                        self.world
                            .components
                            .item
                            .insert(item_entity, Item::OrganContainer(None));
                        self.world
                            .components
                            .tile
                            .insert(item_entity, Tile::Item(Item::OrganContainer(None)));
                        self.message_log.push(Message::DumpOrgan(organ));
                    }
                    Item::Pistol => {
                        if self.world.num_player_claws() >= 2 {
                            self.message_log
                                .push(Message::ActionError(ActionError::NeedsOneHand));
                        } else {
                            if self.world.num_player_claws() == 1 {
                                let player_hands = self
                                    .world
                                    .components
                                    .hands
                                    .get_mut(self.player_entity)
                                    .unwrap();
                                if player_hands.left == Hand::Claw {
                                    self.player_equip_weapon_in_hand(WhichHand::Right, i);
                                } else {
                                    self.player_equip_weapon_in_hand(WhichHand::Left, i);
                                }
                            } else {
                                return Some(GameControlFlow::Menu(Menu {
                                    text: format!("Which hand? (escape to cancel)"),
                                    image: None,
                                    choices: vec![
                                        MenuChoice::EquipWeaponInHand {
                                            which_hand: WhichHand::Left,
                                            inventory_index: i,
                                        },
                                        MenuChoice::EquipWeaponInHand {
                                            which_hand: WhichHand::Right,
                                            inventory_index: i,
                                        },
                                    ],
                                }));
                            }
                        }
                    }
                    Item::Shotgun | Item::RocketLauncher => {
                        if self.world.num_player_claws() >= 1 {
                            self.message_log
                                .push(Message::ActionError(ActionError::NeedsTwoHands));
                        } else {
                            self.equip_two_handed_weapon(i);
                        }
                    }
                    Item::PistolAmmo => {
                        let left = self.get_appropriate_gun_ammo(WhichHand::Left, GunType::Pistol);
                        let right =
                            self.get_appropriate_gun_ammo(WhichHand::Right, GunType::Pistol);
                        let success = match (left, right) {
                            (Some(left), Some(right)) => {
                                if left < right {
                                    self.reload_gun_in_hand(WhichHand::Left)
                                } else {
                                    self.reload_gun_in_hand(WhichHand::Right)
                                }
                                true
                            }
                            (Some(_), None) => {
                                self.reload_gun_in_hand(WhichHand::Left);
                                true
                            }
                            (None, Some(_)) => {
                                self.reload_gun_in_hand(WhichHand::Right);
                                true
                            }
                            (None, None) => false,
                        };
                        if success {
                            let inventory = self
                                .world
                                .components
                                .inventory
                                .get_mut(self.player_entity)
                                .unwrap();
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::ReloadGun(Item::Pistol));
                        } else {
                            self.message_log
                                .push(Message::ActionError(ActionError::NothingToReload));
                        }
                    }
                    Item::ShotgunAmmo => {
                        // 2 handed weapons are always in the lreft hand
                        if self
                            .get_appropriate_gun_ammo(WhichHand::Left, GunType::Shotgun)
                            .is_some()
                        {
                            self.reload_gun_in_hand(WhichHand::Left);
                            let inventory = self
                                .world
                                .components
                                .inventory
                                .get_mut(self.player_entity)
                                .unwrap();
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log.push(Message::ReloadGun(Item::Shotgun));
                        } else {
                            self.message_log
                                .push(Message::ActionError(ActionError::NothingToReload));
                        }
                    }
                    Item::Rocket => {
                        // 2 handed weapons are always in the lreft hand
                        if self
                            .get_appropriate_gun_ammo(WhichHand::Left, GunType::RocketLauncher)
                            .is_some()
                        {
                            self.reload_gun_in_hand(WhichHand::Left);
                            let inventory = self
                                .world
                                .components
                                .inventory
                                .get_mut(self.player_entity)
                                .unwrap();
                            inventory.remove(i);
                            self.world.remove_entity(item_entity);
                            self.message_log
                                .push(Message::ReloadGun(Item::RocketLauncher));
                        } else {
                            self.message_log
                                .push(Message::ActionError(ActionError::NothingToReload));
                        }
                    }
                }
            }
        }
        None
    }

    fn reload_gun_in_hand(&mut self, which_hand: WhichHand) {
        if let Some(entity) = self.player_hand_entity(which_hand) {
            if let Some(gun) = self.world.components.gun.get_mut(entity) {
                gun.ammo.fill();
            }
        }
    }

    fn get_appropriate_gun_ammo(&self, which_hand: WhichHand, gun_type: GunType) -> Option<u32> {
        if let Some(entity) = self.player_hand_entity(which_hand) {
            if let Some(gun) = self.world.components.gun.get(entity) {
                if gun.type_ == gun_type {
                    if !gun.ammo.is_full() {
                        return Some(gun.ammo.current());
                    }
                }
            }
        }
        None
    }

    fn player_hand_entity(&self, which_hand: WhichHand) -> Option<Entity> {
        let hands = self.world.components.hands.get(self.player_entity).unwrap();
        match which_hand {
            WhichHand::Left => hands.left.holding(),
            WhichHand::Right => hands.right.holding(),
        }
    }

    fn equip_two_handed_weapon(&mut self, inventory_index: usize) {
        let hands = self.world.components.hands.get(self.player_entity).unwrap();
        if hands.left.is_claw() || hands.right.is_claw() {
            return;
        }
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(entity) = inventory.remove(inventory_index) {
            self.unequip_from_hand(WhichHand::Left);
            self.unequip_from_hand(WhichHand::Right);
            let hands = self
                .world
                .components
                .hands
                .get_mut(self.player_entity)
                .unwrap();
            hands.left = Hand::Holding(entity);
            if let Some(&item) = self.world.components.item.get(entity) {
                self.message_log.push(Message::EquipItem(item));
            }
        }
    }

    fn unequip_from_hand(&mut self, which_hand: WhichHand) {
        let hands = self
            .world
            .components
            .hands
            .get_mut(self.player_entity)
            .unwrap();
        let hand = match which_hand {
            WhichHand::Left => &mut hands.left,
            WhichHand::Right => &mut hands.right,
        };
        if let Hand::Holding(entity) = hand {
            let entity = *entity;
            let item = *self.world.components.item.get(entity).unwrap();
            *hand = Hand::Empty;
            let inventory = self
                .world
                .components
                .inventory
                .get_mut(self.player_entity)
                .unwrap();
            if let Some(slot) = inventory.first_free_slot() {
                self.message_log.push(Message::UnequipItem(item));
                *slot = Some(entity);
            } else {
                // no room in inventory
                if let Some(coord) = self.world.nearest_itemless_coord(self.player_coord()) {
                    self.message_log.push(Message::DropUnequipItem(item));
                    let _ = self.world.spatial_table.update(
                        entity,
                        Location {
                            coord,
                            layer: Some(Layer::Item),
                        },
                    );
                }
            }
        }
    }

    fn organs_of_corpse_at_player(&self) -> Option<Vec<Organ>> {
        let player_coord = self.player_coord();
        if let Some(Layers {
            item: Some(entity), ..
        }) = self.world.spatial_table.layers_at(player_coord)
        {
            if self.world.components.corpse.contains(*entity) {
                self.world.components.simple_organs.get(*entity).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    fn player_fill_blood_vial(&mut self, item_entity: Entity) {
        if let Some(Layers {
            item: Some(corpse_entity),
            ..
        }) = self.world.spatial_table.layers_at(self.player_coord())
        {
            if self.world.components.corpse.contains(*corpse_entity) {
                self.world
                    .components
                    .item
                    .insert(item_entity, Item::BloodVialFull);
                self.world
                    .components
                    .tile
                    .insert(item_entity, Tile::Item(Item::BloodVialFull));
                return;
            }
        }
        self.message_log
            .push(Message::ActionError(ActionError::NoCorpseHere));
    }

    fn player_drop_all_items(&mut self) {
        let inventory = self
            .world
            .components
            .inventory
            .get(self.player_entity)
            .unwrap();
        for i in 0..inventory.size() {
            let inventory = self
                .world
                .components
                .inventory
                .get_mut(self.player_entity)
                .unwrap();
            if let Some(item_entity) = inventory.remove(i) {
                if let Some(coord) = self.world.nearest_itemless_coord(self.player_coord()) {
                    let _ = self.world.spatial_table.update(
                        item_entity,
                        Location {
                            coord,
                            layer: Some(Layer::Item),
                        },
                    );
                }
            }
        }
    }
    fn player_drop_item(&mut self, i: usize) {
        let inventory = self
            .world
            .components
            .inventory
            .get_mut(self.player_entity)
            .unwrap();
        if let Some(item_entity) = inventory.remove(i) {
            if let Some(&item) = self.world.components.item.get(item_entity) {
                self.message_log.push(Message::DropItem(item));
            }
            if let Some(coord) = self.world.nearest_itemless_coord(self.player_coord()) {
                let _ = self.world.spatial_table.update(
                    item_entity,
                    Location {
                        coord,
                        layer: Some(Layer::Item),
                    },
                );
            }
        }
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
            oxygen: *self
                .world
                .components
                .oxygen
                .get(self.player_entity)
                .unwrap(),
            food: *self.world.components.food.get(self.player_entity).unwrap(),
            poison: *self
                .world
                .components
                .poison
                .get(self.player_entity)
                .unwrap(),
            radiation: *self
                .world
                .components
                .radiation
                .get(self.player_entity)
                .unwrap(),
            power: if self.world.player_has_cyber_core() {
                self.world.components.power.get(self.player_entity).cloned()
            } else {
                None
            },
            satiation: if self.world.player_has_vampiric_organ() {
                self.world
                    .components
                    .satiation
                    .get(self.player_entity)
                    .cloned()
            } else {
                None
            },
        }
    }

    pub fn player_money(&self) -> u32 {
        *self.world.components.money.get(self.player_entity).unwrap()
    }

    pub fn inventory_size(&self) -> usize {
        self.world
            .components
            .inventory
            .get(self.player_entity)
            .unwrap()
            .size()
    }

    pub fn inventory_item(&self, i: usize) -> Option<Item> {
        let inventory = self
            .world
            .components
            .inventory
            .get(self.player_entity)
            .unwrap();
        inventory
            .get(i)
            .map(|entity| *self.world.components.item.get(entity).unwrap())
    }

    fn hand_string(&self, hand: Hand) -> (String, bool) {
        match hand {
            Hand::Empty => ("(empty)".to_string(), false),
            Hand::Claw => ("Claw".to_string(), false),
            Hand::Holding(entity) => {
                if let Some(gun) = self.world.components.gun.get(entity) {
                    let name = match gun.type_ {
                        GunType::Pistol => "Pistol",
                        GunType::Shotgun => "Shotgun",
                        GunType::RocketLauncher => "Rkt Launcher",
                    };
                    let both = gun.hands_required > 1;
                    (
                        format!("{name} ({}/{})", gun.ammo.current(), gun.ammo.max()),
                        both,
                    )
                } else {
                    ("?".to_string(), false)
                }
            }
        }
    }

    pub fn player_hand_contents(&self) -> (String, String) {
        let hands = self.world.components.hands.get(self.player_entity).unwrap();
        let (left, both) = self.hand_string(hands.left);
        let (right, _) = self.hand_string(hands.right);
        if both {
            (left, "^^^".to_string())
        } else {
            (left, right)
        }
    }

    pub fn player_organs(&self) -> Vec<PlayerOrgan> {
        self.world.player_organs()
    }

    pub fn current_level_index(&self) -> usize {
        self.current_level_index
    }
}
