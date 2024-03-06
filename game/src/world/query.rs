use crate::{
    world::{
        data::{Npc, NpcMovement},
        spatial::Layers,
    },
    World,
};
use coord_2d::Coord;
use direction::CardinalDirection;
use entity_table::Entity;

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
        if self.spatial_table.layers_at_checked(start).item.is_none() {
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
}
