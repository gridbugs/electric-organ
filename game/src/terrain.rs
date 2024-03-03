use crate::world::World;
use coord_2d::{Coord, Size};
use procgen::city::{Map, TentacleSpec, Tile};
use rand::Rng;

pub struct Terrain {
    pub world: World,
}

impl Terrain {
    #[allow(unused)]
    pub fn generate_text() -> Self {
        let txt = include_str!("terrain.txt");
        let rows = txt.split('\n').collect::<Vec<_>>();
        let mut world = World::new(Size::new(rows[0].len() as u32, rows.len() as u32));
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
                    _ => log::warn!("unexpected char: {}", ch),
                }
            }
        }
        Self { world }
    }

    pub fn generate<R: Rng>(level_index: usize, rng: &mut R) -> Self {
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
        for (coord, &tile) in map.grid.enumerate() {
            match tile {
                Tile::Street => {
                    world.spawn_street(coord);
                }
                Tile::Alley => {
                    world.spawn_alley(coord);
                }
                Tile::Footpath => {
                    world.spawn_footpath(coord);
                }
                Tile::Wall => {
                    world.spawn_floor(coord);
                    world.spawn_wall(coord);
                }
                Tile::Floor => {
                    world.spawn_floor(coord);
                }
                Tile::Debris => {
                    if debris_count % 5 == 0 {
                        world.spawn_debris_burning(coord, rng);
                    } else {
                        world.spawn_debris(coord);
                    }
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
                    if level_index > 0 {
                        world.spawn_stairs_up(coord);
                    } else {
                        world.spawn_exit(coord);
                    }
                }
            }
        }
        Self { world }
    }
}
