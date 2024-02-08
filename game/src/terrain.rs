use crate::{
    world::{
        data::EntityData,
        spatial::{Layer, Layers, Location},
        World,
    },
    Entity,
};
use coord_2d::{Coord, Size};
use entity_table::entity_data;
use procgen::{
    generate, generate_dungeon, Dungeon as DungeonGen, DungeonCell, Spec, WaterType, WorldCell3,
};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

pub struct Terrain {
    pub world: World,
    pub player_entity: Entity,
    pub num_dungeons: usize,
}

impl Terrain {
    pub fn generate<R: Rng>(
        player_data: EntityData,
        mut victories: Vec<crate::Victory>,
        rng: &mut R,
    ) -> Self {
        todo!()
    }
}
