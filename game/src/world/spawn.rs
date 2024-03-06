use crate::{
    realtime::{self, flicker, movement, particle},
    world::{
        data::{
            CollidesWith, Disposition, DoorState, EntityData, Inventory, Item, Layer, Location,
            Meter, Npc, NpcMovement, NpcType, OnCollision, ProjectileDamage, Tile,
        },
        explosion, World,
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
        player: Some(()),
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
        health: Some(Meter::new_full(10)),
        oxygen: Some(Meter::new_full(10)),
        food: Some(Meter::new_full(10)),
        poison: Some(Meter::new(0, 10)),
        radiation: Some(Meter::new(0, 10)),
        inventory: Some(Inventory::new(12)),
        money: Some(0),
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
                destructible: (),
            },
        )
    }

    pub fn spawn_debris(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Debris,
                solid: (),
                difficult: (),
                destructible: (),
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
                difficult: (),
                destructible: (),
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
                destructible: (),
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

    pub fn spawn_bullet<R: Rng>(&mut self, start: Coord, target: Coord, rng: &mut R) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord: start,
                    layer: None,
                },
            )
            .unwrap();
        self.components.realtime.insert(entity, ());
        self.components.blocks_gameplay.insert(entity, ());
        self.components
            .on_collision
            .insert(entity, OnCollision::Remove);
        self.realtime_components.movement.insert(
            entity,
            {
                use movement::spec::*;
                Movement {
                    path: target - start,
                    cardinal_step_duration: Duration::from_millis(24),
                    repeat: Repeat::Once,
                }
            }
            .build(),
        );
        let particle_emitter_ = {
            use particle::spec::*;
            let colour_range = UniformInclusiveRange {
                low: Rgb24::new(0, 0, 0).to_rgba32(31),
                high: Rgb24::new(255, 255, 255).to_rgba32(31),
            };
            ParticleEmitter {
                emit_particle_every_period: Duration::from_millis(32),
                fade_out_duration: None,
                particle: Particle {
                    tile: None,
                    colour_hint: Some(colour_range),
                    movement: Some(Movement {
                        angle_range: Radians::uniform_range_all(),
                        cardinal_period_range: UniformInclusiveRange {
                            low: Duration::from_millis(200),
                            high: Duration::from_millis(500),
                        },
                    }),
                    fade_duration: Some(Duration::from_millis(1000)),
                    possible_light: None,
                    ..Default::default()
                },
            }
        }
        .build(rng);
        self.realtime_components
            .particle_emitter
            .insert(entity, particle_emitter_);
        self.components.collides_with.insert(
            entity,
            CollidesWith {
                solid: true,
                character: true,
            },
        );
        self.components.tile.insert(entity, Tile::Bullet);
        self.components.particle.insert(entity, ());
        self.components
            .projectile_damage
            .insert(entity, ProjectileDamage { hit_points: 1..=2 });
        entity
    }

    pub fn spawn_rocket<R: Rng>(&mut self, start: Coord, target: Coord, rng: &mut R) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord: start,
                    layer: None,
                },
            )
            .unwrap();
        self.components.realtime.insert(entity, ());
        self.components.blocks_gameplay.insert(entity, ());
        self.components
            .on_collision
            .insert(entity, OnCollision::Remove);
        self.realtime_components.movement.insert(
            entity,
            {
                use movement::spec::*;
                Movement {
                    path: target - start,
                    cardinal_step_duration: Duration::from_millis(24),
                    repeat: Repeat::Once,
                }
            }
            .build(),
        );
        let particle_emitter_ = {
            use particle::spec::*;
            let colour_range = UniformInclusiveRange {
                low: Rgb24::new(0, 0, 0).to_rgba32(31),
                high: Rgb24::new(255, 255, 255).to_rgba32(31),
            };
            ParticleEmitter {
                emit_particle_every_period: Duration::from_millis(32),
                fade_out_duration: None,
                particle: Particle {
                    tile: None,
                    colour_hint: Some(colour_range),
                    movement: Some(Movement {
                        angle_range: Radians::uniform_range_all(),
                        cardinal_period_range: UniformInclusiveRange {
                            low: Duration::from_millis(200),
                            high: Duration::from_millis(500),
                        },
                    }),
                    fade_duration: Some(Duration::from_millis(1000)),
                    possible_light: None,
                    ..Default::default()
                },
            }
        }
        .build(rng);
        self.realtime_components
            .particle_emitter
            .insert(entity, particle_emitter_);
        self.components.collides_with.insert(
            entity,
            CollidesWith {
                solid: true,
                character: true,
            },
        );
        self.components.tile.insert(entity, Tile::Bullet);
        self.components.particle.insert(entity, ());
        self.components
            .projectile_damage
            .insert(entity, ProjectileDamage { hit_points: 3..=4 });
        self.components.on_collision.insert(
            entity,
            OnCollision::Explode({
                use explosion::spec::*;
                Explosion {
                    mechanics: Mechanics { range: 2 },
                    particle_emitter: ParticleEmitter {
                        duration: Duration::from_millis(100),
                        num_particles_per_frame: 25,
                        min_step: Duration::from_millis(10),
                        max_step: Duration::from_millis(30),
                        fade_duration: Duration::from_millis(150),
                    },
                }
            }),
        );
        self.components.light.insert(
            entity,
            Light {
                colour: Rgb24::new(255, 187, 63),
                vision_distance: vision_distance::Circle::new_squared(90),
                diminish: Rational {
                    numerator: 1,
                    denominator: 10,
                },
            },
        );

        entity
    }

    pub fn spawn_explosion_emitter<R: Rng>(
        &mut self,
        coord: Coord,
        spec: &explosion::spec::ParticleEmitter,
        rng: &mut R,
    ) -> Entity {
        let emitter_entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(emitter_entity, Location { coord, layer: None })
            .unwrap();
        self.realtime_components.fade.insert(
            emitter_entity,
            realtime::fade::FadeState::new(spec.duration),
        );
        self.components.realtime.insert(emitter_entity, ());
        self.realtime_components
            .particle_emitter
            .insert(emitter_entity, {
                use realtime::particle::spec::*;
                ParticleEmitter {
                    emit_particle_every_period: realtime::period_per_frame(
                        spec.num_particles_per_frame,
                    ),
                    fade_out_duration: Some(spec.duration),
                    particle: Particle {
                        tile: None,
                        movement: Some(Movement {
                            angle_range: Radians::uniform_range_all(),
                            cardinal_period_range: UniformInclusiveRange {
                                low: spec.min_step,
                                high: spec.max_step,
                            },
                        }),
                        fade_duration: Some(spec.fade_duration),
                        colour_hint: Some(UniformInclusiveRange {
                            low: Rgb24::new(255, 17, 0).to_rgba32(255),
                            high: Rgb24::new(255, 255, 63).to_rgba32(255),
                        }),
                        possible_particle_emitter: Some(Possible {
                            chance: Rational {
                                numerator: 1,
                                denominator: 20,
                            },
                            value: Box::new(ParticleEmitter {
                                emit_particle_every_period: spec.min_step,
                                fade_out_duration: None,
                                particle: Particle {
                                    tile: None,
                                    movement: Some(Movement {
                                        angle_range: Radians::uniform_range_all(),
                                        cardinal_period_range: UniformInclusiveRange {
                                            low: Duration::from_millis(200),
                                            high: Duration::from_millis(500),
                                        },
                                    }),
                                    fade_duration: Some(Duration::from_millis(1000)),
                                    ..Default::default()
                                },
                            }),
                        }),
                        ..Default::default()
                    },
                }
                .build(rng)
            });
        self.components.light.insert(
            emitter_entity,
            Light {
                colour: Rgb24::new(255, 255, 63),
                vision_distance: vision_distance::Circle::new_squared(900),
                diminish: Rational {
                    numerator: 1,
                    denominator: 1000,
                },
            },
        );
        self.realtime_components.light_colour_fade.insert(
            emitter_entity,
            realtime::light_colour_fade::LightColourFadeState {
                fade_state: realtime::fade::FadeState::new(spec.fade_duration * 8),
                from: Rgb24::new(255, 255, 63),
                to: Rgb24::new(127, 127, 0),
            },
        );
        emitter_entity
    }

    pub fn spawn_money(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Item),
            entity_data! {
                tile: Tile::Money,
                money_item: (),
                destructible: (),
            },
        )
    }

    pub fn spawn_item(&mut self, coord: Coord, item: Item) -> Entity {
        self.spawn_entity(
            (coord, Layer::Item),
            entity_data! {
                tile: Tile::Item(item),
                item,
                destructible: (),
            },
        )
    }

    pub fn spawn_zombie(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Zombie,
                npc: Npc {
                    disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Zombie,
                health: Meter::new_full(4),
                resurrects_in: Meter::new_full(10),
            },
        )
    }

    pub fn spawn_climber(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Climber,
                npc: Npc {
                    disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: true,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Climber,
                health: Meter::new_full(3),
            },
        )
    }

    pub fn spawn_trespasser(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Trespasser,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: true,
                    },
                },
                character: (),
                npc_type: NpcType::Trespasser,
                health: Meter::new_full(3),
            },
        )
    }

    pub fn spawn_boomer(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Boomer,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Boomer,
                health: Meter::new_full(2),
                explodes_on_death: (),
            },
        )
    }
}
