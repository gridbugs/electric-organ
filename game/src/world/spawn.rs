use crate::{
    realtime::{self, flicker, movement, particle},
    world::{data::*, explosion, World},
    Entity,
};
use coord_2d::Coord;
use direction::Direction;
use entity_table::entity_data;
use rand::{seq::SliceRandom, Rng};
use rgb_int::Rgb24;
use visible_area_detection::{vision_distance, Light, Rational};

fn player_starting_organs() -> Organs {
    let mut ret = Organs::new(crate::MAX_ORGANS);
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Heart,
        traits: OrganTraits {
            transient: false,
            ..OrganTraits::none()
        },
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Lung,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Lung,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Stomach,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Liver,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::Appendix,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    });
    /*
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::CronenbergPistol,
        traits: OrganTraits {
            vampiric: true,
            ..OrganTraits::none()
        },
        original: true,
        cybernetic: false,
    });
    *ret.first_free_slot().unwrap() = Some(Organ {
        type_: OrganType::CronenbergShotgun,
        traits: OrganTraits::none(),
        original: true,
        cybernetic: false,
    }); */
    ret
}

fn random_organ_traits<R: Rng>(rng: &mut R) -> OrganTraits {
    let mut traits = OrganTraits::none();
    if rng.gen::<f64>() < 0.66 {
        *traits.get_mut(OrganTrait::choose(rng)) = true;
    }
    traits
}

