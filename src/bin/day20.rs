use aoc2020::aoc_input::get_input;
use aoc2020::coordinates::Coord;
use aoc2020::grid::{Axis, Grid, Rotation};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct Tile {
    id: usize,
    top: String,
    left: String,
    right: String,
    bottom: String,
    interior: Grid<char>,
}

impl Tile {
    fn flip_inplace(&mut self, axis: Axis) {
        self.interior.flip_inplace(axis);
        match axis {
            Axis::Horizontal => {
                std::mem::swap(&mut self.top, &mut self.bottom);
                self.left = self.left.chars().rev().collect();
                self.right = self.right.chars().rev().collect();
            }
            Axis::Vertical => {
                std::mem::swap(&mut self.left, &mut self.right);
                self.top = self.top.chars().rev().collect();
                self.bottom = self.bottom.chars().rev().collect();
            }
        }
    }

    fn rotate_clockwise_inplace(&mut self) {
        self.interior.rotate_clockwise_inplace();
        std::mem::swap(&mut self.top, &mut self.right);
        std::mem::swap(&mut self.left, &mut self.top);
        std::mem::swap(&mut self.bottom, &mut self.left);
        self.top = self.top.chars().rev().collect();
        self.bottom = self.bottom.chars().rev().collect();
    }

    fn rotate_inplace(&mut self, rotation: Rotation) {
        for _ in 0..rotation.to_cw_count() {
            self.rotate_clockwise_inplace();
        }
    }

    fn faces<'a>(&'a self) -> [&'a String; 4] {
        return [&self.top, &self.right, &self.bottom, &self.left];
    }
}

fn parse_tile(lines: &[&str]) -> Tile {
    let right = lines[0].split(' ').nth(1).unwrap();
    let right = &right[..right.find(':').unwrap()];
    let id: usize = right.parse().unwrap();
    let grid_lines = &lines[1..];
    let full_grid: Grid<char> = grid_lines.join("\n").parse().unwrap();

    let w = full_grid.width() as isize;
    let h = full_grid.height() as isize;

    let mut top = String::with_capacity(w as usize);
    let mut bottom = String::with_capacity(w as usize);
    for x in 0..w {
        top.push(*full_grid.get(Coord(x, 0)).unwrap());
        bottom.push(*full_grid.get(Coord(x, h - 1)).unwrap());
    }

    let mut left = String::with_capacity(h as usize);
    let mut right = String::with_capacity(h as usize);
    for y in 0..h {
        left.push(*full_grid.get(Coord(0, y)).unwrap());
        right.push(*full_grid.get(Coord(w - 1, y)).unwrap());
    }

    let interior_lines: Vec<_> = grid_lines[1..grid_lines.len() - 1]
        .iter()
        .copied()
        .map(|line| &line[1..line.len() - 1])
        .collect();
    let interior: Grid<char> = interior_lines.join("\n").parse().unwrap();

    Tile {
        id,
        top,
        bottom,
        left,
        right,
        interior,
    }
}

struct ArrangeCtx {
    tile_map: HashMap<usize, Tile>,
    face_map: HashMap<String, Vec<usize>>,
    tile_locs: HashMap<Coord, usize>,
    tile_locs_rev: HashMap<usize, Coord>,
    tile_locs_xmax: isize,
    tile_locs_ymax: isize,
}

#[derive(Debug, Clone)]
enum Constraint {
    Boundary,
    FaceEqual(String),
}

#[derive(Debug, Clone)]
struct Constraints {
    left: Constraint,
    top: Constraint,
}

impl ArrangeCtx {
    fn new(tiles: Vec<Tile>) -> Self {
        let mut tile_map = HashMap::new();
        for tile in tiles {
            tile_map.insert(tile.id, tile);
        }

        let mut face_map = HashMap::<String, Vec<usize>>::new();
        for tile in tile_map.values() {
            for f in tile.faces().iter().copied() {
                let rev = f.chars().rev().collect::<String>();
                face_map.entry(f.clone()).or_default().push(tile.id);
                face_map.entry(rev).or_default().push(tile.id);
            }
        }

        ArrangeCtx {
            tile_map,
            face_map,
            tile_locs: HashMap::new(),
            tile_locs_rev: HashMap::new(),
            tile_locs_xmax: 0,
            tile_locs_ymax: 0,
        }
    }

    fn is_boundary(&self, face: &String) -> bool {
        let ids = self.face_map.get(face).unwrap();
        ids.len() == 1
    }

    fn find_corner(&self) -> usize {
        let mut tile_edge_face_counts = HashMap::<usize, usize>::new();
        for v in self.face_map.values() {
            match v.len() {
                1 => *tile_edge_face_counts.entry(v[0]).or_default() += 1,
                2 => continue,
                _ => unreachable!(),
            }
        }

        for (id, noadj_count) in tile_edge_face_counts {
            match noadj_count / 2 {
                0 | 1 => continue,
                2 => return id,
                _ => unreachable!(),
            }
        }

        unreachable!()
    }

    fn set_tile_loc(&mut self, id: usize, coord: Coord) {
        self.tile_locs.insert(coord, id);
        self.tile_locs_rev.insert(id, coord);
        self.tile_locs_xmax = self.tile_locs_xmax.max(coord.0);
        self.tile_locs_ymax = self.tile_locs_ymax.max(coord.1);
    }

