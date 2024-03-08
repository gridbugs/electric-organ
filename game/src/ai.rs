use crate::{
    world::data::{Disposition, Npc, NpcMovement},
    Input, World,
};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use entity_table::Entity;
use grid_2d::Grid;
use grid_search_cardinal::{
    best::{BestSearch, Context as BestSearchContext, Depth, Step},
    distance_map::{
        DistanceMap, PopulateContext as DistanceMapPopulateContext,
        SearchContext as DistanceMapSearchContext,
    },
    point_to_point::{expand, Context as PointToPointSearchContext, NoPath},
    CanEnter, Path,
};
use line_2d::LineSegment;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use shadowcast::{vision_distance, Context as ShadowcastContext, InputGrid, VisionDistance};
use std::collections::HashMap;

const FLEE_DISTANCE: u32 = 10;

struct Visibility;

impl InputGrid for Visibility {
    type Grid = World;
    type Opacity = u8;
    fn size(&self, world: &Self::Grid) -> Size {
        world.size()
    }
    fn get_opacity(&self, grid: &Self::Grid, coord: Coord) -> Self::Opacity {
        grid.get_opacity(coord)
    }
}

struct WorldCanEnterIgnoreCharacters<'a> {
    world: &'a World,
    npc_movement: NpcMovement,
}

impl<'a> CanEnter for WorldCanEnterIgnoreCharacters<'a> {
    fn can_enter(&self, coord: Coord) -> bool {
        self.world
            .can_npc_traverse_feature_at_coord_with_movement(coord, self.npc_movement)
    }
}

struct WorldCanEnterAvoidNpcs<'a> {
    world: &'a World,
    npc_movement: NpcMovement,
}

impl<'a> CanEnter for WorldCanEnterAvoidNpcs<'a> {
    fn can_enter(&self, coord: Coord) -> bool {
        let can_traverse = self
            .world
            .can_npc_traverse_feature_at_coord_with_movement(coord, self.npc_movement);
        can_traverse && !self.world.is_npc_at_coord(coord)
    }
}

fn has_line_of_sight(
    eye: Coord,
    dest: Coord,
    vision_distance: vision_distance::Circle,
    world: &World,
) -> bool {
    let mut opacity_sum = 0;
    for coord in LineSegment::new(eye, dest).iter() {
        let eye_to_coord = coord - eye;
        if !vision_distance.in_range(eye_to_coord) {
            return false;
        }
        opacity_sum += world.get_opacity(coord) as u32;
        if opacity_sum >= 255 {
            return false;
        }
    }
    true
}

#[derive(Serialize, Deserialize)]
pub struct AiContext {
    best_search_context: BestSearchContext,
    point_to_point_search_context: PointToPointSearchContext,
    distance_map_populate_context: DistanceMapPopulateContext,
    distance_map_search_context: DistanceMapSearchContext,
    player_approach: HashMap<NpcMovement, DistanceMap>,
    player_flee: HashMap<NpcMovement, DistanceMap>,
    item_distance: DistanceMap,
    wander_path: Path,
    shadowcast: ShadowcastContext<u8>,
}

