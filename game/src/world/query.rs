use crate::World;
use coord_2d::Coord;

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
}