    fn align_corner(&mut self, id: usize) {
        let tile = self.tile_map.get(&id).unwrap();
        let top_boundary = self.is_boundary(&tile.top);
        let left_boundary = self.is_boundary(&tile.left);
        let rotation = match (top_boundary, left_boundary) {
            (true, true) => Rotation::Cw0,
            (false, true) => Rotation::Cw90,
            (false, false) => Rotation::Cw180,
            (true, false) => Rotation::Cw270,
        };
        drop(tile);
        self.tile_map.get_mut(&id).unwrap().rotate_inplace(rotation);
        self.set_tile_loc(id, Coord::origin());
    }

    fn next_bfs_layer(layer: HashSet<Coord>) -> HashSet<Coord> {
        let mut next_layer = HashSet::new();
        for c in layer {
            next_layer.insert(Coord(c.0 + 1, c.1));
            next_layer.insert(Coord(c.0, c.1 + 1));
        }
        next_layer
    }

    fn has_boundary(&self, id: usize) -> bool {
        let tile = self.tile_map.get(&id).unwrap();
        let faces = &[&tile.left, &tile.top, &tile.right, &tile.bottom];
        faces.iter().any(|f| self.is_boundary(*f))
    }

    fn find_tile_for_loc(&mut self, c: Coord) -> Option<(usize, Constraints)> {
        let top = match self.tile_locs.get(&Coord(c.0, c.1 - 1)) {
            None => Constraint::Boundary,
            Some(tile_id) => {
                let tile = self.tile_map.get(tile_id).unwrap();
                Constraint::FaceEqual(tile.bottom.clone())
            }
        };

        let left = match self.tile_locs.get(&Coord(c.0 - 1, c.1)) {
            None => Constraint::Boundary,
            Some(tile_id) => {
                let tile = self.tile_map.get(tile_id).unwrap();
                Constraint::FaceEqual(tile.right.clone())
            }
        };

        let constraints = Constraints { left, top };
        let mut candidates: HashSet<usize> = self
            .tile_map
            .keys()
            .copied()
            .filter(|k| !self.tile_locs_rev.contains_key(k))
            .collect();
        for c in &[&constraints.left, &constraints.top] {
            match c {
                Constraint::Boundary => {
                    candidates.retain(|c| self.has_boundary(*c));
                }
                Constraint::FaceEqual(s) => {
                    let cur_candidates = self.face_map.get(s).unwrap();
                    candidates.retain(|c| cur_candidates.contains(c));
                }
            }
        }

        match candidates.len() {
            0 => None,
            1 => Some((*candidates.iter().next().unwrap(), constraints)),
            _ => panic!("Could not find unique tile for {:?}", c),
        }
    }

    fn check_constraints(&self, id: usize, c: &Constraints) -> bool {
        let tile = self.tile_map.get(&id).unwrap();

        let left_res = match &c.left {
            Constraint::Boundary => self.is_boundary(&tile.left),
            Constraint::FaceEqual(s) => s == &tile.left,
        };

        let top_res = match &c.top {
            Constraint::Boundary => self.is_boundary(&tile.top),
            Constraint::FaceEqual(s) => s == &tile.top,
        };

        left_res && top_res
    }

    fn solve_tile_in_loc(&mut self, c: Coord) -> bool {
        let (id, constraints) = match self.find_tile_for_loc(c) {
            None => return false,
            Some(t) => t,
        };

        let mut count = 0;
        while !self.check_constraints(id, &constraints) {
            assert_ne!(count, 8);
            let tile = self.tile_map.get_mut(&id).unwrap();

            tile.rotate_clockwise_inplace();
            count += 1;

            if count == 4 {
                tile.flip_inplace(Axis::Horizontal);
            }
        }

        self.set_tile_loc(id, c);
        true
    }

    fn solve_puzzle(&mut self) {
        let corner_id = self.find_corner();
        self.align_corner(corner_id);

        let mut bfs_layer = HashSet::new();
        bfs_layer.insert(Coord::origin());

        while self.tile_locs.len() < self.tile_map.len() {
            assert_ne!(bfs_layer.len(), 0);
            bfs_layer = Self::next_bfs_layer(bfs_layer);
            let mut removals = Vec::new();

            for c in bfs_layer.iter().copied() {
                if !self.solve_tile_in_loc(c) {
                    removals.push(c);
                }
            }

            for r in removals {
                bfs_layer.remove(&r);
            }
        }
    }

    fn corner_product(&self) -> usize {
        let left = 0isize;
        let right = self.tile_locs_xmax;
        let top = 0isize;
        let bottom = self.tile_locs_ymax;
        let corners = [
            Coord(left, top),
            Coord(right, top),
            Coord(left, bottom),
            Coord(right, bottom),
        ];

        corners
            .iter()
            .map(|c| self.tile_locs.get(c).unwrap())
            .product()
    }
}

fn main() {
    let input = get_input(20);
    let lines: Vec<_> = input[..input.len() - 1].lines().collect();
    let groups: Vec<_> = lines.split(|line| line.is_empty()).collect();

    let tiles: Vec<_> = groups.iter().copied().map(parse_tile).collect();
    let mut ctx = ArrangeCtx::new(tiles);
    ctx.solve_puzzle();
    dbg!(ctx.corner_product());
}
