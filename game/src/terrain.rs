use crate::world::{data::*, World};
use coord_2d::{Coord, Size};
use procgen::city::{Map, TentacleSpec, Tile};
use rand::{seq::SliceRandom, Rng};

pub struct Terrain {
    pub world: World,
}

impl Terrain {
    #[allow(unused)]
    pub fn generate_text() -> Self {
        let txt = include_str!("terrain.txt");
        let rows = txt.split('\n').collect::<Vec<_>>();
        let mut world = World::new(Size::new(50, 25));
        for (y, row) in rows.into_iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let coord = Coord::new(x as i32, y as i32);
                world.spawn_floor(coord);
                match ch {
                    '.' => (),
                    '#' => {
                        world.spawn_wall(coord);
                    }
                    '+' => {
                        world.spawn_door(coord);
                    }
                    '>' => {
                        world.spawn_stairs_down(coord);
                    }
                    '<' => {
                        world.spawn_stairs_up(coord);
                    }
                    '$' => {
                        world.spawn_money(coord);
                    }
                    '1' => {
                        world.spawn_item(coord, Item::Stimpack);
                    }
                    '2' => {
                        world.spawn_item(coord, Item::Antidote);
                    }
                    '3' => {
                        world.spawn_item(coord, Item::Food);
                    }
                    '4' => {
                        world.spawn_item(coord, Item::BloodVialEmpty);
                    }
                    '5' => {
                        world.spawn_item(coord, Item::BloodVialFull);
                    }
                    '6' => {
                        world.spawn_item(coord, Item::Battery);
                    }
                    '7' => {
                        world.spawn_item(coord, Item::AntiRads);
                    }
                    '8' => {
                        world.spawn_item(coord, Item::OrganContainer(None));
                    }
                    '9' => {
                        world.spawn_item(
                            coord,
                            Item::OrganContainer(Some(Organ {
                                type_: OrganType::Lung,
                                traits: OrganTraits {
                                    vampiric: true,
                                    damaged: true,
                                    ..OrganTraits::none()
                                },
                                cybernetic: false,
                            })),
                        );
                    }
                    _ => log::warn!("unexpected char: {}", ch),
                }
            }
        }
        Self { world }
    }

    pub fn generate<R: Rng>(level_index: usize, rng: &mut R) -> Self {
        return Self::generate_text();
        let tentacle_spec = TentacleSpec {
            num_tentacles: 3,
            segment_length: 1.5,
            distance_from_centre: 35.0,
            spread: 0.3,
        };
        let map = Map::generate(&tentacle_spec, rng);
        let mut world = World::new(map.grid.size());
        let mut tentacle_count = 0;
        let mut debris_count = 0;
        let mut empty_space = Vec::new();
        let mut player_spawn = None;
        for (coord, &tile) in map.grid.enumerate() {
            match tile {
                Tile::Street => {
                    empty_space.push(coord);
                    world.spawn_street(coord);
                }
                Tile::Alley => {
                    empty_space.push(coord);
                    world.spawn_alley(coord);
                }
                Tile::Footpath => {
                    empty_space.push(coord);
                    world.spawn_footpath(coord);
                }
                Tile::Wall => {
                    world.spawn_floor(coord);
                    world.spawn_wall(coord);
                }
                Tile::Floor => {
                    empty_space.push(coord);
                    world.spawn_floor(coord);
                }
                Tile::Debris => {
                    if debris_count % 5 == 0 {
                        world.spawn_debris_burning(coord, rng);
                    } else {
                        world.spawn_debris(coord);
                    }
                    world.spawn_floor(coord);
                    debris_count += 1;
                }
                Tile::Door => {
                    world.spawn_floor(coord);
                    world.spawn_door(coord);
                }
                Tile::Tentacle => {
                    if tentacle_count % 10 == 0 {
                        world.spawn_tentacle_glow(coord);
                    } else {
                        world.spawn_tentacle(coord);
                    }
                    tentacle_count += 1;
                }
                Tile::StairsDown => {
                    if level_index < crate::NUM_LEVELS - 1 {
                        world.spawn_stairs_down(coord);
                    }
                }
                Tile::StairsUp => {
                    player_spawn = Some(coord);
                    if level_index > 0 {
                        world.spawn_stairs_up(coord);
                    } else {
                        world.spawn_exit(coord);
                    }
                }
            }
        }
        let player_spawn = player_spawn.expect("no player spawn");
        let mut npc_spawn_candidates = empty_space
            .iter()
            .cloned()
            .filter(|coord| coord.manhattan_distance(player_spawn) > 8)
            .collect::<Vec<_>>();
        npc_spawn_candidates.shuffle(rng);
        for _ in 0..0 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_zombie(coord);
            }
        }
        for _ in 0..0 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_climber(coord);
            }
        }
        for _ in 0..0 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_trespasser(coord);
            }
        }
        for _ in 0..0 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_boomer(coord);
            }
        }
        Self { world }
    }
}
