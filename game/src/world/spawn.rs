use crate::{
    realtime::{flicker, particle},
    world::{
        data::{DoorState, EntityData, Layer, Location, Tile},
        World,
    },
    Entity,
};
use coord_2d::Coord;
use entity_table::entity_data;
use rand::Rng;
use rgb_int::Rgb24;
use visible_area_detection::{vision_distance, Light, Rational};

pub fn make_player() -> EntityData {
    EntityData {
        character: Some(()),
        tile: Some(Tile::Player),
        light: Some(Light {
            colour: Rgb24::new(150, 150, 150),
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
                solid_for_particles: (),
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

    pub fn spawn_debris_burning<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let entity = self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                realtime: (),
                tile: Tile::DebrisBurning,
                solid: (),
                light: Light {
                    colour: Rgb24::new(255, 87, 0),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },
                },
            },
        );
        self.realtime_components.flicker.insert(entity, {
            use flicker::spec::*;
            let colour_range = UniformInclusiveRange {
                low: Rgb24::new(187, 127, 0).to_rgba32(255),
                high: Rgb24::new(255, 187, 0).to_rgba32(255),
            };
            let light_colour_range = UniformInclusiveRange {
                low: Rgb24::new(187, 127, 0),
                high: Rgb24::new(255, 187, 0),
            };
            Flicker {
                colour_hint: Some(colour_range),
                light_colour: Some(light_colour_range),
                tile: None,
                until_next_event: UniformInclusiveRange {
                    low: Duration::from_millis(50),
                    high: Duration::from_millis(200),
                },
            }
            .build(rng)
        });
        self.realtime_components.particle_emitter.insert(entity, {
            use particle::spec::*;
            let colour_range = UniformInclusiveRange {
                low: Rgb24::new(0, 0, 0).to_rgba32(31),
                high: Rgb24::new(255, 255, 255).to_rgba32(31),
            };
            ParticleEmitter {
                emit_particle_every_period: Duration::from_millis(16),
                fade_out_duration: None,
                particle: Particle {
                    colour_hint: Some(colour_range),
                    movement: Some(Movement {
                        angle_range: Radians::uniform_range_all(),
                        /*
                        angle_range: UniformLeftInclusiveRange {
                            low: Radians::from_degrees(-135.0),
                            high: Radians::from_degrees(-45.0),
                        }, */
                        cardinal_period_range: UniformInclusiveRange {
                            low: Duration::from_millis(500),
                            high: Duration::from_millis(1000),
                        },
                    }),
                    fade_duration: Some(Duration::from_millis(5000)),
                    ..Default::default()
                },
            }
            .build(rng)
        });
        entity
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
                    colour: Rgb24::new(0, 255, 0),
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
                solid_for_particles: (),
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
                stairs_up: (),
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

    pub fn spawn_exit(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Exit,
                exit: (),
                light: Light {
                    colour: Rgb24::new(0, 0, 255),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 40,
                    },

                },
            },
        )
    }

    pub fn spawn_zombie(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Zombie,
                npc: (),
                character: (),
            },
        )
    }
}
