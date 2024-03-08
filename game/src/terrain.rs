use crate::world::{data::*, World};
use coord_2d::{Coord, Size};
use direction::Direction;
use procgen::city::{Map, TentacleSpec, Tile};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

pub struct Terrain {
    pub world: World,
}

impl Terrain {
    #[allow(unused)]
    pub fn generate_text() -> Self {
        let txt = include_str!("terrain.txt");
        let rows = txt.split('\n').collect::<Vec<_>>();
        let mut world = World::new(Size::new(50, 25));
        let mut rng = StdRng::from_entropy();
        for (y, row) in rows.into_iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let coord = Coord::new(x as i32, y as i32);
                let floor_entity = world.spawn_floor(coord);
                match ch {
                    '.' => (),
                    'C' => {
                        world.spawn_organ_clinic(coord, 0, &mut rng);
                    }
                    'I' => {
                        world.spawn_item_store(coord, &mut rng);
                    }
                    'G' => {
                        world.spawn_gun_store(coord, &mut rng);
                    }
                    'x' => {
                        world.spawn_corruptor(coord, &mut rng);
                    }
                    'z' => {
                        world.spawn_zombie(coord, &mut rng);
                    }
                    's' => {
                        world.spawn_snatcher(coord, &mut rng);
                    }
                    'c' => {
                        world.spawn_climber(coord, &mut rng);
                    }
                    'g' => {
                        world.spawn_glower(coord, &mut rng);
                    }
                    'p' => {
                        world.spawn_poisoner(coord, &mut rng);
                    }
                    'd' => {
                        world.spawn_divider(coord, &mut rng);
                    }
                    'v' => {
                        world.spawn_venter(coord, &mut rng);
                    }
                    '#' => {
                        world.spawn_wall(coord);
                    }
                    '+' => {
                        world.spawn_door(coord);
                    }
                    '%' => {
                        world.spawn_debris_burning(coord, &mut rng);
                    }
                    'r' => {
                        world.spawn_tentacle_glow(coord);
                    }
                    '>' => {
                        world.spawn_stairs_down(coord);
                    }
                    '<' => {
                        world.spawn_stairs_up(coord);
                    }
                    '$' => {
                        world.spawn_money(coord, &mut rng);
                    }
                    '~' => {
                        world
                            .components
                            .tile
                            .insert(floor_entity, crate::Tile::FloorPoison);
                        world.components.floor_poison.insert(floor_entity, ());
                    }
                    '1' => {
                        world.spawn_item(coord, Item::Pistol);
                    }
                    '2' => {
                        world.spawn_item(coord, Item::PistolAmmo);
                    }
                    '3' => {
                        world.spawn_item(coord, Item::Shotgun);
                    }
                    '4' => {
                        world.spawn_item(coord, Item::ShotgunAmmo);
                    }
                    '5' => {
                        world.spawn_item(coord, Item::RocketLauncher);
                    }
                    '6' => {
                        world.spawn_item(coord, Item::Rocket);
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
                                original: false,
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
        let tentacle_spec = TentacleSpec {
            num_tentacles: 2,
            segment_length: 2.,
            distance_from_centre: 35.0,
            spread: 0.2,
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
                    world.spawn_floor(coord);
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

        if level_index == crate::NUM_LEVELS - 1 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_corruptor(coord, rng);
            }
        }

        for (i, coord) in npc_spawn_candidates.iter().enumerate() {
            let mut good = true;
            for d in Direction::all() {
                if let Some(layers) = world.spatial_table.layers_at(*coord + d.coord()) {
                    if layers.feature.is_some() || layers.character.is_some() {
                        good = false;
                    }
                }
            }
            if good {
                let coord = *coord;
                npc_spawn_candidates.swap_remove(i);
                world.spawn_organ_clinic(coord, level_index, rng);
                break;
            }
        }

        for (i, coord) in npc_spawn_candidates.iter().enumerate() {
            let mut good = true;
            for d in Direction::all() {
                if let Some(layers) = world.spatial_table.layers_at(*coord + d.coord()) {
                    if layers.feature.is_some() || layers.character.is_some() {
                        good = false;
                    }
                }
            }
            if good {
                let coord = *coord;
                npc_spawn_candidates.swap_remove(i);
                world.spawn_item_store(coord, rng);
                break;
            }
        }

        for (i, coord) in npc_spawn_candidates.iter().enumerate() {
            let mut good = true;
            for d in Direction::all() {
                if let Some(layers) = world.spatial_table.layers_at(*coord + d.coord()) {
                    if layers.feature.is_some() || layers.character.is_some() {
                        good = false;
                    }
                }
            }
            if good {
                let coord = *coord;
                npc_spawn_candidates.swap_remove(i);
                world.spawn_gun_store(coord, rng);
                break;
            }
        }

        for _ in 0..6 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_money(coord, rng);
            }
        }

