use gridbugs::{
    coord_2d::{Coord, Size},
    direction::{CardinalDirection, Direction},
    grid_2d::Grid,
    line_2d,
    perlin2::Perlin2,
};
use rand::{seq::SliceRandom, Rng};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
};
use vector::{Cartesian, Radial, Radians};

pub struct Spec {
    pub size: Size,
}

pub struct LandCell {
    pub height: f64,
}

pub struct Land {
    pub cells: Grid<LandCell>,
    pub height_diff: f64,
}

#[derive(Debug, PartialEq)]
struct SearchCell {
    cost: f64,
    heuristic: f64,
    coord: Coord,
}

impl PartialOrd for SearchCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.heuristic.partial_cmp(&self.heuristic)
    }
}

impl Eq for SearchCell {}

impl Ord for SearchCell {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.heuristic < other.heuristic {
            Ordering::Greater
        } else if self.heuristic > other.heuristic {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl Land {
    pub fn base_height_on_row(&self, row: usize) -> f64 {
        ((self.cells.width() as usize - row - 1) as f64 * self.height_diff)
            / self.cells.width() as f64
    }
    pub fn get_height(&self, coord: Coord) -> Option<f64> {
        self.cells
            .get(coord)
            .map(|land_cell| self.base_height_on_row(coord.x as usize) + land_cell.height)
    }
    pub fn plot_river(&self) -> Vec<Coord> {
        let mut best_cost = f64::MAX;
        let mut best = Vec::new();
        for i in 0..self.cells.height() as usize {
            let (path, cost) = self.plot_river_from_row(i);
            if cost < best_cost {
                best_cost = cost;
                best = path;
            }
        }
        best
    }
    pub fn plot_river_from_row(&self, start_row: usize) -> (Vec<Coord>, f64) {
        let start_coord = Coord::new(0, start_row as i32);
        let start_cost = self.get_height(start_coord).unwrap();
        let start = SearchCell {
            coord: start_coord,
            cost: start_cost,
            heuristic: self.cells.width() as f64 + start_cost,
        };
        let mut seen: HashMap<Coord, f64> = HashMap::new();
        seen.insert(start.coord, start.cost);
        let mut pq: BinaryHeap<SearchCell> = BinaryHeap::new();
        pq.push(start);
        let mut chain: HashMap<Coord, Coord> = HashMap::new();
        let mut end = None;
        while let Some(cell) = pq.pop() {
            if cell.coord.x as u32 == self.cells.width() - 1 {
                end = Some(cell);
                break;
            }
            for d in CardinalDirection::all() {
                let neighbour_coord = cell.coord + d.coord();
                if let Some(neighbour_height) = self.get_height(neighbour_coord) {
                    let mut neighbour_cost = cell.cost + neighbour_height.powf(2.);
                    let top_bottom_barrier_width = 5;
                    let top_bottom_barrier_cost = 1000.;
                    if neighbour_coord.y < top_bottom_barrier_width
                        || neighbour_coord.y
                            >= (self.cells.height() as i32 - top_bottom_barrier_width)
                    {
                        neighbour_cost += top_bottom_barrier_cost;
                    }
                    if let Some(best_existing_neighbour_cost) = seen.get(&neighbour_coord) {
                        if *best_existing_neighbour_cost <= neighbour_cost {
                            continue;
                        }
                    }
                    seen.insert(neighbour_coord, neighbour_cost);
                    pq.push(SearchCell {
                        coord: neighbour_coord,
                        cost: neighbour_cost,
                        heuristic: neighbour_cost
                            + (self.cells.width() as f64 - neighbour_coord.x as f64),
                    });
                    chain.insert(neighbour_coord, cell.coord);
                }
            }
        }
        let end = end.unwrap();
        let mut current = end.coord;
        let mut sequence = vec![current];
        while let Some(coord) = chain.remove(&current) {
            current = coord;
            sequence.push(coord);
        }
        sequence.reverse();
        (sequence, end.cost)
    }
}

fn validate_river(river: &[Coord]) -> bool {
    let n = 4;
    'outer: for w in river.windows((2 * n) + 1) {
        let delta_0 = w[1] - w[0];
        for i in 1..n {
            let delta = w[i + 1] - w[i];
            if delta != delta_0 {
                continue 'outer;
            }
        }
        let delta_n = w[n + 1] - w[n];
        for i in (n + 1)..(2 * n) {
            let delta = w[i + 1] - w[i];
            if delta != delta_n {
                continue 'outer;
            }
        }
        if delta_0 != delta_n {
            return false;
        }
    }
    true
}

pub fn land_and_river<R: Rng>(spec: &Spec, rng: &mut R) -> (Land, Vec<Coord>) {
    let perlin2 = Perlin2::new(rng);
    let zoom = 0.05;
    let height_weight = 1000.;
    let scale_coord = |coord: Coord| (coord.x as f64 * zoom, coord.y as f64 * zoom);
    let land_cells = Grid::new_fn(spec.size, |coord| LandCell {
        height: (perlin2.noise(scale_coord(coord)).abs() * height_weight) + 1.,
    });
    let land = Land {
        cells: land_cells,
        height_diff: 0.,
    };
    let river = land.plot_river();
    (land, river)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldCell1 {
    Land,
    Water,
}

fn world_grid1_from_river(size: Size, river: &[Coord]) -> Grid<WorldCell1> {
    let mut grid = Grid::new_copy(size, WorldCell1::Land);
    for &coord in river {
        *grid.get_checked_mut(coord) = WorldCell1::Water;
    }
    grid
}

fn world_grid1_widen_river(grid: &mut Grid<WorldCell1>) {
    let mut new_grid = grid.clone();
    for coord in grid.coord_iter() {
        for d in Direction::all() {
            let neighbour_coord = coord + d.coord();
            if let Some(&WorldCell1::Water) = grid.get(neighbour_coord) {
                *new_grid.get_checked_mut(coord) = WorldCell1::Water;
            }
        }
    }
    *grid = new_grid;
}

fn world_grid1_validate_no_loops(grid: &Grid<WorldCell1>) -> bool {
    let mut seen = HashSet::new();
    let mut region_counter = 0;
    for (coord, &cell) in grid.enumerate() {
        if let WorldCell1::Water = cell {
            continue;
        }
        if !seen.insert(coord) {
            continue;
        }
        let mut to_visit = VecDeque::new();
        to_visit.push_front(coord);
        while let Some(coord) = to_visit.pop_back() {
            for d in CardinalDirection::all() {
                let neighbour_coord = coord + d.coord();
                if let Some(&WorldCell1::Land) = grid.get(neighbour_coord) {
                    if seen.insert(neighbour_coord) {
                        to_visit.push_front(neighbour_coord);
                    }
                }
            }
        }
        region_counter += 1;
    }
    // the river should divide the land into 2 regions
    region_counter == 2
}

const TOWN_SIZE: Size = Size::new_u16(20, 20);

fn is_point_valid_for_river_town(grid: &Grid<WorldCell1>, coord: Coord) -> bool {
    if *grid.get_checked(coord) != WorldCell1::Water {
        return false;
    }
    let rect_grid = Grid::new_copy(TOWN_SIZE, ());
    let rect_top_left = coord - (rect_grid.size() / 2);
    let mut current = if let Some(cell) = grid.get(rect_top_left).cloned() {
        if let WorldCell1::Water = cell {
            return false;
        }
        WorldCell1::Land
    } else {
        return false;
    };
    let mut transition_count = 0;
    for relative_edge_coord in rect_grid.edge_coord_iter() {
        let coord = relative_edge_coord + rect_top_left;
        if let Some(&cell) = grid.get(coord) {
            if cell != current && current == WorldCell1::Land {
                transition_count += 1;
            }
            current = cell;
        } else {
            return false;
        }
    }
    // the rectangle intersects the river in two locations
    transition_count == 2
}

fn get_town_candidate_positions(grid: &Grid<WorldCell1>, river: &[Coord]) -> Vec<Vec<Coord>> {
    let town_position_range = 10;
    let town_indicies_approx = vec![river.len() / 4, (4 * river.len()) / 5];
    town_indicies_approx
        .into_iter()
        .map(|index_approx| {
            let mut candidates = Vec::new();
            for i in (index_approx - town_position_range)..(index_approx + town_position_range) {
                if let Some(&coord) = river.get(i) {
                    if is_point_valid_for_river_town(grid, coord) {
                        candidates.push(coord);
                    }
                }
            }
            candidates
        })
        .collect()
}

fn make_towns<R: Rng>(
    grid: &Grid<WorldCell1>,
    town_candidate_positions: &Vec<Vec<Coord>>,
    rng: &mut R,
) -> Grid<WorldCell1> {
    let mut output = grid.clone();
    for candidates in town_candidate_positions {
        let &centre = candidates.choose(rng).unwrap();
        let rect_grid = Grid::new_copy(TOWN_SIZE, ());
        let rect_top_left = centre - (rect_grid.size() / 2);
        for relative_coord in rect_grid.coord_iter() {
            let coord = relative_coord + rect_top_left;
            *output.get_checked_mut(coord) = WorldCell1::Water;
        }
    }
    output
}

fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    use std::f64::consts;
    fn left_most_point(points: &[(f64, f64)]) -> (f64, f64) {
        let mut left_most = None;
        let mut x_min = f64::MAX;
        for &(x, y) in points {
            if x < x_min {
                x_min = x;
                left_most = Some((x, y));
            }
        }
        left_most.unwrap()
    }
    fn normalize_angle(mut radians: f64) -> f64 {
        while radians > consts::PI {
            radians -= consts::PI * 2.;
        }
        while radians <= -consts::PI {
            radians += consts::PI * 2.;
        }
        radians
    }
    fn get_next_point(
        points: &[(f64, f64)],
        current: (f64, f64),
        prev_angle: f64,
    ) -> ((f64, f64), f64) {
        let mut max_angle = f64::MIN;
        let mut best_point = None;
        for &point in points {
            if point == current {
                continue;
            }
            let current_to_point = (point.0 - current.0, point.1 - current.1);
            let angle = normalize_angle(
                current_to_point.1.atan2(current_to_point.0) - prev_angle
                    + std::f64::consts::FRAC_PI_2,
            );
            if angle > max_angle {
                max_angle = angle;
                best_point = Some(point);
            }
        }
        (
            best_point.unwrap(),
            normalize_angle(max_angle + prev_angle - std::f64::consts::FRAC_PI_2),
        )
    }
    let start = left_most_point(points);
    let mut current = start;
    let mut hull = vec![current];
    let mut prev_angle = std::f64::consts::FRAC_PI_2;
    loop {
        let (next_point, angle) = get_next_point(points, current, prev_angle);
        prev_angle = angle;
        if next_point == start {
            break;
        }
        hull.push(next_point);
        current = next_point;
    }
    hull
}

pub struct Blob {
    pub border: Vec<Coord>,
    pub inside: Vec<Coord>,
}

fn blob<R: Rng>(size: Size, rng: &mut R) -> Blob {
    let n = 400;
    let radius = 0.5;
    let mut points = Vec::new();
    for _ in 0..n {
        let angle = Radians(rng.gen::<f64>() * (2.0 * std::f64::consts::PI));
        let length = rng.gen::<f64>() * radius;
        let vector = Radial { length, angle };
        let Cartesian { x, y } = vector.to_cartesian();
        points.push((
            (x + radius) * size.width() as f64,
            (y + radius) * size.height() as f64,
        ));
    }
    let ch = convex_hull(&points);
    let mut ch_coords = ch
        .into_iter()
        .map(|(x, y)| Coord::new(x as i32, y as i32))
        .collect::<Vec<_>>();
    ch_coords.push(ch_coords[0]);
    let border_set = ch_coords
        .windows(2)
        .flat_map(|w| line_2d::coords_between_cardinal(w[0], w[1]))
        .collect::<HashSet<_>>();
    let border = border_set.iter().cloned().collect::<Vec<_>>();
    let centre = size.to_coord().unwrap() / 2;
    let mut to_visit = VecDeque::new();
    to_visit.push_front(centre);
    let mut seen = HashSet::new();
    seen.insert(centre);
    while let Some(coord) = to_visit.pop_back() {
        for d in CardinalDirection::all() {
            let neighbour_coord = coord + d.coord();
            if !border_set.contains(&neighbour_coord) {
                if seen.insert(neighbour_coord) {
                    to_visit.push_front(neighbour_coord);
                }
            }
        }
    }
    let inside = seen.into_iter().collect::<Vec<_>>();
    Blob { border, inside }
}

pub struct Terrain {
    pub land: Land,
    pub river: Vec<Coord>,
    pub world1: Grid<WorldCell1>,
    pub blob: Blob,
}

pub fn generate<R: Rng>(spec: &Spec, rng: &mut R) -> Terrain {
    loop {
        let (land, river) = loop {
            let (land, river) = land_and_river(spec, rng);
            if validate_river(&river) {
                break (land, river);
            }
        };
        let mut world1 = world_grid1_from_river(spec.size, &river);
        world_grid1_widen_river(&mut world1);
        world_grid1_widen_river(&mut world1);
        let town_candidate_positions = get_town_candidate_positions(&world1, &river);
        if town_candidate_positions.iter().any(|v| v.is_empty()) {
            continue;
        }
        let world1 = make_towns(&world1, &town_candidate_positions, rng);
        if !world_grid1_validate_no_loops(&world1) {
            continue;
        }
        break Terrain {
            land,
            river,
            world1,
            blob: blob(spec.size, rng),
        };
    }
}

#[test]
fn convex_hull_test() {
    use std::collections::BTreeSet;
    let points = vec![
        (0., 0.),
        (1., 1.),
        (1., -1.),
        (-1., 1.),
        (-1., -1.),
        (0.5, 0.5),
    ];
    let ch = convex_hull(&points);
    panic!("{:?}", ch);
}
