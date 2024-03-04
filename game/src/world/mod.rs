use crate::realtime::RealtimeComponents;
use coord_2d::Size;
use entity_table::{Entity, EntityAllocator};
use grid_search_cardinal::distance_map::DistanceMap;
use serde::{Deserialize, Serialize};

pub mod spatial;
use spatial::SpatialTable;

pub mod data;
use data::{Components, EntityData};

pub mod spawn;

mod action;
mod query;

#[derive(Debug, Serialize, Deserialize)]
pub struct World {
    pub entity_allocator: EntityAllocator,
    pub components: Components,
    pub realtime_components: RealtimeComponents,
    pub spatial_table: SpatialTable,
    pub distance_map: DistanceMap,
}

impl World {
    pub fn new(size: Size) -> Self {
        let entity_allocator = EntityAllocator::default();
        let components = Components::default();
        let realtime_components = RealtimeComponents::default();
        let spatial_table = SpatialTable::new(size);
        Self {
            entity_allocator,
            components,
            realtime_components,
            spatial_table,
            distance_map: DistanceMap::new(size),
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) -> EntityData {
        self.entity_allocator.free(entity);
        self.realtime_components.remove_entity(entity);
        self.spatial_table.remove(entity);
        self.components.remove_entity_data(entity)
    }
}