fn random_basic_organ<R: Rng>(rng: &mut R) -> Organ {
    let types = vec![
        OrganType::Heart,
        OrganType::Lung,
        OrganType::Stomach,
        OrganType::Liver,
    ];
    let type_ = *types.choose(rng).unwrap();
    let traits = random_organ_traits(rng);
    Organ {
        type_,
        traits,
        cybernetic: false,
        original: false,
    }
}

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
        health: Some(Meter::new(20, 20)),
        oxygen: Some(Meter::new(20, 20)),
        food: Some(Meter::new(50, 50)),
        poison: Some(Meter::new(0, 10)),
        radiation: Some(Meter::new(0, 80)),
        inventory: Some(Inventory::new(16)),
        satiation: Some(Meter::new(0, 20)),
        power: Some(Meter::new(0, 0)),
        money: Some(0),
        organs: Some(player_starting_organs()),
        hands: Some(Hands {
            left: Hand::Empty,
            right: Hand::Empty,
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
                smoke: (),
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
                difficult: (),
                tentacle: (),
            },
        )
    }

    pub fn spawn_tentacle_glow(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::TentacleGlow,
                solid: (),
                radioactive: (),
                difficult: (),
                tentacle: (),
                light: Light {
                    colour: Rgb24::new(0, 255, 255),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 200,
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

    pub fn spawn_bullet<R: Rng>(
        &mut self,
        start: Coord,
        target: Coord,
        projectile_damage: ProjectileDamage,
        rng: &mut R,
    ) -> Entity {
        let target = if target == start {
            start + rng.gen::<Direction>().coord()
        } else {
            target
        };
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
            .insert(entity, projectile_damage);
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
            .insert(entity, ProjectileDamage { hit_points: 5..=10 });
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

    pub fn spawn_money<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let amount = rng.gen_range(10..=20);
        self.spawn_entity(
            (coord, Layer::Item),
            entity_data! {
                tile: Tile::Money(amount),
                money_item: amount,
                destructible: (),
            },
        )
    }

    pub fn spawn_item(&mut self, coord: Coord, item: Item) -> Entity {
        let mut data = entity_data! {
            tile: Tile::Item(item),
            item,
            destructible: (),
        };
        make_gun(&mut data);
        self.spawn_entity((coord, Layer::Item), data)
    }

    pub fn spawn_item_no_coord(&mut self, item: Item) -> Entity {
        let mut data = entity_data! {
            tile: Tile::Item(item),
            item,
            destructible: (),
        };
        make_gun(&mut data);
        let entity = self.entity_allocator.alloc();
        self.components.insert_entity_data(entity, data);
        entity
    }

    pub fn spawn_zombie<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
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
                bump_damage: 1..=3,
                simple_organs: vec![
                    Organ {
                        type_: OrganType::Heart,
                        traits: random_organ_traits(rng),
                        original: false,
                        cybernetic: false,
                    },
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_climber<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
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
                bump_damage: 1..=3,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_trespasser<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
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
                bump_damage: 1..=3,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_boomer<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
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
                bump_damage: 1..=3,
                explodes_on_death: (),
                slow: 2,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_snatcher<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Snatcher,
                npc: Npc { disposition: Disposition::Thief,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Snatcher,
                health: Meter::new_full(10),
                simple_inventory: Vec::new(),
                bump_damage: 1..=2,
                get_on_touch: (),
                slow: 3,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_poisoner<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Poisoner,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Poisoner,
                health: Meter::new_full(3),
                bump_damage: 1..=2,
                spread_poison: (),
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_divider<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Divider,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Divider,
                health: Meter::new_full(16),
                bump_damage: 1..=2,
                split_on_damage: (),
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }

    pub fn spawn_glower<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Glower,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Glower,
                health: Meter::new_full(8),
                bump_damage: 2..=4,
                radioactive: (),
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
                light: Light {
                    colour: Rgb24::hex(0x009973),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 100,
                    },
                },
                slow: 2,
            },
        )
    }

    pub fn spawn_venter<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let entity = self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Venter,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::Venter,
                health: Meter::new_full(8),
                bump_damage: 2..=4,
                smoke: (),
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
                realtime: (),
            },
        );
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

    pub fn spawn_corruptor<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let entity = self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Corruptor,
                npc: Npc { disposition: Disposition::Hostile,
                    movement: NpcMovement {
                        can_traverse_difficult: true,
                        can_open_doors: true,
                    },
                },
                character: (),
                npc_type: NpcType::Corruptor,
                health: Meter::new_full(30),
                bump_damage: 5..=10,
                radioactive: (),
                smoke: (),
                spread_poison: (),
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                    Organ {
                        type_: OrganType::CorruptedHeart,
                        cybernetic: false,
                        original: false,
                        traits: OrganTraits {
                            ..OrganTraits::none()
                        }
                    }
                ],
              light: Light {
                    colour: Rgb24::hex(0xf00ff),
                    vision_distance: vision_distance::Circle::new_squared(200),
                    diminish: Rational {
                        numerator: 1,
                        denominator: 100,
                    },
                },
                realtime: (),
                boss: (),
            },
        );
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

    pub fn spawn_gun_store<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let item_pool = vec![
            Item::Pistol,
            Item::PistolAmmo,
            Item::Shotgun,
            Item::ShotgunAmmo,
            Item::RocketLauncher,
            Item::Rocket,
        ];
        let mut simple_inventory = Vec::new();
        for _ in 0..8 {
            let item = *item_pool.choose(rng).unwrap();
            let entity = self.spawn_item_no_coord(item);
            simple_inventory.push(entity);
        }
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::GunStore,
                npc: Npc { disposition: Disposition::Neutral,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::GunStore,
                health: Meter::new_full(50),
                bump_damage: 10..=20,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
                simple_inventory,
                shop: Shop {
                    message: "Welcome to my Gun Shop!".to_string(),
                }
            },
        )
    }

    pub fn spawn_item_store<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        let item_pool = vec![
            Item::Stimpack,
            Item::Antidote,
            Item::BloodVialEmpty,
            Item::Battery,
            Item::Food,
            Item::AntiRads,
            Item::OrganContainer(None),
        ];
        let mut simple_inventory = Vec::new();
        for _ in 0..8 {
            let item = *item_pool.choose(rng).unwrap();
            let entity = self.spawn_item_no_coord(item);
            simple_inventory.push(entity);
        }
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::ItemStore,
                npc: Npc { disposition: Disposition::Neutral,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::ItemStore,
                health: Meter::new_full(50),
                bump_damage: 10..=20,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
                simple_inventory,
                shop: Shop {
                    message: "Welcome to my Item Shop!".to_string(),
                }
            },
        )
    }

    pub fn spawn_organ_clinic<R: Rng>(
        &mut self,
        coord: Coord,
        level: usize,
        rng: &mut R,
    ) -> Entity {
        let pool = match level {
            0 => vec![
                OrganType::Heart,
                OrganType::Liver,
                OrganType::Lung,
                OrganType::Stomach,
            ],
            1 => vec![
                OrganType::Heart,
                OrganType::Liver,
                OrganType::Lung,
                OrganType::Stomach,
                OrganType::CronenbergPistol,
                OrganType::CronenbergShotgun,
                OrganType::Claw,
            ],
            2 => vec![
                OrganType::Heart,
                OrganType::Liver,
                OrganType::Lung,
                OrganType::Stomach,
                OrganType::CronenbergPistol,
                OrganType::CronenbergShotgun,
                OrganType::Claw,
            ],
            3 => vec![
                OrganType::Heart,
                OrganType::Liver,
                OrganType::Lung,
                OrganType::Stomach,
                OrganType::CronenbergPistol,
                OrganType::CronenbergShotgun,
                OrganType::Claw,
            ],
            _ => panic!(),
        };
        let cybernetic_chance = match level {
            0 => 0.0,
            1 => 0.2,
            2 => 0.4,
            3 => 0.8,
            _ => panic!(),
        };
        let mut simple_organs = Vec::new();
        if level > 0 {
            simple_organs.push(Organ {
                type_: OrganType::CyberCore,
                cybernetic: false,
                traits: OrganTraits::none(),
                original: false,
            });
        }
        for _ in 0..6 {
            let type_ = *pool.choose(rng).unwrap();
            let cybernetic = rng.gen::<f64>() < cybernetic_chance;
            simple_organs.push(Organ {
                type_,
                cybernetic,
                traits: OrganTraits::none(),
                original: false,
            });
        }
        for _ in 0..3 {
            let type_ = *pool.choose(rng).unwrap();
            let cybernetic = rng.gen::<f64>() < cybernetic_chance;
            let mut traits = OrganTraits::none();
            let random_trait = traits.get_mut(OrganTrait::choose(rng));
            *random_trait = true;
            simple_organs.push(Organ {
                type_,
                cybernetic,
                traits,
                original: false,
            });
        }
        for _ in 0..3 {
            let type_ = *pool.choose(rng).unwrap();
            let cybernetic = rng.gen::<f64>() < cybernetic_chance;
            let mut traits = OrganTraits::none();
            let random_trait = traits.get_mut(OrganTrait::choose(rng));
            *random_trait = true;
            let random_trait = traits.get_mut(OrganTrait::choose(rng));
            *random_trait = true;
            simple_organs.push(Organ {
                type_,
                cybernetic,
                traits,
                original: false,
            });
        }
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::OrganClinic,
                npc: Npc { disposition: Disposition::Neutral,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::OrganClinic,
                health: Meter::new_full(50),
                bump_damage: 10..=20,
                simple_organs,
                organ_clinic: (),
                shop: Shop {
                    message: "Welcome to the Organ Clinic. I'll pay good money for your original organs, assuming they are in good condition. I'll also remove any other organs you want to get rid of...for a fee. I'll also install organs from organ containers or my own personal collection. Why not see what I have in stock?.".to_string(),
                }
            },
        )
    }

    pub fn _spawn_organ_trader<R: Rng>(&mut self, coord: Coord, rng: &mut R) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::OrganClinic,
                npc: Npc { disposition: Disposition::Neutral,
                    movement: NpcMovement {
                        can_traverse_difficult: false,
                        can_open_doors: false,
                    },
                },
                character: (),
                npc_type: NpcType::OrganClinic,
                health: Meter::new_full(50),
                bump_damage: 5..=10,
                simple_organs: vec![
                    random_basic_organ(rng),
                    random_basic_organ(rng),
                ],
            },
        )
    }
}

fn make_gun(data: &mut EntityData) {
    match data.item {
        Some(Item::Pistol) => data.gun = Some(Gun::pistol()),
        Some(Item::Shotgun) => data.gun = Some(Gun::shotgun()),
        Some(Item::RocketLauncher) => data.gun = Some(Gun::rocket_launcher()),
        _ => (),
    }
}
