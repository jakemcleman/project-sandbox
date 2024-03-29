use std::fmt;

use crate::{gridvec::*, GridBounds};

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct GridLine {
    pub a: GridVec,
    pub b: GridVec,
}

pub struct GridLineIterator {
    end: GridVec,
    current: GridVec,
    done: bool,
}

impl GridLine {
    pub fn new(a: GridVec, b: GridVec) -> Self {
        Self {a, b}
    }

    pub fn sq_length(&self) -> i32 {
        (self.a.x - self.b.x).pow(2) + (self.a.y - self.b.y).pow(2)
    }

    pub fn manhattan_length(&self) -> i32 {
        (self.a.x - self.b.x).abs() + (self.a.y - self.b.y).abs()
    }

    pub fn along(&self) -> GridLineIterator {
        GridLineIterator {
            current: self.a,
            end: self.b,
            done: false,
        }
    }

    pub fn reversed(&self) -> GridLine {
        GridLine::new(self.b, self.a)
    }

    pub fn get_bounds(&self) -> GridBounds {
        GridBounds::containing(&vec![self.a, self.b])
    }

    pub fn intersect(&self, other: &GridLine) -> Option<GridVec> {
        //print!("intersecting {0} with {1}... ", self, other);
        let x1 = self.a.x;
        let y1 = self.a.y;

        let x2 = self.b.x;
        let y2 = self.b.y;

        let x3 = other.a.x;
        let y3 = other.a.y;

        let x4 = other.b.x;
        let y4 = other.b.y;

        // Calculate the intersection t
        // leaving as ratio until final step because integer
        let t_num = ((x1 - x3) * (y3 - y4)) - ((y1 - y3) * (x3 - x4));
        let t_den = ((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4));
        //print!("t = ({0}/{1})... ", t_num, t_den);

        let u_num = ((x1 - x2) * (y1 - y3)) - ((y1 - y2) * (x1 - x3));
        let u_den = ((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4));
        //print!("u = ({0}/{1})... ", u_num, u_den);

        // If t and u are both in [0, 1], there is an intersection
        // Check for < 0 by making sure the signs match
        if t_den == 0 || u_den == 0 {
            //println!("Big 0 - no intersection");
            return None
        }
        if (t_num != 0 && (t_num.signum() != t_den.signum())) || (u_num != 0 && (u_num.signum() == u_den.signum())) {
            //println!("Negative value - no intersection");
            return None
        }
        // Check for > 1 by making sure the numerator is not > denominator
        if t_num.abs() > t_den.abs() || u_num.abs() > u_den.abs() {
            //println!(">1 - no intersection");
            return None
        }

        // There is an intersection
        let i_x = x1 + ((t_num * (x2 - x1)) / t_den);
        let i_y = y1 + ((t_num * (y2 - y1)) / t_den);

        //println!("Intersect! At ({0}, {1})", i_x, i_y);

        Some(GridVec::new(i_x, i_y))
    }
}

impl fmt::Display for GridLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "|{0} to {1}|", self.a, self.b)
    }
}

impl Iterator for GridLineIterator {
    type Item = GridVec;

    fn next(&mut self) -> Option<GridVec> {
        if self.done {
            None
        }
        else if self.current == self.end {
            self.done = true;
            Some(self.end)
        }
        else {
            let last = self.current;
            let move_vec = self.end - self.current;
            if move_vec.x == 0 || move_vec.y == 0 {
                // Alligned on one axis, move along it
                self.current.x += move_vec.x.signum();
                self.current.y += move_vec.y.signum();
            }
            else {
                // Decide which movement option gets closer
                let x_move = self.current + GridVec::new(move_vec.x.signum(), 0);
                let y_move = self.current + GridVec::new(0, move_vec.y.signum());
            
                if self.end.sq_distance(x_move) < self.end.sq_distance(y_move) {
                    self.current = x_move;
                }
                else {
                    self.current = y_move;
                }
            }
            Some(last)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gridline::*;

    #[test]
    fn axis_intersect() {
        let x = GridLine::new(GridVec::new(-10, 0), GridVec::new(10, 0));
        let y = GridLine::new(GridVec::new(0, -10), GridVec::new(0, 10));
        let origin = GridVec::new(0, 0);
        assert_eq!(x.intersect(&y), Some(origin));
        assert_eq!(y.intersect(&x), Some(origin));
    }

    #[test]
    fn orthogonal_no_intersect() {
        // y axis
        let y = GridLine::new(GridVec::new(0, -10), GridVec::new(0, 10));
        let x = GridLine::new(GridVec::new(5, 0), GridVec::new(15, 0));
        
        assert_eq!(x.intersect(&y), None);
        assert_eq!(y.intersect(&x), None);
    }

    #[test]
    fn offset_orthogonal_no_intersect() {
        // y axis
        let y = GridLine::new(GridVec::new(0, 0), GridVec::new(0, 10));
        let x = GridLine::new(GridVec::new(5, 5), GridVec::new(15, 5));
        
        assert_eq!(x.intersect(&y), None);
        assert_eq!(y.intersect(&x), None);
    }

    #[test]
    fn diagonal_intersect() {
        let a = GridLine::new(GridVec::new(-10, -10), GridVec::new(10, 10));
        let b = GridLine::new(GridVec::new(-10, 10), GridVec::new(10, -10));
        let origin = GridVec::new(0, 0);
        assert_eq!(a.intersect(&b), Some(origin));
        assert_eq!(b.intersect(&a), Some(origin));
    }

    #[test]
    fn diagonal_intersect_offset() {
        let a = GridLine::new(GridVec::new(0, -10), GridVec::new(10, 10));
        let b = GridLine::new(GridVec::new(0, 10), GridVec::new(10, -10));
        let intersect = GridVec::new(5, 0);
        assert_eq!(a.intersect(&b), Some(intersect));
        assert_eq!(b.intersect(&a), Some(intersect));
    }

    #[test]
    fn intersect_at_endpoint() {
        let x = GridLine::new(GridVec::new(-10, 0), GridVec::new(10, 0));
        let y1 = GridLine::new(GridVec::new(0, 0), GridVec::new(0, 10));
        let y2 = GridLine::new(GridVec::new(0, 10), GridVec::new(0, 0));
        let origin = GridVec::new(0, 0);
        assert_eq!(x.intersect(&y1), Some(origin));
        assert_eq!(y1.intersect(&x), Some(origin));
        assert_eq!(x.intersect(&y2), Some(origin));
        assert_eq!(y2.intersect(&x), Some(origin));
    }

    #[test]
    fn zero_length_along_iter() {
        let origin = GridVec::new(0, 0);
        let a = GridLine::new(origin, origin);

        let mut seen = 0;
        for point in a.along() {
            seen += 1;
            assert!(point == origin);
        }

        assert_eq!(seen, 1);
    }

    #[test]
    fn length_along_axis_iter() {
        let origin = GridVec::new(0, 0);
        let end = GridVec::new(10, 0);
        let a = GridLine::new(origin, end);

        let mut seen = 0;
        for point in a.along() {
            assert_eq!(point, GridVec::new(seen, 0));
            seen += 1;
        }

        assert_eq!(seen, 11);
    }

    #[test]
    fn length_along_diagonal_iter() {
        let origin = GridVec::new(0, 0);
        let end = GridVec::new(5, 5);
        let a = GridLine::new(origin, end);

        let mut seen = 0;
        for point in a.along() {
            seen += 1;
        }

        assert_eq!(seen, 11);
    }
}