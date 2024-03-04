use crate::{
    world::{data::Npc, spatial::Layers},
    World,
};
use coord_2d::Coord;
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
    pub fn can_npc_traverse_feature_at_coord(&self, coord: Coord) -> bool {
        if let Some(layers) = self.spatial_table.layers_at(coord) {
            if let Some(feature) = layers.feature {
                !self.components.solid.contains(feature)
            } else {
                true
            }
        } else {
            false
        }
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
}
