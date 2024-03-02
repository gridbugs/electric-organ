use crate::{
    world::{
        data::EntityData,
        spatial::{Layer, Location},
        World,
    },
    Entity,
};
use coord_2d::{Coord, Size};
use procgen::city::{Map, TentacleSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub struct Terrain {
    pub world: World,
    pub player_entity: Entity,
}

impl Terrain {
    #[allow(unused)]
    pub fn generate_text(player_data: EntityData) -> Self {
        let mut player_entity: Option<Entity> = None;
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
                    '@' => {
                        let player_location = Location {
                            layer: Some(Layer::Character),
                            coord,
                        };
                        player_entity =
                            Some(world.insert_entity_data(player_location, player_data.clone()));
                    }
                    _ => log::warn!("unexpected char: {}", ch),
                }
            }
        }
        let player_entity = player_entity.expect("no player in terrain file");
        Self {
            world,
            player_entity,
        }
    }

    pub fn generate<R: Rng>(player_data: EntityData, rng: &mut R) -> Self {
        let tentacle_spec = TentacleSpec {
            num_tentacles: 3,
            segment_length: 1.5,
            distance_from_centre: 35.0,
            spread: 0.3,
        };
        let map = Map::generate(&tentacle_spec, rng);
        todo!()
    }
}
