use crate::{
    world::{data::*, spatial::Layers},
    World,
};
use coord_2d::Coord;
use direction::CardinalDirection;
use entity_table::Entity;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

impl World {
    pub fn stairs_up_or_exit_coord(&self) -> Option<Coord> {
        self.components
            .stairs_up
            .entities()
            .next()
            .and_then(|entity| self.spatial_table.coord_of(entity))
            .or_else(|| {
                self.components
                    .exit
                    .entities()
                    .next()
                    .and_then(|entity| self.spatial_table.coord_of(entity))
            })
    }
    pub fn stairs_down_coord(&self) -> Option<Coord> {
        self.components
            .stairs_down
            .entities()
            .next()
            .and_then(|entity| self.spatial_table.coord_of(entity))
    }
    pub fn entity_coord(&self, entity: Entity) -> Option<Coord> {
        self.spatial_table.coord_of(entity)
    }

    pub fn can_npc_traverse_feature_at_coord_with_movement(
        &self,
        coord: Coord,
        movement: NpcMovement,
    ) -> bool {
        if let Some(layers) = self.spatial_table.layers_at(coord) {
            if let Some(feature) = layers.feature {
                !self.components.solid.contains(feature)
                    || movement.can_open_doors && self.components.door_state.contains(feature)
                    || movement.can_traverse_difficult
                        && self.components.difficult.contains(feature)
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn can_npc_traverse_feature_at_coord_with_entity(
        &self,
        coord: Coord,
        npc_entity: Entity,
    ) -> bool {
        let npc = self.components.npc.get(npc_entity).expect("not an npc");
        self.can_npc_traverse_feature_at_coord_with_movement(coord, npc.movement)
    }

    pub fn is_npc_at_coord(&self, coord: Coord) -> bool {
        if let Some(layers) = self.spatial_table.layers_at(coord) {
            if let Some(character) = layers.character {
                self.components.npc.contains(character)
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn get_opacity(&self, coord: Coord) -> u8 {
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
    pub fn character_at_coord(&self, coord: Coord) -> Option<Entity> {
        if let Some(layers) = self.spatial_table.layers_at(coord) {
            layers.character
        } else {
            None
        }
    }
    pub fn entity_npc(&self, entity: Entity) -> Option<&Npc> {
        self.components.npc.get(entity)
    }

    pub fn nearest_itemless_coord(&self, start: Coord) -> Option<Coord> {
        use std::collections::{HashSet, VecDeque};
        if let Some(layers) = self.spatial_table.layers_at(start) {
            if layers.feature.is_none() {
                if layers.item.is_none() {
                    return Some(start);
                }
            }
        }
        let mut seen = HashSet::new();
        seen.insert(start);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(coord) = queue.pop_front() {
            for d in CardinalDirection::all() {
                let coord = coord + d.coord();
                if seen.insert(coord) {
                    if let Some(layers) = self.spatial_table.layers_at(coord) {
                        if layers.feature.is_none() {
                            if layers.item.is_none() {
                                return Some(coord);
                            } else {
                                queue.push_back(coord);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn nearest_characterless_coord(&self, start: Coord) -> Option<Coord> {
        use std::collections::{HashSet, VecDeque};
        if self
            .spatial_table
            .layers_at_checked(start)
            .character
            .is_none()
        {
            return Some(start);
        }
        let mut seen = HashSet::new();
        seen.insert(start);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(coord) = queue.pop_front() {
            for d in CardinalDirection::all() {
                let coord = coord + d.coord();
                if seen.insert(coord) {
                    if let Some(layers) = self.spatial_table.layers_at(coord) {
                        if layers.feature.is_none() {
                            if layers.character.is_none() {
                                return Some(coord);
                            } else {
                                queue.push_back(coord);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn nearest_non_poison_coord(&self, start: Coord) -> Option<Coord> {
        use std::collections::{HashSet, VecDeque};
        let layers = self.spatial_table.layers_at_checked(start);
        if layers.feature.is_none() {
            if let Some(e) = layers.floor {
                if !self.components.floor_poison.contains(e) {
                    return Some(start);
                }
            }
        }
        let mut seen = HashSet::new();
        seen.insert(start);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        let mut rng = StdRng::from_entropy();
        while let Some(coord) = queue.pop_front() {
            let mut dirs = CardinalDirection::all().collect::<Vec<_>>();
            dirs.shuffle(&mut rng);
            for d in dirs {
                let coord = coord + d.coord();
                if seen.insert(coord) {
                    if let Some(layers) = self.spatial_table.layers_at(coord) {
                        if layers.feature.is_none() {
                            if let Some(e) = layers.floor {
                                if !self.components.floor_poison.contains(e) {
                                    return Some(coord);
                                }
                            }
                            queue.push_back(coord);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn num_player_claws(&self) -> usize {
        let player = self.components.player.entities().next().unwrap();
        self.components.organs.get(player).unwrap().num_claws()
    }

    pub fn player_inventory_item_index(&self, item: Item) -> Option<usize> {
        let player = self.components.player.entities().next().unwrap();
        let inventory = self.components.inventory.get(player).unwrap();
        for (i, entity) in inventory.items().into_iter().enumerate() {
            if let Some(entity) = entity {
                if let Some(current_item) = self.components.item.get(*entity) {
                    if *current_item == item {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    pub fn is_game_over(&self) -> bool {
        if let Some(player_entity) = self.components.player.entities().next() {
            if self.components.to_remove.contains(player_entity) {
                return true;
            }
            if let Some(health) = self.components.health.get(player_entity) {
                if health.current() == 0 {
                    return true;
                }
            }
            false
        } else {
            true
        }
    }

    pub fn player_has_vampiric_organ(&self) -> bool {
        let player_entity = self.components.player.entities().next().unwrap();
        let organs = self.components.organs.get(player_entity).unwrap();
        for organ in organs.organs() {
            if let Some(organ) = organ {
                if organ.traits.vampiric {
                    return true;
                }
            }
        }
        false
    }

    pub fn player_has_cyber_core(&self) -> bool {
        let player_entity = self.components.player.entities().next().unwrap();
        let organs = self.components.organs.get(player_entity).unwrap();
        for organ in organs.organs() {
            if let Some(organ) = organ {
                if organ.type_ == OrganType::CyberCore {
                    return true;
                }
            }
        }
        false
    }

    pub fn player_organs(&self) -> Vec<PlayerOrgan> {
        let mut ret = Vec::new();
        let player_entity = self.components.player.entities().next().unwrap();
        let organs = self.components.organs.get(player_entity).unwrap();
        let satiation = self.components.satiation.get(player_entity).unwrap();
        let power = if self.player_has_cyber_core() {
            self.components.power.get(player_entity).unwrap().current()
        } else {
            0
        };
        for organ in organs.organs() {
            if let Some(organ) = organ {
                let mut active = true;
                if organ.traits.vampiric && satiation.current() == 0 {
                    active = false;
                }
                if organ.cybernetic && power == 0 {
                    active = false;
                }
                ret.push(PlayerOrgan {
                    organ: *organ,
                    active,
                });
            }
        }
        ret
    }

    pub fn active_player_organs(&self) -> Vec<Organ> {
        let mut ret = Vec::new();
        for po in self.player_organs() {
            if po.active {
                ret.push(po.organ);
            }
        }
        ret
    }

    pub fn line_distance_stopping_at_solid(&self, from: Coord, to: Coord) -> Option<usize> {
        let mut count = 0;
        for coord in line_2d::coords_between(from, to) {
            if let Some(Layers {
                feature: Some(feature),
                ..
            }) = self.spatial_table.layers_at(coord)
            {
                if self.components.solid.contains(*feature)
                    && !self.components.difficult.contains(*feature)
                {
                    return None;
                }
            }
            count += 1;
        }
        Some(count)
    }
}

pub struct PlayerOrgan {
    pub organ: Organ,
    pub active: bool,
}