impl AiContext {
    pub fn new(size: Size) -> Self {
        Self {
            best_search_context: BestSearchContext::new(size),
            point_to_point_search_context: PointToPointSearchContext::new(size),
            distance_map_populate_context: DistanceMapPopulateContext::default(),
            distance_map_search_context: DistanceMapSearchContext::new(size),
            player_approach: NpcMovement::ALL
                .iter()
                .map(|&npc_movement| (npc_movement, DistanceMap::new(size)))
                .collect(),
            player_flee: NpcMovement::ALL
                .iter()
                .map(|&npc_movement| (npc_movement, DistanceMap::new(size)))
                .collect(),
            item_distance: DistanceMap::new(size),
            wander_path: Path::default(),
            shadowcast: ShadowcastContext::default(),
        }
    }
    pub fn update(&mut self, player: Entity, world: &World) {
        if let Some(player_coord) = world.entity_coord(player) {
            for (&npc_movement, player_approach) in self.player_approach.iter_mut() {
                self.distance_map_populate_context.add(player_coord);
                self.distance_map_populate_context.populate_approach(
                    &WorldCanEnterIgnoreCharacters {
                        world,
                        npc_movement,
                    },
                    20,
                    player_approach,
                );
            }
            for (&npc_movement, player_flee) in self.player_flee.iter_mut() {
                self.distance_map_populate_context.add(player_coord);
                self.distance_map_populate_context.populate_flee(
                    &WorldCanEnterIgnoreCharacters {
                        world,
                        npc_movement,
                    },
                    20,
                    player_flee,
                );
            }
        } else {
            self.player_approach.clear();
            self.player_flee.clear();
        }
        for item_entity in world.components.item.entities() {
            if let Some(coord) = world.spatial_table.coord_of(item_entity) {
                self.distance_map_populate_context.add(coord);
            }
        }
        for item_entity in world.components.money_item.entities() {
            if let Some(coord) = world.spatial_table.coord_of(item_entity) {
                self.distance_map_populate_context.add(coord);
            }
        }
        self.distance_map_populate_context.populate_approach(
            &WorldCanEnterIgnoreCharacters {
                world,
                npc_movement: NpcMovement {
                    // XXX note that this is based on the "snatcher" npc's movement ability
                    can_traverse_difficult: false,
                    can_open_doors: false,
                },
            },
            20,
            &mut self.item_distance,
        );
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LastSeenCell {
    count: u64,
    avoid_until: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LastSeenGrid {
    count: u64,
    last_seen: Grid<LastSeenCell>,
}

#[derive(Clone, Copy, Debug)]
struct CanSeePlayer;

impl LastSeenGrid {
    fn new(size: Size) -> Self {
        Self {
            count: 1,
            last_seen: Grid::new_fn(size, |_| LastSeenCell {
                count: 0,
                avoid_until: 0,
            }),
        }
    }

    fn update(
        &mut self,
        npc: &Npc,
        eye: Coord,
        vision_distance: vision_distance::Circle,
        world: &World,
        can_see_player: Option<CanSeePlayer>,
        ai_context: &mut AiContext,
    ) {
        self.count += 1;
        let distance_map_to_player = ai_context.player_approach.get(&npc.movement).unwrap();
        ai_context.shadowcast.for_each_visible(
            eye,
            &Visibility,
            world,
            vision_distance,
            255,
            |cell_coord, _visible_directions, _visibility| {
                if let Some(cell) = self.last_seen.get_mut(cell_coord) {
                    cell.count = self.count;
                    if let Some(CanSeePlayer) = can_see_player {
                        if let Some(distance_to_player) =
                            distance_map_to_player.distance(cell_coord)
                        {
                            if distance_to_player < FLEE_DISTANCE {
                                cell.avoid_until = self.count + 20;
                            }
                        }
                    }
                }
            },
        );
    }
}

struct Wander<'a, R> {
    world: &'a World,
    last_seen_grid: &'a LastSeenGrid,
    min_last_seen_coord: Option<Coord>,
    min_last_seen_count: u64,
    entity: Entity,
    rng: &'a mut R,
    avoid: bool,
}

impl<'a, R: Rng> BestSearch for Wander<'a, R> {
    fn is_at_max_depth(&self, _depth: Depth) -> bool {
        false
    }
    fn can_enter_initial_updating_best(&mut self, coord: Coord) -> bool {
        if self
            .world
            .can_npc_traverse_feature_at_coord_with_entity(coord, self.entity)
        {
            if let Some(entity) = self.world.character_at_coord(coord) {
                if entity != self.entity {
                    let my_coord = self.world.entity_coord(self.entity).unwrap();
                    if my_coord.manhattan_distance(coord) < 4 {
                        let can_see_character = has_line_of_sight(
                            my_coord,
                            coord,
                            vision_distance::Circle::new_squared(40),
                            &self.world,
                        );
                        if can_see_character && self.rng.gen_range(0u8..4) > 0 {
                            return false;
                        }
                    }
                }
            }
            let last_seen_cell = self.last_seen_grid.last_seen.get_checked(coord);
            if self.avoid && last_seen_cell.avoid_until > self.min_last_seen_count {
                return false;
            }
            let last_seen_count = last_seen_cell.count;
            if last_seen_count < self.min_last_seen_count {
                self.min_last_seen_count = last_seen_count;
                self.min_last_seen_coord = Some(coord);
            }
            true
        } else {
            false
        }
    }
    fn can_step_updating_best(&mut self, step: Step) -> bool {
        self.can_enter_initial_updating_best(step.to_coord)
    }
    fn best_coord(&self) -> Option<Coord> {
        self.min_last_seen_coord
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    last_seen_grid: LastSeenGrid,
    vision_distance: vision_distance::Circle,
    behaviour: Behaviour,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Behaviour {
    Wander {
        avoid: bool,
    },
    Chase {
        last_seen_player_coord: Coord,
        accurate: bool,
    },
    Flee,
    Steal,
    Nothing,
}

impl Agent {
    pub fn new(size: Size) -> Self {
        Self {
            last_seen_grid: LastSeenGrid::new(size),
            vision_distance: vision_distance::Circle::new_squared(40),
            behaviour: Behaviour::Wander { avoid: true },
        }
    }
    pub fn act<R: Rng>(
        &mut self,
        entity: Entity,
        world: &World,
        player: Entity,
        ai_context: &mut AiContext,
        rng: &mut R,
    ) -> Option<Input> {
        let coord = world.entity_coord(entity)?;
        let npc = world.entity_npc(entity).expect("not an npc");
        self.behaviour = if let Some(player_coord) = world.entity_coord(player) {
            let can_see_player =
                if has_line_of_sight(coord, player_coord, self.vision_distance, world) {
                    Some(CanSeePlayer)
                } else {
                    None
                };
            self.last_seen_grid.update(
                npc,
                coord,
                self.vision_distance,
                world,
                can_see_player,
                ai_context,
            );
            if let Some(CanSeePlayer) = can_see_player {
                match npc.disposition {
                    Disposition::Neutral => Behaviour::Nothing,
                    Disposition::Thief => Behaviour::Steal,
                    Disposition::Hostile => Behaviour::Chase {
                        last_seen_player_coord: player_coord,
                        accurate: true,
                    },
                    Disposition::Afraid => {
                        let player_approach =
                            ai_context.player_approach.get(&npc.movement).unwrap();
                        if let Some(distance) = player_approach.distance(coord) {
                            if distance < FLEE_DISTANCE {
                                Behaviour::Flee
                            } else {
                                Behaviour::Wander { avoid: true }
                            }
                        } else {
                            Behaviour::Wander { avoid: true }
                        }
                    }
                }
            } else {
                match npc.disposition {
                    Disposition::Neutral => Behaviour::Nothing,
                    _ => match self.behaviour {
                        Behaviour::Nothing => Behaviour::Nothing,
                        Behaviour::Steal => Behaviour::Steal,
                        Behaviour::Chase {
                            last_seen_player_coord,
                            ..
                        } => {
                            if last_seen_player_coord == coord {
                                // walk up to where the player was last seen, then go back to wandering
                                Behaviour::Wander { avoid: true }
                            } else {
                                Behaviour::Chase {
                                    last_seen_player_coord,
                                    accurate: last_seen_player_coord == player_coord,
                                }
                            }
                        }
                        Behaviour::Wander { avoid } => Behaviour::Wander { avoid },
                        Behaviour::Flee => {
                            // stop fleeing the player if you can't see them
                            Behaviour::Wander { avoid: true }
                        }
                    },
                }
            }
        } else {
            match npc.disposition {
                Disposition::Neutral => Behaviour::Nothing,
                _ => {
                    self.last_seen_grid.update(
                        npc,
                        coord,
                        self.vision_distance,
                        world,
                        None,
                        ai_context,
                    );
                    Behaviour::Wander { avoid: false }
                }
            }
        };
        match self.behaviour {
            Behaviour::Nothing => None,
            Behaviour::Steal => {
                let maybe_cardinal_direction = ai_context.distance_map_search_context.search_first(
                    &WorldCanEnterAvoidNpcs {
                        world,
                        npc_movement: npc.movement,
                    },
                    coord,
                    5,
                    &ai_context.item_distance,
                );
                match maybe_cardinal_direction {
                    None => None,
                    Some(cardinal_direction) => Some(Input::Walk(cardinal_direction)),
                }
            }
            Behaviour::Wander { avoid } => {
                let mut path_node = ai_context.wander_path.pop();
                let need_new_path = if let Some(path_node) = path_node {
                    let implied_current_coord = path_node.to_coord - path_node.in_direction.coord();
                    implied_current_coord != coord
                } else {
                    true
                };
                if need_new_path {
                    ai_context.best_search_context.best_search_path(
                        Wander {
                            world,
                            last_seen_grid: &self.last_seen_grid,
                            min_last_seen_coord: None,
                            min_last_seen_count: self
                                .last_seen_grid
                                .last_seen
                                .get_checked(coord)
                                .count,
                            entity,
                            avoid,
                            rng,
                        },
                        coord,
                        &mut ai_context.wander_path,
                    );
                    path_node = ai_context.wander_path.pop();
                }
                if let Some(path_node) = path_node {
                    Some(Input::Walk(path_node.in_direction))
                } else {
                    None
                }
            }
            Behaviour::Flee => {
                let player_flee = ai_context.player_flee.get(&npc.movement).unwrap();
                let maybe_cardinal_direction = ai_context.distance_map_search_context.search_first(
                    &WorldCanEnterAvoidNpcs {
                        world,
                        npc_movement: npc.movement,
                    },
                    coord,
                    5,
                    player_flee,
                );
                match maybe_cardinal_direction {
                    None => {
                        self.behaviour = Behaviour::Wander { avoid: true };
                        None
                    }
                    Some(cardinal_direction) => Some(Input::Walk(cardinal_direction)),
                }
            }
            Behaviour::Chase {
                last_seen_player_coord,
                accurate,
            } => {
                if accurate {
                    let player_approach = ai_context.player_approach.get(&npc.movement).unwrap();
                    let maybe_cardinal_direction =
                        ai_context.distance_map_search_context.search_first(
                            &WorldCanEnterAvoidNpcs {
                                world,
                                npc_movement: npc.movement,
                            },
                            coord,
                            5,
                            player_approach,
                        );
                    match maybe_cardinal_direction {
                        None => {
                            self.behaviour = Behaviour::Wander { avoid: true };
                            None
                        }
                        Some(cardinal_direction) => {
                            let dest = coord + cardinal_direction.coord();
                            let cardinal_direction = if player_approach.distance(dest) == Some(1) {
                                // The agent is about to be 1 space away from the player. This can
                                // cause a problem where the agent consistently moves to block the
                                // player's movement which is annoying. The consistency comes from
                                // the fact that the distance map search favours certain directions
                                // over others. Break this monotony by randomly choosing between
                                // equally good positions.
                                let mut options = Vec::new();
                                for direction in CardinalDirection::all() {
                                    let dest = coord + direction.coord();
                                    if player_approach.distance(dest) == Some(1) {
                                        options.push(direction);
                                    }
                                }
                                *options.choose(rng).unwrap()
                            } else {
                                cardinal_direction
                            };
                            Some(Input::Walk(cardinal_direction))
                        }
                    }
                } else {
                    let result = ai_context
                        .point_to_point_search_context
                        .point_to_point_search_first(
                            expand::JumpPoint,
                            &WorldCanEnterAvoidNpcs {
                                world,
                                npc_movement: npc.movement,
                            },
                            coord,
                            last_seen_player_coord,
                        );
                    match result {
                        Err(NoPath) | Ok(None) => {
                            self.behaviour = Behaviour::Wander { avoid: true };
                            None
                        }
                        Ok(Some(cardinal_direction)) => Some(Input::Walk(cardinal_direction)),
                    }
                }
            }
        }
    }
}
