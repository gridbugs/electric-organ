use coord_2d::{Coord, Size};
use direction::{CardinalDirection, Direction, OrdinalDirection};
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
        let grid = Grid::new_clone(Size::new(50, 25), Tile1::Wall);
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
                if width > self.grid.width() / 2 {
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
                if height > self.grid.height() / 2 {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile4 {
    Wall,
    Street,
    Alley,
    Footpath,
    Floor,
    Debris,
    Door,
}

impl Tile4 {
    fn is_solid(&self) -> bool {
        match self {
            Self::Wall | Self::Debris => true,
            _ => false,
        }
    }
    fn is_open(&self) -> bool {
        !self.is_solid()
    }
    fn is_outside(&self) -> bool {
        match self {
            Self::Street | Self::Alley | Self::Footpath => true,
            _ => false,
        }
    }
}

struct Area {
    boundary: HashSet<Coord>,
}

struct Adjacencies {
    areas: Vec<Area>,
    shared_boundaries: Grid<HashSet<Coord>>,
    neighbours: Vec<Vec<usize>>,
}

impl Adjacencies {
    fn shared_boundary(
        shared_boundaries: &Grid<HashSet<Coord>>,
        a: usize,
        b: usize,
    ) -> &HashSet<Coord> {
        let coord = if a < b {
            Coord::new(a as i32, b as i32)
        } else {
            Coord::new(b as i32, a as i32)
        };
        shared_boundaries.get_checked(coord)
    }

    fn neighbours(&self, i: usize) -> &[usize] {
        &self.neighbours[i]
    }
}

struct DisconnectedRooms {
    as_coords: HashSet<Coord>,
}

impl DisconnectedRooms {
    fn all_disconnected(adjacencies: &Adjacencies) -> Self {
        Self {
            as_coords: adjacencies
                .shared_boundaries
                .enumerate()
                .filter_map(|(coord, shared_boundary)| {
                    if shared_boundary.is_empty() {
                        None
                    } else {
                        Some(coord)
                    }
                })
                .collect(),
        }
    }

    fn connect(&mut self, a: usize, b: usize) {
        let coord = if a < b {
            Coord::new(a as i32, b as i32)
        } else {
            Coord::new(b as i32, a as i32)
        };
        self.as_coords.remove(&coord);
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.as_coords
            .iter()
            .map(|coord| (coord.x as usize, coord.y as usize))
    }
}

pub struct Map4 {
    grid: Grid<Tile4>,
}

impl Map4 {
    fn from_map3(map3: &Map3) -> Self {
        Self {
            grid: map3.grid.map_ref(|&tile| match tile {
                Tile3::Wall => Tile4::Wall,
                Tile3::Street => Tile4::Street,
                Tile3::Alley => Tile4::Alley,
                Tile3::Footpath => Tile4::Footpath,
                Tile3::Floor => Tile4::Floor,
            }),
        }
    }

    fn add_debris<R: Rng>(&mut self, rng: &mut R) {
        let mut outside_coords = self
            .grid
            .enumerate()
            .filter_map(|(coord, &tile)| {
                if tile.is_outside()
                    && coord.x > 10
                    && coord.x < (self.grid.width() as i32 - 10)
                    && coord.y > 7
                    && coord.y < (self.grid.height() as i32 - 7)
                {
                    Some(coord)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        outside_coords.shuffle(rng);
        for _ in 0..4 {
            if let Some(mut coord) = outside_coords.pop() {
                for _ in 0..8 {
                    *self.grid.get_checked_mut(coord) = Tile4::Debris;
                    let new_coord = coord + rng.gen::<Direction>().coord();
                    if let Some(tile) = self.grid.get(new_coord) {
                        if tile.is_outside() {
                            coord = new_coord;
                        }
                    }
                }
            }
        }
        let mut open_coords = self
            .grid
            .enumerate()
            .filter_map(|(coord, &tile)| {
                if tile.is_open()
                    && (!tile.is_outside()
                        || (coord.x > 10
                            && coord.x < (self.grid.width() as i32 - 10)
                            && coord.y > 7
                            && coord.y < (self.grid.height() as i32 - 7)))
                {
                    Some(coord)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        open_coords.shuffle(rng);
        for _ in 0..20 {
            if let Some(coord) = open_coords.pop() {
                *self.grid.get_checked_mut(coord) = Tile4::Debris;
            }
        }
    }

    fn areas(&self) -> Vec<Area> {
        use CardinalDirection::*;
        let mut seen = HashSet::new();
        let mut ret = Vec::new();
        for (coord, &tile) in self.grid.enumerate() {
            if !tile.is_solid() && seen.insert(coord) {
                let mut boundary = HashSet::new();
                let mut area_coords = HashSet::new();
                let mut queue = VecDeque::new();
                area_coords.insert(coord);
                queue.push_back(coord);
                while let Some(coord) = queue.pop_front() {
                    for d in CardinalDirection::all() {
                        let coord = coord + d.coord();
                        if let Some(&tile) = self.grid.get(coord) {
                            if tile.is_solid() {
                                if tile == Tile4::Wall {
                                    let valid_h = self.grid.get(coord + East.coord())
                                        == Some(&Tile4::Wall)
                                        && self.grid.get(coord + West.coord())
                                            == Some(&Tile4::Wall);
                                    let valid_v = self.grid.get(coord + North.coord())
                                        == Some(&Tile4::Wall)
                                        && self.grid.get(coord + South.coord())
                                            == Some(&Tile4::Wall);
                                    if valid_h || valid_v {
                                        boundary.insert(coord);
                                    }
                                }
                            } else {
                                if area_coords.insert(coord) {
                                    seen.insert(coord);
                                    queue.push_back(coord);
                                }
                            }
                        }
                    }
                }
                ret.push(Area { boundary });
            }
        }
        ret
    }

    fn adjacencies(&self) -> Adjacencies {
        let areas = self.areas();
        let shared_boundaries =
            Grid::new_fn(Size::new(areas.len() as u32, areas.len() as u32), |coord| {
                let a = coord.x as usize;
                let b = coord.y as usize;
                if a < b {
                    let a = &areas[a];
                    let b = &areas[b];
                    a.boundary.intersection(&b.boundary).cloned().collect()
                } else {
                    HashSet::new()
                }
            });
        let neighbours = (0..areas.len())
            .map(|i| {
                (0..areas.len())
                    .filter_map(|j| {
                        let shared_boundary =
                            Adjacencies::shared_boundary(&shared_boundaries, i, j);
                        if shared_boundary.is_empty() {
                            None
                        } else {
                            Some(j)
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        Adjacencies {
            areas,
            shared_boundaries,
            neighbours,
        }
    }

    fn add_doors<R: Rng>(&mut self, rng: &mut R) {
        let adjacencies = self.adjacencies();
        let mut disconnected_rooms = DisconnectedRooms::all_disconnected(&adjacencies);
        let spanning_tree_start_index = rng.gen_range(0..adjacencies.areas.len());
        let mut to_visit = vec![spanning_tree_start_index];
        let mut seen = HashSet::new();
        seen.insert(spanning_tree_start_index);
        while !to_visit.is_empty() {
            let index_to_visit = to_visit.swap_remove(rng.gen_range(0..to_visit.len()));
            for &neighbour_index in adjacencies.neighbours(index_to_visit) {
                if seen.insert(neighbour_index) {
                    to_visit.push(neighbour_index);
                    let shared_boundary = Adjacencies::shared_boundary(
                        &adjacencies.shared_boundaries,
                        index_to_visit,
                        neighbour_index,
                    )
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                    let &door_coord = shared_boundary
                        .choose(rng)
                        .expect("Shared boundary between rooms should not be empty");
                    *self.grid.get_checked_mut(door_coord) = Tile4::Door;
                    disconnected_rooms.connect(index_to_visit, neighbour_index);
                }
            }
        }
        let mut disconnected_rooms = disconnected_rooms.iter().collect::<Vec<_>>();
        disconnected_rooms.shuffle(rng);
        for _ in 0..(2 * disconnected_rooms.len() / 3) {
            if let Some((a, b)) = disconnected_rooms.pop() {
                let shared_boundary =
                    Adjacencies::shared_boundary(&adjacencies.shared_boundaries, a, b)
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>();
                let &door_coord = shared_boundary
                    .choose(rng)
                    .expect("Shared boundary between rooms should not be empty");
                *self.grid.get_checked_mut(door_coord) = Tile4::Door;
            }
        }
    }

    pub fn print(&self) {
        for row in self.grid.rows() {
            for cell in row {
                match cell {
                    Tile4::Street => print!("."),
                    Tile4::Alley => print!(","),
                    Tile4::Footpath => print!(","),
                    Tile4::Wall => print!("#"),
                    Tile4::Floor => print!("."),
                    Tile4::Debris => print!("%"),
                    Tile4::Door => print!("+"),
                }
            }
            println!("");
        }
    }

    pub fn generate<R: Rng>(rng: &mut R) -> Self {
        let map3 = Map3::generate(rng);
        let mut map4 = Self::from_map3(&map3);
        map4.add_debris(rng);
        map4.add_doors(rng);
        map4
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile5 {
    Wall,
    Street,
    Alley,
    Footpath,
    Floor,
    Debris,
    Door,
    Tentacle,
    StairsDown,
    StairsUp,
}

impl Tile5 {
    fn is_solid(&self) -> bool {
        match self {
            Self::Wall | Self::Debris | Self::Tentacle => true,
            _ => false,
        }
    }
    fn is_open(&self) -> bool {
        !self.is_solid()
    }
}

pub struct Map5 {
    pub grid: Grid<Tile5>,
    tentacle_coords: Vec<Coord>,
}

pub struct TentacleSpec {
    pub num_tentacles: usize,
    pub segment_length: f64,
    pub spread: f64,
    pub distance_from_centre: f64,
}

impl Map5 {
    fn from_map4(map4: &Map4) -> Self {
        Self {
            grid: map4.grid.map_ref(|&tile| match tile {
                Tile4::Wall => Tile5::Wall,
                Tile4::Street => Tile5::Street,
                Tile4::Alley => Tile5::Alley,
                Tile4::Footpath => Tile5::Footpath,
                Tile4::Floor => Tile5::Floor,
                Tile4::Debris => Tile5::Debris,
                Tile4::Door => Tile5::Door,
            }),
            tentacle_coords: Vec::new(),
        }
    }

    fn add_tentacles<R: Rng>(
        &mut self,
        corner: OrdinalDirection,
        tentacle_spec: &TentacleSpec,
        rng: &mut R,
    ) {
        use vector::*;
        let bottom_right = self.grid.size().to_coord().unwrap();
        let corner = match corner {
            OrdinalDirection::NorthEast => bottom_right.set_y(0),
            OrdinalDirection::SouthEast => bottom_right,
            OrdinalDirection::SouthWest => bottom_right.set_x(0),
            OrdinalDirection::NorthWest => Coord::new(0, 0),
        };
        let towards_map_centre = Cartesian::from_coord(Coord::new(
            self.grid.width() as i32 / 2,
            self.grid.height() as i32 / 2,
        ))
        .sub(Cartesian::from_coord(corner));
        let angle = towards_map_centre.to_radial().angle;
        let centre = Cartesian {
            x: self.grid.width() as f64 / 2.0,
            y: self.grid.height() as f64 / 2.0,
        }
        .add(
            Radial {
                length: -tentacle_spec.distance_from_centre,
                angle,
            }
            .to_cartesian(),
        );
        let delta = Radial {
            length: tentacle_spec.segment_length,
            angle,
        };
        for t in 0..tentacle_spec.num_tentacles {
            let mut prev_coord = None;
            let mut delta = delta;
            let base_angle_delta = ((t as f64 * tentacle_spec.spread)
                / tentacle_spec.num_tentacles as f64)
                - (tentacle_spec.spread / 2.0);
            let bend = base_angle_delta + (0.2 * rng.gen::<f64>() - 0.1);
            delta.angle.0 += bend;
            let mut vector = centre;
            let num_steps = 15;
            for i in 0..num_steps {
                vector = vector.add(delta.to_cartesian());
                let coord = vector.to_coord_round_nearest();
                if let Some(prev_coord) = prev_coord {
                    for coord in line_2d::coords_between(prev_coord, coord) {
                        let size = (num_steps - i) / 4 + 1;
                        let rect = Rect {
                            coord,
                            size: Size::new(size, size),
                        };
                        delta.angle.0 += bend * 0.2 * rng.gen::<f64>();
                        for coord in rect.coord_iter() {
                            if let Some(tile) = self.grid.get_mut(coord) {
                                *tile = Tile5::Tentacle;
                                self.tentacle_coords.push(coord);
                            }
                        }
                    }
                }
                prev_coord = Some(coord);
            }
        }
    }

    pub fn add_stairs<R: Rng>(
        &mut self,
        corner: OrdinalDirection,
        tile: Tile5,
        rng: &mut R,
    ) -> Option<Coord> {
        let bottom_right = self.grid.size().to_coord().unwrap();
        let corner_coord = match corner {
            OrdinalDirection::NorthEast => bottom_right.set_y(0),
            OrdinalDirection::SouthEast => bottom_right,
            OrdinalDirection::SouthWest => bottom_right.set_x(0),
            OrdinalDirection::NorthWest => Coord::new(0, 0),
        };
        let candidate_coords = self
            .grid
            .enumerate()
            .filter_map(|(coord, &tile)| {
                if tile == Tile5::Floor {
                    if coord.manhattan_distance(corner_coord) < 10 {
                        for d in Direction::all() {
                            if self.grid.get(coord + d.coord()) == Some(&Tile5::Wall) {
                                return None;
                            }
                        }
                        return Some(coord);
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        if let Some(&stairs_coord) = candidate_coords.choose(rng) {
            *self.grid.get_checked_mut(stairs_coord) = tile;
            Some(stairs_coord)
        } else {
            None
        }
    }

    fn clear_around_tentactle(&mut self) {
        let mut border = Vec::new();
        for &coord in &self.tentacle_coords {
            for d in Direction::all() {
                let coord = coord + d.coord();
                if let Some(&tile) = self.grid.get(coord) {
                    match tile {
                        Tile5::Wall | Tile5::Door | Tile5::Debris => {
                            *self.grid.get_checked_mut(coord) = Tile5::Floor;
                            border.push(coord);
                        }
                        _ => (),
                    }
                }
            }
        }
        for coord in border {
            for d in Direction::all() {
                let coord = coord + d.coord();
                if let Some(&tile) = self.grid.get(coord) {
                    match tile {
                        Tile5::Wall | Tile5::Door | Tile5::Debris => {
                            *self.grid.get_checked_mut(coord) = Tile5::Floor;
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn add_debris_around_edge(&mut self) {
        for (_coord, tile) in self.grid.edge_enumerate_mut() {
            if tile.is_open() {
                *tile = Tile::Debris;
            }
        }
    }

    pub fn print(&self) {
        for row in self.grid.rows() {
            for cell in row {
                match cell {
                    Tile5::Street => print!("."),
                    Tile5::Alley => print!(","),
                    Tile5::Footpath => print!(","),
                    Tile5::Wall => print!("#"),
                    Tile5::Floor => print!("."),
                    Tile5::Debris => print!("%"),
                    Tile5::Door => print!("+"),
                    Tile5::Tentacle => print!("~"),
                    Tile5::StairsDown => print!(">"),
                    Tile5::StairsUp => print!("<"),
                }
            }
            println!("");
        }
    }

    fn is_path_between(&self, start: Coord, end: Coord) -> bool {
        if start == end {
            return true;
        }
        let mut seen = HashSet::new();
        seen.insert(start);
        let mut queue = VecDeque::new();
        queue.push_front(start);
        while let Some(coord) = queue.pop_back() {
            for d in CardinalDirection::all() {
                let coord = coord + d.coord();
                if let Some(tile) = self.grid.get(coord) {
                    if tile.is_open() {
                        if seen.insert(coord) {
                            if coord == end {
                                return true;
                            }
                            queue.push_front(coord);
                        }
                    }
                }
            }
        }
        false
    }

    pub fn generate<R: Rng>(tentacle_spec: &TentacleSpec, rng: &mut R) -> Self {
        loop {
            let map4 = Map4::generate(rng);
            let mut map5 = Self::from_map4(&map4);
            let mut corners = OrdinalDirection::all().collect::<Vec<_>>();
            corners.shuffle(rng);
            map5.add_tentacles(corners.pop().unwrap(), &tentacle_spec, rng);
            map5.clear_around_tentactle();
            let stairs_down =
                if let Some(x) = map5.add_stairs(corners.pop().unwrap(), Tile5::StairsDown, rng) {
                    x
                } else {
                    continue;
                };
            let stairs_up =
                if let Some(x) = map5.add_stairs(corners.pop().unwrap(), Tile5::StairsUp, rng) {
                    x
                } else {
                    continue;
                };
            if stairs_down == stairs_up {
                continue;
            }
            if !map5.is_path_between(stairs_down, stairs_up) {
                continue;
            }
            map5.add_debris_around_edge();
            break map5;
        }
    }
}

pub type Tile = Tile5;
pub type Map = Map5;
