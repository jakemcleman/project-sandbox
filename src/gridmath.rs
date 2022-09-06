use std::ops;
use std::cmp;
use std::fmt;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct GridVec {
    pub x: i32,
    pub y: i32,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct GridBounds {
    pub bottom_left: GridVec,
    pub top_right: GridVec,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct ScreenPos {
    pub x: u32,
    pub y: u32,
}

impl GridVec {
    pub fn new(x:i32, y:i32) -> Self {
        GridVec {x, y}
    }

    pub fn manhattan_distance(&self, other: GridVec) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn is_adjacent(&self, other: GridVec) -> bool {
        match self.manhattan_distance(other) {
            1 => true,
            2 => (self.x - other.x).abs() == 1,
            _ => false
        }
    }

    /*
        Concatenate the bits of the 2 coordinates into a single 64 bit value
        Used for hashing and storage
    */
    pub fn combined(&self) -> u64 {
        self.x as u64 | (self.y as u64) << 32
    }

    /*
        Extract a grid coordinate from the bits of 2 coordinates combined into a 
        single 64 bit value
        Used for hashing and storage
    */
    pub fn decombined(combo: u64) -> GridVec {
        GridVec::new(
            (combo & 0x00000000FFFFFFFF) as i32, 
            ((combo & 0xFFFFFFFF00000000) >> 32) as i32)
    }
}

impl ops::Add<GridVec> for GridVec {
    type Output = GridVec;

    fn add(self, rhs: GridVec) -> GridVec {
        GridVec{x: self.x + rhs.x, y: self.y + rhs.y}
    }
}

impl ops::Sub<GridVec> for GridVec {
    type Output = GridVec;

    fn sub(self, rhs: GridVec) -> GridVec {
        GridVec{x: self.x - rhs.x, y: self.y - rhs.y}
    }
}

impl ops::Mul<i32> for GridVec {
    type Output = GridVec;

    fn mul(self, rhs: i32) -> GridVec {
        GridVec{x: self.x * rhs, y: self.y * rhs}
    }
}

impl ops::Div<i32> for GridVec {
    type Output = GridVec;

    fn div(self, rhs: i32) -> GridVec {
        GridVec{x: self.x / rhs, y: self.y / rhs}
    }
}

impl ops::Rem<i32> for GridVec {
    type Output = GridVec;

    fn rem(self, rhs: i32) -> Self::Output {
        GridVec{x: self.x % rhs, y: self.y / rhs }
    }
}

impl fmt::Display for GridVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

impl GridBounds {
    pub fn new(center: GridVec, half_extent: GridVec) -> Self {
        GridBounds { bottom_left: center - half_extent, top_right: center + half_extent }
    }

    pub fn new_from_corner(bottom_left: GridVec, size: GridVec) -> Self {
        GridBounds { bottom_left, top_right: bottom_left + size }
    }

    pub fn new_from_extents(bottom_left: GridVec, top_right: GridVec) -> Self {
        GridBounds { bottom_left, top_right }
    }

    pub fn bottom(&self) -> i32 {
        self.bottom_left.y
    }

    pub fn left(&self) -> i32 {
        self.bottom_left.x
    }

    pub fn top(&self) -> i32 {
        self.top_right.y
    }

    pub fn right(&self) -> i32 {
        self.top_right.x
    }

    pub fn bottom_left(&self) -> GridVec {
        self.bottom_left   
    }

    pub fn _bottom_right(&self) -> GridVec {
        GridVec { x: self.top_right.x, y: self.bottom_left.y }   
    }

    pub fn top_left(&self) -> GridVec {
        GridVec { x: self.bottom_left.x, y: self.top_right.y }      
    }

    pub fn top_right(&self) -> GridVec {
        self.top_right   
    }

    pub fn width(&self) -> u32 {
        (self.top_right.x - self.bottom_left.x) as u32
    }

    pub fn _height(&self) -> u32 {
        (self.top_right.y - self.bottom_left.y) as u32
    }

    pub fn center(&self) -> GridVec {
        (self.top_right + self.bottom_left) / 2
    }

    pub fn extent(&self) -> GridVec {
        self.top_right - self.bottom_left  
    }

    pub fn half_extent(&self) -> GridVec {
        self.extent() / 2
    }

    pub fn contains(&self, point: GridVec) -> bool {
        let delta = point - self.center();
        let half_extent = self.half_extent();
        return delta.x.abs() <= half_extent.x && delta.y.abs() <= half_extent.y;
    }

    pub fn is_boundary(&self, point: GridVec) -> bool {
        self.contains(point) 
        && (point.x == self.bottom_left().x 
            || point.x == self.top_right().x - 1
            || point.y == self.bottom_left().y
            || point.y == self.top_right().y - 1
        )
    }

    pub fn iter(&self) -> GridIterator {
        GridIterator { bounds: self.clone(), current: self.bottom_left() + GridVec::new(-1, 0) }
    }

    pub fn slide_iter(&self, flip_x: bool) -> SlideGridIterator {
        SlideGridIterator { 
            bounds: self.clone(), 
            current: if flip_x { self.top_right() + GridVec::new(-1, 0) } else { self.top_left() },
            flip_x,    
        }
    }

    // Returns a bounds that exactly contains both input bounds
    pub fn union(&self, other: GridBounds) -> GridBounds {
        let bottom_left = GridVec::new(
            cmp::min(self.bottom_left().x, other.bottom_left().x),
            cmp::min(self.bottom_left().y, other.bottom_left().y)
        );
        let top_right = GridVec::new(
            cmp::max(self.top_right().x, other.top_right().x),
            cmp::max(self.top_right().y, other.top_right().y)
        );
        GridBounds::new_from_extents(bottom_left, top_right)
    }

    pub fn option_union(a: Option<GridBounds>, b: Option<GridBounds>) -> Option<GridBounds> {
        if a.is_none() && b.is_none() { None }
        else if let Some(bound_a) = a {
            if let Some(bound_b) = b {
                Some(bound_a.union(bound_b))
            }
            else {
                a
            }
        }
        else {
            b
        }
    }

    pub fn intersect_option(&self, other: Option<GridBounds>) -> Option<GridBounds>  {
        if let Some(bounds) = other {
            self.intersect(bounds)
        }
        else {
            None
        }
    }

    // If there is an intersection, returns the bounds of the overlapping area
    pub fn intersect(&self, other: GridBounds) -> Option<GridBounds> {
        let dx = other.center().x - self.center().x;
        let px = (other.half_extent().x + self.half_extent().x) - dx.abs();
        if px <= 0 {
            return None;
        }

        let dy = other.center().y - self.center().y;
        let py = (other.half_extent().y + self.half_extent().y) - dy.abs();
        if py <= 0 {
            return None;
        }

        let bottom_left = GridVec::new(
            cmp::max(self.bottom_left().x, other.bottom_left().x),
            cmp::max(self.bottom_left().y, other.bottom_left().y)
        );
        let top_right = GridVec::new(
            cmp::min(self.top_right().x, other.top_right().x),
            cmp::min(self.top_right().y, other.top_right().y)
        );
        Some(GridBounds::new_from_extents(bottom_left, top_right))
    }
}

pub struct GridIterator {
    bounds: GridBounds,
    current: GridVec,
}

pub struct SlideGridIterator {
    bounds: GridBounds,
    current: GridVec,
    flip_x: bool,
}

impl Iterator for GridIterator {
    type Item = GridVec;

    fn next(&mut self) -> Option<GridVec> {
        self.current.x += 1;
        if self.current.x >= self.bounds.top_right().x {
            self.current.x = self.bounds.bottom_left().x;
            self.current.y += 1;

            if self.current.y >= self.bounds.top_right().y {
                return None
            }
        }

        return Some(self.current);
    }
}

impl Iterator for SlideGridIterator {
    type Item = GridVec;

    fn next(&mut self) -> Option<GridVec> {
        self.current.y -= 1;

        if self.current.y < self.bounds.bottom() {
            self.current.y = self.bounds.top() - 1;

            self.current.x += if self.flip_x { -1 } else { 1 };

            if (self.flip_x && self.current.x < self.bounds.left()) 
                || (!self.flip_x && self.current.x >= self.bounds.right()) {
                return None
            }
        }
        
        return Some(self.current);
    }
}

#[cfg(test)]
mod tests {
    use crate::gridmath::*;

    #[test]
    fn basic_addition() {
        let a = GridVec::new(1, 0);
        let b = GridVec::new(0, 2);
        let result = a + b;
        let expected = GridVec::new(1, 2);
        assert_eq!(result, expected);
    }

    #[test]
    fn basic_subtraction() {
        let a = GridVec::new(1, 0);
        let b = GridVec::new(0, 2);
        let result = a - b;
        let expected = GridVec::new(1, -2);
        assert_eq!(result, expected);
    }

    #[test]
    fn basic_multiplication() {
        let a = GridVec::new(1, 2);
        let result = a * 2;
        let expected = GridVec::new(2, 4);
        assert_eq!(result, expected);
    }

    #[test]
    fn basic_division() {
        let a = GridVec::new(1, 2);
        let result = a/ 2;
        let expected = GridVec::new(0, 1);
        assert_eq!(result, expected);
    }

    #[test]
    fn manhattan_distance() {
        let a = GridVec::new(1, 1);
        let b = GridVec::new(-1, 0);
        let result = a.manhattan_distance(b);
        let expected = 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn manhattan_distance_zero() {
        let a = GridVec::new(1, 1);
        let b = GridVec::new(1, 1);
        let result = a.manhattan_distance(b);
        let expected = 0;
        assert_eq!(result, expected);
    }

    #[test]
    fn adjacency_orthogonal() {
        let a = GridVec::new(0, 0);
        let b = GridVec::new(0, -1);
        let result = a.is_adjacent(b);
        let expected = true;
        assert_eq!(result, expected);
    }

    #[test]
    fn adjacency_diagonal() {
        let a = GridVec::new(0, 0);
        let b = GridVec::new(1, 1);
        let result = a.is_adjacent(b);
        let expected = true;
        assert_eq!(result, expected);
    }

    #[test]
    fn adjacency_miss() {
        let a = GridVec::new(0, 0);
        let b = GridVec::new(0, 2);
        let result = a.is_adjacent(b);
        let expected = false;
        assert_eq!(result, expected);
    }

    #[test]
    fn combination() {
        let result = GridVec::new(4, 10).combined();
        let expected = 0x0000000A00000004;
        assert_eq!(result, expected);
    }

    #[test]
    fn decombination() {
        let result = GridVec::decombined(0x0000000A00000004);
        let expected = GridVec::new(4, 10);
        assert_eq!(result, expected);
    }

    #[test]
    fn combination_decombination() {
        let result = GridVec::decombined(GridVec::new(4, 10).combined());
        let expected = GridVec::new(4, 10);
        assert_eq!(result, expected);
    }

    #[test]
    fn bounds_corners() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(1, 1));
        let bottom_left = a.bottom_left();
        let top_right = a.top_right();

        assert_eq!(bottom_left, GridVec::new(-1, -1));
        assert_eq!(top_right, GridVec::new(1, 1));
    }

    #[test]
    fn bounds_corners_from_corner() {
        let bottom_left = GridVec::new(0, 0);
        let size = GridVec::new(16, 16);
        let top_right = size;
        
        let a = GridBounds::new_from_corner(bottom_left, size);
        
        assert_eq!(a.bottom_left(), bottom_left);
        assert_eq!(a.top_right(), top_right);
    }

    #[test]
    fn intersection_overlap_none() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(1, 1));
        let b = GridBounds::new(GridVec::new(3, 0), GridVec::new(1, 1));

        let result = a.intersect(b);
        let expected = None;
        assert_eq!(result, expected);
    }

    #[test]
    fn intersection_overlap_contained() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(1, 1));
        let b = GridBounds::new(GridVec::new(0, 0), GridVec::new(10, 10));

        let result = a.intersect(b);
        let expected = Some(a);
        assert_eq!(result, expected);
    }

    #[test]
    fn intersection_overlap_partial() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(2, 2));
        let b = GridBounds::new(GridVec::new(2, 2), GridVec::new(2, 2));

        let result = a.intersect(b);
        let expected = Some(GridBounds::new(GridVec::new(1, 1), GridVec::new(1, 1)));
        assert_eq!(result, expected);
    }

    #[test]
    fn union_overlap_none() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(1, 1));
        let b = GridBounds::new(GridVec::new(4, 0), GridVec::new(1, 1));

        let result = a.union(b);
        let expected = GridBounds::new(GridVec::new(2, 0), GridVec::new(3, 1));
        assert_eq!(result, expected);
    }

    #[test]
    fn union_overlap_contained() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(1, 1));
        let b = GridBounds::new(GridVec::new(0, 0), GridVec::new(10, 10));

        let result = a.union(b);
        let expected = b;
        assert_eq!(result, expected);
    }

    #[test]
    fn union_overlap_partial() {
        let a = GridBounds::new(GridVec::new(0, 0), GridVec::new(4, 4));
        let b = GridBounds::new(GridVec::new(2, 2), GridVec::new(4, 4));

        let result = a.union(b);
        let expected = GridBounds::new(GridVec::new(1, 1), GridVec::new(5, 5));
        assert_eq!(result, expected);
    }
}