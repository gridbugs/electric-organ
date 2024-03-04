use crate::World;
use coord_2d::Coord;
use entity_table::Entity;
use grid_2d::Grid;
use grid_search_cardinal::best::{BestSearch, Depth, Step};
use rand::Rng;
use serde::{Deserialize, Serialize};
use shadowcast::vision_distance;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LastSeenCell {
    count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LastSeenGrid {
    count: u64,
    last_seen: Grid<LastSeenCell>,
}

struct Wander<'a, R> {
    world: &'a World,
    last_seen_grid: &'a LastSeenGrid,
    min_last_seen_coord: Option<Coord>,
    min_last_seen_count: u64,
    entity: Entity,
    rng: &'a mut R,
}
/*
impl<'a, R: Rng> BestSearch for Wander<'a, R> {
    fn is_at_max_depth(&self, _depth: Depth) -> bool {
        false
    }
    fn can_enter_initial_updating_best(&mut self, coord: Coord) -> bool {
        if self.world.can_npc_traverse_feature_at_coord(coord) {
            if let Some(entity) = self.world.get_character_at_coord(coord) {
                if entity != self.entity {
                    let my_coord = self.world.entity_coord(self.entity).unwrap();
                    if my_coord.manhattan_distance(coord) < 4 {
                        let can_see_character = has_line_of_sight(
                            my_coord,
                            coord,
                            self.world,
                            vision_distance::Circle::new_squared(40),
                        );
                        if can_see_character && self.rng.gen_range(0u8..4) > 0 {
                            return false;
                        }
                    }
                }
            }
            let last_seen_cell = self.last_seen_grid.last_seen.get_checked(coord);
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
}*/
