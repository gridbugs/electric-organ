use crate::{
    world::{
        data::{DoorState, EntityData, Layer, Location, Tile},
        World,
    },
    Entity,
};
use coord_2d::Coord;
use entity_table::entity_data;
use rgb_int::Rgb24;
use visible_area_detection::{vision_distance, Light, Rational};

pub fn make_player() -> EntityData {
    EntityData {
        tile: Some(Tile::Player),
        light: Some(Light {
            colour: Rgb24::new(127, 127, 127),
            vision_distance: vision_distance::Circle::new_squared(200),
            diminish: Rational {
                numerator: 1,
                denominator: 100,
            },
        }),
        ..Default::default()
    }
}

impl World {
    pub fn insert_entity_data(&mut self, location: Location, entity_data: EntityData) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.spatial_table.update(entity, location).unwrap();
        self.components.insert_entity_data(entity, entity_data);
        entity
    }

    fn spawn_entity<L: Into<Location>>(&mut self, location: L, entity_data: EntityData) -> Entity {
        let entity = self.entity_allocator.alloc();
        let location @ Location { layer, coord } = location.into();
        if let Err(e) = self.spatial_table.update(entity, location) {
            panic!("{:?}: There is already a {:?} at {:?}", e, layer, coord);
        }
        self.components.insert_entity_data(entity, entity_data);
        entity
    }

    pub fn spawn_wall(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Wall,
                solid: (),
                opacity: 255,
            },
        )
    }

    pub fn spawn_debris(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Debris,
                solid: (),
            },
        )
    }

    pub fn spawn_debris_burning(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::DebrisBurning,
                solid: (),
                light: Light {
                    colour: Rgb24::new(255, 127, 0),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },
                },
            },
        )
    }

    pub fn spawn_tentacle(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Tentacle,
                solid: (),
            },
        )
    }

    pub fn spawn_tentacle_glow(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::TentacleGlow,
                solid: (),
                light: Light {
                    colour: Rgb24::new(127, 255, 0),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },
                },
            },
        )
    }

    pub fn spawn_floor(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Floor),
            entity_data! {
                tile: Tile::Floor,
            },
        )
    }

    pub fn spawn_street(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Floor),
            entity_data! {
                tile: Tile::Street,
            },
        )
    }

    pub fn spawn_alley(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Floor),
            entity_data! {
                tile: Tile::Alley,
            },
        )
    }

    pub fn spawn_footpath(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Floor),
            entity_data! {
                tile: Tile::Footpath,
            },
        )
    }

    pub fn spawn_door(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::DoorClosed,
                solid: (),
                door_state: DoorState::Closed,
                opacity: 255,
            },
        )
    }

    pub fn spawn_stairs_down(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::StairsDown,
                stairs_down: (),
                light: Light {
                    colour: Rgb24::new(0, 255, 255),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },
                },
            },
        )
    }

    pub fn spawn_stairs_up(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::StairsUp,
                light: Light {
                    colour: Rgb24::new(0, 255, 255),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },

                },
            },
        )
    }
}
