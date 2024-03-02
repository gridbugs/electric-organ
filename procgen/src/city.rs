use coord_2d::{Coord, Size};
use direction::{CardinalDirection, Direction};
use grid_2d::Grid;
use rand::{seq::SliceRandom, Rng};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ground1 {
    Street,
    Alley,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile1 {
    Ground(Ground1),
    Wall,
}

struct StreetBuilder {
    coord: Coord,
    direction: CardinalDirection,
}

pub struct Map1 {
    grid: Grid<Tile1>,
    street_builders: Vec<StreetBuilder>,
}

struct Rect {
    coord: Coord,
    size: Size,
}

impl Rect {
    fn coord_iter<'a>(&'a self) -> impl Iterator<Item = Coord> + 'a {
        self.size
            .coord_iter_row_major()
            .map(|coord| coord + self.coord)
    }

    fn bottom_right(&self) -> Coord {
        self.coord + self.size.to_coord().unwrap() - Coord::new(1, 1)
    }
}

impl Map1 {
    fn new() -> Self {
        let grid = Grid::new_clone(Size::new(60, 30), Tile1::Wall);
        Self {
            grid,
            street_builders: Vec::new(),
        }
    }

    fn add_street_builders<R: Rng>(&mut self, rng: &mut R) {
        use CardinalDirection::*;
        let mut candidates = self
            .grid
            .edge_coord_iter()
            .filter_map(|coord| {
                let top = coord.y == 0;
                let bottom = coord.y == self.grid.height() as i32 - 1;
                let left = coord.x == 0;
                let right = coord.x == self.grid.width() as i32 - 1;
                if (top || bottom) && (coord.x < 12 || coord.x > self.grid.width() as i32 - 13) {
                    None
                } else if (left || right)
                    && (coord.y < 9 || coord.y > self.grid.height() as i32 - 10)
                {
                    None
                } else if top {
                    Some(StreetBuilder {
                        coord,
                        direction: South,
                    })
                } else if bottom {
                    Some(StreetBuilder {
                        coord,
                        direction: North,
                    })
                } else if left {
                    Some(StreetBuilder {
                        coord,
                        direction: East,
                    })
                } else {
                    assert!(right);
                    Some(StreetBuilder {
                        coord,
                        direction: West,
                    })
                }
            })
            .collect::<Vec<_>>();
        candidates.shuffle(rng);
        'outer: for candidate in candidates {
            for existing in &self.street_builders {
                match (candidate.direction, existing.direction) {
                    (North | South, North | South) => {
                        if (existing.coord.x - candidate.coord.x).abs() < 20 {
                            continue 'outer;
                        }
                    }
                    (East | West, East | West) => {
                        if (existing.coord.y - candidate.coord.y).abs() < 15 {
                            continue 'outer;
                        }
                    }
                    _ => (),
                }
            }
            self.street_builders.push(candidate);
        }
    }

    fn build_streets_tick<R: Rng>(&mut self, rng: &mut R) {
        let index = rng.gen_range(0..self.street_builders.len());
        let sb = &mut self.street_builders[index];
        if !sb.coord.is_valid(self.grid.size()) {
            self.street_builders.swap_remove(index);
            return;
        }
        if let Tile1::Ground(_) = *self.grid.get_checked(sb.coord) {
            if rng.gen::<f64>() < 0.5 {
                self.street_builders.swap_remove(index);
                return;
            }
        }
        *self.grid.get_checked_mut(sb.coord) = Tile1::Ground(Ground1::Street);
        sb.coord += sb.direction.coord();
    }

    fn build_streets<R: Rng>(&mut self, rng: &mut R) {
        while !self.street_builders.is_empty() {
            self.build_streets_tick(rng);
        }
    }

    fn split_rectangles<R: Rng>(&mut self, rng: &mut R) {
        let mut seen = Grid::new_copy(self.grid.size(), false);
        for coord in self.grid.coord_iter() {
            if let Tile1::Ground(_) = *self.grid.get_checked(coord) {
                continue;
            }
            if !*seen.get_checked(coord) {
                let width = {
                    let mut i = 0;
                    while let Some(&Tile1::Wall) = self.grid.get(coord + Coord::new(i, 0)) {
                        i += 1;
                    }
                    i as u32
                };
                let height = {
                    let mut i = 0;
                    while let Some(&Tile1::Wall) = self.grid.get(coord + Coord::new(0, i)) {
                        i += 1;
                    }
                    i as u32
                };
                let rect = Rect {
                    coord,
                    size: Size::new(width, height),
                };
                for coord in rect.coord_iter() {
                    *seen.get_checked_mut(coord) = true;
                }
                if width > 30 {
                    let split_x = width / 2; //rng.gen_range(width / 3..(2 * width) / 3);
                    let tile = if rng.gen::<f64>() < 0.5 {
                        Ground1::Alley
                    } else {
                        Ground1::Street
                    };

                    for i in 0..height {
                        let coord = coord + Coord::new(split_x as i32, i as i32);
                        *self.grid.get_checked_mut(coord) = Tile1::Ground(tile);
                    }
                }
                if height > 15 {
                    let split_y = height / 2; // rng.gen_range(height / 3..(2 * height) / 3);
                    let tile = if rng.gen::<f64>() < 0.5 {
                        Ground1::Alley
                    } else {
                        Ground1::Street
                    };

                    for i in 0..width {
                        let coord = coord + Coord::new(i as i32, split_y as i32);
                        *self.grid.get_checked_mut(coord) = Tile1::Ground(tile);
                    }
                }
            }
        }
    }

    // check if removing part of a street would create an L-shaped corner
    fn check_trim_candidate(&self, start: Coord) -> Option<Vec<Coord>> {
        use CardinalDirection::*;
        let direction = if start.x == 0 {
            East
        } else if start.y == 0 {
            South
        } else if start.x == self.grid.width() as i32 - 1 {
            West
        } else {
            assert!(start.y == self.grid.height() as i32 - 1);
            North
        };
        let mut coord = start;
        let mut to_remove = Vec::new();
        loop {
            if !self.grid.size().is_valid(coord) {
                return None;
            }
            if self.grid.get(coord) == Some(&Tile1::Ground(Ground1::Alley))
                || self.grid.get(coord + direction.left45().coord())
                    == Some(&Tile1::Ground(Ground1::Alley))
                || self.grid.get(coord + direction.right45().coord())
                    == Some(&Tile1::Ground(Ground1::Alley))
            {
                return None;
            }

            to_remove.push(coord);
            let mut left_right_count = 0;
            if let Some(Tile1::Ground(Ground1::Street)) =
                self.grid.get(coord + direction.left45().coord())
            {
                left_right_count += 1;
            }
            if let Some(Tile1::Ground(Ground1::Street)) =
                self.grid.get(coord + direction.right45().coord())
            {
                left_right_count += 1;
            }
            if left_right_count == 1 {
                return Some(to_remove);
            }
            if left_right_count == 2 {
                return None;
            }
            coord += direction.coord();
        }
    }

    fn trim_streets<R: Rng>(&mut self, rng: &mut R) {
        let mut candidates = self
            .grid
            .edge_enumerate()
            .filter_map(|(coord, &tile)| {
                if let Tile1::Ground(_) = tile {
                    self.check_trim_candidate(coord)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        candidates.shuffle(rng);
        for to_remove in candidates {
            if rng.gen::<f64>() < 0.5 {
                for coord in to_remove {
                    *self.grid.get_checked_mut(coord) = Tile1::Wall;
                }
            }
        }
    }

    fn is_boring_before_trimming(&self) -> bool {
        for (coord, tile) in self.grid.enumerate() {
            if let Tile1::Ground(_) = tile {
                let neighbour_street_count = CardinalDirection::all()
                    .filter(|d| {
                        if let Some(Tile1::Ground(_)) = self.grid.get(coord + d.coord()) {
                            true
                        } else {
                            false
                        }
                    })
                    .count();
                if neighbour_street_count == 3 {
                    return false;
                }
            }
        }
        true
    }

    fn widen_streets(&mut self) {
        use CardinalDirection::*;
        let mut grid1 = self.grid.clone();
        for (coord, &tile) in self.grid.enumerate() {
            if let Tile1::Ground(Ground1::Street) = tile {
                for d in [East] {
                    let coord = coord + d.coord();
                    if coord.is_valid(grid1.size()) {
                        *grid1.get_checked_mut(coord) = Tile1::Ground(Ground1::Street);
                    }
                }
            }
        }
        let mut grid2 = grid1.clone();
        for (coord, &tile) in grid1.enumerate() {
            if let Tile1::Ground(Ground1::Street) = tile {
                let coord = coord + South.coord();
                if coord.is_valid(grid1.size()) {
                    *grid2.get_checked_mut(coord) = Tile1::Ground(Ground1::Street);
                }
            }
        }
        let mut grid3 = grid2.clone();
        for (coord, &tile) in grid2.enumerate() {
            if let Tile1::Ground(Ground1::Street) = tile {
                let num_ground_neighbours = CardinalDirection::all()
                    .filter(|d| {
                        if let Some(Tile1::Wall) = grid2.get(coord + d.coord()) {
                            false
                        } else {
                            true
                        }
                    })
                    .count();
                if num_ground_neighbours <= 2 {
                    *grid3.get_checked_mut(coord) = Tile1::Wall;
                }
            }
        }
        let mut grid4 = grid3.clone();
        for (coord, &tile) in grid3.enumerate() {
            if let Tile1::Ground(Ground1::Street) = tile {
                let num_ground_neighbours = CardinalDirection::all()
                    .filter(|d| {
                        if let Some(Tile1::Wall) = grid3.get(coord + d.coord()) {
                            false
                        } else {
                            true
                        }
                    })
                    .count();
                if num_ground_neighbours <= 1 {
                    *grid4.get_checked_mut(coord) = Tile1::Wall;
                }
            }
        }
        self.grid = grid4;
    }

    pub fn print(&self) {
        for row in self.grid.rows() {
            for cell in row {
                match cell {
                    Tile1::Ground(Ground1::Street) => print!("s"),
                    Tile1::Ground(Ground1::Alley) => print!("a"),
                    Tile1::Wall => print!("."),
                }
            }
            println!("");
        }
    }

    fn is_boring(&self) -> bool {
        for &cell in self.grid.iter() {
            if cell == Tile1::Ground(Ground1::Alley) {
                return false;
            }
        }
        true
    }

    pub fn generate<R: Rng>(rng: &mut R) -> Self {
        let mut map = loop {
            let mut map = loop {
                let mut map = Self::new();
                map.add_street_builders(rng);
                map.build_streets(rng);
                map.split_rectangles(rng);
                if map.is_boring_before_trimming() {
                    continue;
                }
                break map;
            };
            map.trim_streets(rng);
            if map.is_boring() {
                continue;
            }
            break map;
        };
        map.widen_streets();
        map
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile2 {
    Wall,
    Street,
    Alley,
    Footpath,
}

pub struct Map2 {
    grid: Grid<Tile2>,
}

impl Map2 {
    fn from_map1(map1: &Map1) -> Self {
        let grid = Grid::new_fn(map1.grid.size(), |coord| {
            match map1.grid.get_checked(coord) {
                Tile1::Wall => Tile2::Wall,
                Tile1::Ground(Ground1::Alley) => Tile2::Alley,
                Tile1::Ground(Ground1::Street) => Tile2::Street,
            }
        });
        Self { grid }
    }

    fn add_footpath(&mut self) {
        let mut grid1 = self.grid.clone();
        for (coord, &tile) in self.grid.enumerate() {
            if let Tile2::Street = tile {
                for d in Direction::all() {
                    let coord = coord + d.coord();
                    if let Some(Tile2::Wall) = self.grid.get(coord) {
                        *grid1.get_checked_mut(coord) = Tile2::Footpath;
                    }
                }
            }
        }
        let mut grid2 = grid1.clone();
        for (coord, &tile) in grid1.enumerate() {
            if let Tile2::Footpath = tile {
                let num_footpath_neighbours = Direction::all()
                    .filter(|d| grid1.get(coord + d.coord()) == Some(&Tile2::Footpath))
                    .count();
                let num_street_neighbours = Direction::all()
                    .filter(|d| {
                        grid1.get(coord + d.coord()) == Some(&Tile2::Street)
                            || grid1.get(coord + d.coord()) == Some(&Tile2::Alley)
                    })
                    .count();
                let num_wall_neighbours = Direction::all()
                    .filter(|d| grid1.get(coord + d.coord()) == Some(&Tile2::Wall))
                    .count();
                if num_footpath_neighbours == 4
                    && num_street_neighbours == 3
                    && num_wall_neighbours == 1
                {
                    for d in Direction::all() {
                        let coord = coord + d.coord();
                        if let Some(cell) = grid2.get_mut(coord) {
                            if *cell == Tile2::Wall {
                                *cell = Tile2::Footpath;
                            }
                        }
                    }
                }
            }
        }
        self.grid = grid2;
    }

    pub fn print(&self) {
        for row in self.grid.rows() {
            for cell in row {
                match cell {
                    Tile2::Street => print!("s"),
                    Tile2::Alley => print!("a"),
                    Tile2::Footpath => print!("f"),
                    Tile2::Wall => print!("."),
                }
            }
            println!("");
        }
    }

    pub fn generate<R: Rng>(rng: &mut R) -> Self {
        let map1 = Map1::generate(rng);
        let mut map2 = Self::from_map1(&map1);
        map2.add_footpath();
        map2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile3 {
    Wall,
    Street,
    Alley,
    Footpath,
    Floor,
}

pub struct Map3 {
    grid: Grid<Tile3>,
}

struct Block {
    coords: HashSet<Coord>,
}

struct BlockSplitByWall {
    block1: Block,
    block2: Block,
    wall: Vec<Coord>,
}

impl Block {
    fn bounding_box(&self) -> Rect {
        let mut x_min = i32::MAX;
        let mut y_min = i32::MAX;
        let mut x_max = i32::MIN;
        let mut y_max = i32::MIN;
        for coord in self.coords.iter() {
            x_min = x_min.min(coord.x);
            y_min = y_min.min(coord.y);
            x_max = x_max.max(coord.x);
            y_max = y_max.max(coord.y);
        }
        let coord = Coord::new(x_min, y_min);
        let size = (Coord::new(x_max, y_max) - coord + Coord::new(1, 1))
            .to_size()
            .unwrap();
        Rect { coord, size }
    }

    fn split_horizontal(&self, y: i32) -> BlockSplitByWall {
        let block1 = Self {
            coords: self
                .coords
                .iter()
                .filter(|coord| coord.y < y)
                .cloned()
                .collect(),
        };
        let block2 = Self {
            coords: self
                .coords
                .iter()
                .filter(|coord| coord.y > y)
                .cloned()
                .collect(),
        };
        let wall = self
            .coords
            .iter()
            .filter(|coord| coord.y == y)
            .cloned()
            .collect();
        BlockSplitByWall {
            block1,
            block2,
            wall,
        }
    }

    fn split_vertical(&self, x: i32) -> BlockSplitByWall {
        let block1 = Self {
            coords: self
                .coords
                .iter()
                .filter(|coord| coord.x < x)
                .cloned()
                .collect(),
        };
        let block2 = Self {
            coords: self
                .coords
                .iter()
                .filter(|coord| coord.x > x)
                .cloned()
                .collect(),
        };
        let wall = self
            .coords
            .iter()
            .filter(|coord| coord.x == x)
            .cloned()
            .collect();
        BlockSplitByWall {
            block1,
            block2,
            wall,
        }
    }
}

impl Map3 {
    fn from_map2(map2: &Map2) -> Self {
        let grid = Grid::new_fn(map2.grid.size(), |coord| {
            match map2.grid.get_checked(coord) {
                Tile2::Wall => Tile3::Wall,
                Tile2::Street => Tile3::Street,
                Tile2::Alley => Tile3::Alley,
                Tile2::Footpath => Tile3::Footpath,
            }
        });
        Self { grid }
    }

    fn blocks(&self) -> Vec<Block> {
        let mut seen = self.grid.map_ref(|_| false);
        let mut ret = Vec::new();
        for (coord, tile) in self.grid.enumerate() {
            if let Tile3::Wall = tile {
                if *seen.get_checked(coord) {
                    continue;
                }
                let mut queue = VecDeque::new();
                queue.push_front(coord);
                *seen.get_checked_mut(coord) = true;
                let mut visited = HashSet::new();
                visited.insert(coord);
                while let Some(coord) = queue.pop_back() {
                    for d in Direction::all() {
                        let coord = coord + d.coord();
                        if let Some(Tile3::Wall) = self.grid.get(coord) {
                            if seen.get(coord) == Some(&false) {
                                *seen.get_checked_mut(coord) = true;
                                queue.push_front(coord);
                                visited.insert(coord);
                            }
                        }
                    }
                }
                ret.push(Block { coords: visited });
            }
        }
        ret
    }

    fn add_floor(&mut self) -> Vec<Block> {
        let mut floor_blocks = Vec::new();
        let blocks = self.blocks();
        let mut grid1 = self.grid.clone();
        for block in blocks {
            let mut floor_block = Block {
                coords: HashSet::new(),
            };
            for coord in block.coords {
                let mut wall_neighbour_count = 0;
                for d in Direction::all() {
                    let coord = coord + d.coord();
                    if let Some(Tile3::Wall) = self.grid.get(coord) {
                        wall_neighbour_count += 1;
                    }
                }
                if wall_neighbour_count == 8 {
                    floor_block.coords.insert(coord);
                    *grid1.get_checked_mut(coord) = Tile3::Floor;
                }
            }
            floor_blocks.push(floor_block);
        }
        self.grid = grid1;
        floor_blocks
    }

    fn split_floor_block<R: Rng>(&mut self, block: Block, depth: usize, rng: &mut R) {
        let bb = block.bounding_box();
        if bb.size.height() == 1 || bb.size.width() == 1 {
            // don't split accidental corridors
            return;
        }
        if bb.size.height() > 7 && bb.size.width() > 11 && rng.gen::<f64>() < 0.75 && depth > 0 {
            // corridor split
            let (split1, split2) = if bb.size.width() > (3 * bb.size.height() / 2) {
                let x_min = bb.coord.x + 3;
                let x_max = bb.bottom_right().x - 6;
                let x = rng.gen_range(x_min..=x_max);
                let split1 = block.split_vertical(x);
                let split2 = split1.block2.split_vertical(x + 2);
                (split1, split2)
            } else {
                let y_min = bb.coord.y + 2;
                let y_max = bb.bottom_right().y - 4;
                let y = rng.gen_range(y_min..=y_max);
                let split1 = block.split_horizontal(y);
                let split2 = split1.block2.split_horizontal(y + 2);
                (split1, split2)
            };
            for coord in split1.wall {
                *self.grid.get_checked_mut(coord) = Tile3::Wall;
            }
            for coord in split2.wall {
                *self.grid.get_checked_mut(coord) = Tile3::Wall;
            }
            self.split_floor_block(split1.block1, depth + 1, rng);
            self.split_floor_block(split2.block2, depth + 1, rng);
        } else if bb.size.height() > 6 || bb.size.width() > 10 {
            // wall split
            let BlockSplitByWall {
                block1,
                block2,
                wall,
            } = if bb.size.width() > (3 * bb.size.height() / 2) {
                let x_min = bb.coord.x + 5;
                let x_max = bb.bottom_right().x - 5;
                let x = rng.gen_range(x_min..=x_max);
                block.split_vertical(x)
            } else {
                let y_min = bb.coord.y + 3;
                let y_max = bb.bottom_right().y - 3;
                let y = rng.gen_range(y_min..=y_max);
                block.split_horizontal(y)
            };
            for coord in wall {
                *self.grid.get_checked_mut(coord) = Tile3::Wall;
            }
            self.split_floor_block(block1, depth + 1, rng);
            self.split_floor_block(block2, depth + 1, rng);
        }
    }

    fn clean_corridors(&mut self) {
        use Direction::*;
        let mut grid1 = self.grid.clone();
        for (coord, tile) in self.grid.enumerate() {
            if let Tile3::Wall = tile {
                if self.grid.get(coord + NorthEast.coord()) == Some(&Tile3::Wall)
                    && self.grid.get(coord + NorthWest.coord()) == Some(&Tile3::Wall)
                    && self.grid.get(coord + SouthEast.coord()) == Some(&Tile3::Wall)
                    && self.grid.get(coord + SouthWest.coord()) == Some(&Tile3::Wall)
                {
                    *grid1.get_checked_mut(coord) = Tile3::Floor;
                }
            }
        }
        self.grid = grid1;
    }

    pub fn print(&self) {
        for row in self.grid.rows() {
            for cell in row {
                match cell {
                    Tile3::Street => print!("."),
                    Tile3::Alley => print!(","),
                    Tile3::Footpath => print!(","),
                    Tile3::Wall => print!("#"),
                    Tile3::Floor => print!("."),
                }
            }
            println!("");
        }
    }

    pub fn generate<R: Rng>(rng: &mut R) -> Self {
        let map2 = Map2::generate(rng);
        let mut map3 = Self::from_map2(&map2);
        let floor_blocks = map3.add_floor();
        for block in floor_blocks {
            map3.split_floor_block(block, 0, rng);
        }
        map3.clean_corridors();
        map3
    }
}