        for _ in 0..4 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                let roll = rng.gen::<f64>();
                if roll < 0.3 {
                    world.spawn_item(coord, Item::Antidote);
                } else if roll < 0.6 {
                    world.spawn_item(coord, Item::AntiRads);
                } else {
                    world.spawn_item(coord, Item::Stimpack);
                }
            }
        }
        if let Some(coord) = npc_spawn_candidates.pop() {
            world.spawn_item(coord, Item::Stimpack);
        }
        if let Some(coord) = npc_spawn_candidates.pop() {
            world.spawn_item(coord, Item::Antidote);
        }
        if let Some(coord) = npc_spawn_candidates.pop() {
            world.spawn_item(coord, Item::AntiRads);
        }
        for _ in 0..2 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_item(coord, Item::BloodVialEmpty);
            }
        }

        for _ in 0..2 {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_item(coord, Item::Food);
            }
        }

        let num_zombies;
        let num_climbers;
        let num_trespassers;
        let num_snatchers;
        let mut num_boomers = 0;
        let mut num_dividers = 0;
        let mut num_poisoners = 0;
        let mut num_venters = 0;
        let mut num_glowers = 0;
        match level_index {
            0 => {
                num_zombies = rng.gen_range(1..=2);
                num_climbers = rng.gen_range(1..=3);
                num_trespassers = 1;
                num_snatchers = 1;
                let hard_enemy_choice = rng.gen::<f64>();
                if hard_enemy_choice < 0.33 {
                    num_boomers = 1;
                } else if hard_enemy_choice < 0.66 {
                    num_dividers = 1;
                } else {
                    num_poisoners = 1;
                }
            }
            1 => {
                num_zombies = rng.gen_range(2..=3);
                num_climbers = rng.gen_range(2..=3);
                num_trespassers = rng.gen_range(1..=2);
                num_snatchers = rng.gen_range(1..=2);
                num_boomers = 1;
                num_dividers = 1;
                num_poisoners = 1;
                let hard_enemy_choice = rng.gen::<f64>();
                if hard_enemy_choice < 0.33 {
                    num_boomers = 0;
                } else if hard_enemy_choice < 0.66 {
                    num_dividers = 0;
                } else {
                    num_poisoners = 0;
                }
                num_venters = 1;
                num_glowers = 1;
            }
            2 => {
                num_zombies = rng.gen_range(1..=2);
                num_climbers = rng.gen_range(1..=2);
                num_trespassers = rng.gen_range(1..=2);
                num_snatchers = rng.gen_range(1..=2);
                num_boomers = rng.gen_range(1..=2);
                num_dividers = rng.gen_range(1..=2);
                num_poisoners = rng.gen_range(1..=2);
                num_venters = rng.gen_range(1..=2);
                num_glowers = rng.gen_range(1..=2);
            }
            3 => {
                num_zombies = rng.gen_range(1..=2);
                num_climbers = rng.gen_range(1..=2);
                num_trespassers = 1;
                num_snatchers = 1;
                let hard_enemy_choice = rng.gen::<f64>();
                if hard_enemy_choice < 0.33 {
                    num_boomers = 1;
                } else if hard_enemy_choice < 0.66 {
                    num_dividers = 1;
                } else {
                    num_poisoners = 1;
                }
                num_venters = 1;
                num_glowers = 1;
            }
            _ => panic!(),
        }

        for _ in 0..num_zombies {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_zombie(coord, rng);
            }
        }
        for _ in 0..num_climbers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_climber(coord, rng);
            }
        }
        for _ in 0..num_trespassers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_trespasser(coord, rng);
            }
        }
        for _ in 0..num_snatchers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_snatcher(coord, rng);
            }
        }
        for _ in 0..num_boomers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_boomer(coord, rng);
            }
        }
        for _ in 0..num_dividers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_divider(coord, rng);
            }
        }
        for _ in 0..num_poisoners {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_poisoner(coord, rng);
            }
        }
        for _ in 0..num_venters {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_venter(coord, rng);
            }
        }
        for _ in 0..num_glowers {
            if let Some(coord) = npc_spawn_candidates.pop() {
                world.spawn_glower(coord, rng);
            }
        }
        match level_index {
            0 => {
                let pool = vec![OrganType::Claw, OrganType::CronenbergPistol];
                if let Some(coord) = npc_spawn_candidates.pop() {
                    let organ = Organ {
                        type_: *pool.choose(rng).unwrap(),
                        cybernetic: false,
                        original: false,
                        traits: OrganTraits::with_one_random(rng),
                    };
                    world.spawn_item(coord, Item::OrganContainer(Some(organ)));
                }
                let pool = vec![
                    Item::PistolAmmo,
                    Item::PistolAmmo,
                    Item::PistolAmmo,
                    Item::ShotgunAmmo,
                    Item::Rocket,
                    Item::Pistol,
                ];
                for &item in pool.choose_multiple(rng, 4) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        world.spawn_item(coord, item);
                    }
                }
            }
            1 => {
                let pool = vec![
                    OrganType::Claw,
                    OrganType::CronenbergPistol,
                    OrganType::CronenbergShotgun,
                    OrganType::CyberCore,
                ];
                for &type_ in pool.choose_multiple(rng, 2) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        let organ = Organ {
                            type_,
                            cybernetic: false,
                            original: false,
                            traits: OrganTraits::with_one_random(rng),
                        };
                        world.spawn_item(coord, Item::OrganContainer(Some(organ)));
                    }
                }
                let pool = vec![
                    Item::Battery,
                    Item::Battery,
                    Item::PistolAmmo,
                    Item::PistolAmmo,
                    Item::ShotgunAmmo,
                    Item::ShotgunAmmo,
                    Item::Rocket,
                    Item::Rocket,
                    Item::Shotgun,
                    Item::Shotgun,
                    Item::Pistol,
                    Item::Pistol,
                ];
                for &item in pool.choose_multiple(rng, 6) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        world.spawn_item(coord, item);
                    }
                }
            }
            2 => {
                let pool = vec![
                    OrganType::Claw,
                    OrganType::CronenbergPistol,
                    OrganType::CronenbergShotgun,
                    OrganType::CyberCore,
                ];
                for &type_ in pool.choose_multiple(rng, 1) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        let organ = Organ {
                            type_,
                            cybernetic: false,
                            original: false,
                            traits: OrganTraits::with_one_random(rng),
                        };
                        world.spawn_item(coord, Item::OrganContainer(Some(organ)));
                    }
                }
                let pool = vec![
                    Item::Battery,
                    Item::Battery,
                    Item::PistolAmmo,
                    Item::PistolAmmo,
                    Item::ShotgunAmmo,
                    Item::ShotgunAmmo,
                    Item::Rocket,
                    Item::Rocket,
                ];
                for &item in pool.choose_multiple(rng, 4) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        world.spawn_item(coord, item);
                    }
                }
            }
            3 => {
                let pool = vec![
                    Item::Battery,
                    Item::Battery,
                    Item::PistolAmmo,
                    Item::PistolAmmo,
                    Item::ShotgunAmmo,
                    Item::ShotgunAmmo,
                    Item::Rocket,
                    Item::Rocket,
                ];
                for &item in pool.choose_multiple(rng, 8) {
                    if let Some(coord) = npc_spawn_candidates.pop() {
                        world.spawn_item(coord, item);
                    }
                }
            }
            _ => panic!(),
        }
        Self { world }
    }
}
