use rand::Rng;

use std::fmt;
use std::io::{self, Write};

mod strategy;
use strategy::DotStrategy as _;
use strategy::PlacerStrategy as _;

const BOARD_W: usize = 9;
const BOARD_H: usize = 9;
const BOARD_SIZE: usize = BOARD_W * BOARD_H;

/// Represents a position on the board.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Pos(u8);

impl Pos {
    /// The position of the center of the board.
    const CENTER: Self = Self((BOARD_H/2 * BOARD_W + BOARD_W/2) as u8);

    /// Return a random position within distance 1 of the center.
    fn near_center<R: Rng>(rng: &mut R) -> Self {
        match rng.gen_range(0, 7) {
            i@0..=5 => Pos::CENTER.neighbors()[i].unwrap(),
            6 => Pos::CENTER,
            _ => unreachable!(),
        }
    }

    /// Return a random position anywhere in the game board.
    fn random<R: Rng>(mut rng: R) -> Self {
        Self(rng.gen_range(0, BOARD_SIZE as u8))
    }

    /// Construct a position from a pair of coordinates.
    fn from_xy(x: u8, y: u8) -> Self {
        debug_assert!(x < (BOARD_W as u8));
        debug_assert!(y < (BOARD_H as u8));
        Self(y * (BOARD_W as u8) + x)
    }

    /// Get the coordinates associated with this position.
    fn xy(self) -> [u8; 2] {
        let y = self.0 / BOARD_W as u8;
        let x = self.0 % BOARD_W as u8;
        [x, y]
    }

    /// Return the distance between this position and the
    /// edge of the board.
    fn dist_to_edge(self) -> u8 {
        use std::cmp::min;

        let [x, y] = self.xy();
        let dist_left  = x + 1;
        let dist_right = BOARD_W as u8 - x;
        let dist_up    = y + 1;
        let dist_down  = BOARD_H as u8 - y;

        min(
            min(dist_left, dist_right),
            min(dist_up, dist_down),
        )
    }

    /// Return an array of positions adjacent to this one,
    /// ordered first by y-coordinate and then by x-coordinate.
    /// Use `None` to represent positions that would be out
    /// of bounds.
    fn neighbors(self) -> [Option<Self>; 6] {
        let [x, y] = self.xy();

        let compute_neighbor = |dx, dy| {
            let nx = dx + x as i8;
            let ny = dy + y as i8;
            if (0..BOARD_W as i8).contains(&nx)
            && (0..BOARD_H as i8).contains(&ny) {
                Some(Self::from_xy(nx as u8, ny as u8))
            } else {
                None
            }
        };

        // Because the rows are staggered, odd rows
        // need to have the x-components of some base
        // coordinates incremented by 1.
        let y_parity = (y % 2) as i8;

        [
            compute_neighbor(-1 + y_parity, -1),
            compute_neighbor( 0 + y_parity, -1),
            compute_neighbor(-1,             0),
            compute_neighbor( 1,             0),
            compute_neighbor(-1 + y_parity,  1),
            compute_neighbor( 0 + y_parity,  1),
        ]
    }
}

impl fmt::Debug for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let [x, y] = self.xy();
        write!(f, "({}, {})", x, y)
    }
}

/// Represents the state of an active game. Ordered from
/// least to most significant, bits in locations 0 through
/// (BOARD_SIZE - 1) represent spaces on the board which are
/// filled in. Bits 120 through 127 represent the current
/// position of the dot (though this is technically only a
/// 7-bit value). All other bits should be set to zero.
#[derive(Clone, Copy, Default)]
pub struct State(u128);

// Ensure that this representation is possible.
sa::const_assert!(BOARD_SIZE <= 120 as usize);

impl State {
    /// Construct an empty board with the dot at the position
    /// provided.
    fn with_dot(pos: Pos) -> Self {
        let mut bytes = [0; 16];
        bytes[0] = pos.0;
        Self(u128::from_be_bytes(bytes))
    }

    /// Initialize a board with the dot near the middle
    /// and 8 random filled spaces.
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        let mut self_ = Self::with_dot(Pos::near_center(rng));
        for _ in 0..3 {
            let pos = Pos::random(&mut *rng);
            let _ = self_.fill(pos);
        }
        self_
    }

    /// Get the position of the dot.
    fn dot(self) -> Pos {
        Pos((self.0 >> 120) as u8)
    }

    /// Move the dot to a different position.
    fn set_dot(&mut self, pos: Pos) {
        self.0 &= !(0b11111111 << 120);
        self.0 |= (pos.0 as u128) << 120;
    }

    /// Fill a particular space on the board. Return `false`
    /// if the space could not be filled (either because it
    /// has already been filled, or because it is occupied
    /// by the dot).
    fn fill(&mut self, pos: Pos) -> bool {
        let can_fill = !(pos == self.dot() || self.has_filled(pos));
        self.0 |= (can_fill as u128) << pos.0;
        can_fill
    }

    /// Check if a particular space on the board is filled.
    fn has_filled(self, pos: Pos) -> bool {
        (self.0 >> pos.0) & 1 > 0
    }

    /// Return the number of steps that would be required for
    /// the dot to pathfind to the edge from its current position.
    fn dist_to_reach_edge(self) -> Option<u8> {
        // bit field representing the set of searched positions
        let mut searched = 1 << (self.dot().0 as u128);

        // stacks representing the frontiers
        let mut old_frontier = vec![];
        let mut new_frontier = vec![self.dot()];
        let mut steps = 0;

        loop {
            steps += 1;
            std::mem::swap(&mut new_frontier, &mut old_frontier);
            if old_frontier.is_empty() {
                return None;
            }
            for pos in old_frontier.drain(..) {
                for &n in &pos.neighbors() {
                    if let Some(neighbor) = n {
                        if searched & (1u128 << neighbor.0) == 0
                        && !self.has_filled(neighbor) {
                            searched |= 1u128 << neighbor.0;
                            new_frontier.push(neighbor);
                        }
                    } else {
                        return Some(steps)
                    }
                }
            }
        }
    }

    /// Check if the placer has won.
    fn placer_win(self) -> bool {
        self.dot().neighbors()
            .iter()
            .all(|&n| match n {
                None => false,
                Some(n) => self.has_filled(n),
            })
    }

    /// Return an array of new versions of the state in which
    /// the dot has moved in all possible directions. Return
    /// `Some(None)` for cases where the dot wins. Return `None`
    /// for cases where the dot cannot move due to being blocked.
    fn branch_dot(self) -> [Option<Option<Self>>; 6] {
        let neighbors = self.dot().neighbors();
        let compute_branch = |nidx: usize|
            match neighbors[nidx] {
                None => Some(None),
                Some(n) if self.has_filled(n) => None,
                Some(n) => {
                    let mut new = self;
                    new.set_dot(n);
                    Some(Some(new))
                }
            };

        [
            compute_branch(0),
            compute_branch(1),
            compute_branch(2),
            compute_branch(3),
            compute_branch(4),
            compute_branch(5),
        ]
    }

    /// Return an iterator over new versions of the state
    /// in which the placer has placed in all (relevantly
    /// distinct) possible positions. Return `None`
    /// for cases where the placer wins.
    fn branch_placer(self) -> impl Iterator<Item=Option<Self>> {
        let dot = self.dot();
        (0..BOARD_H as u8) // board y coordinates
            .flat_map(|y| (0..BOARD_W as u8) // product with x coordinates
                .map(move |x| Pos::from_xy(x, y))) // construct point
            .filter_map(move |p| {
                let mut new = self;
                if p == dot { // can't place over where the dot is
                    None
                } else if !new.fill(p) { // can't place if there already is one
                    None
                } else if new.placer_win() { // does this placement let us win?
                    Some(None)
                } else {
                    Some(Some(new))
                }
            })
    }

    /// Write a string representation of this board to `w`.
    pub fn display<W: Write>(self, mut w: W) -> io::Result<()> {
        let dot_pos = self.dot();
        writeln!(w, "{:=<1$}", "", BOARD_W * 2)?;
        for y in 0..BOARD_H {
            // offset space for odd rows
            if y % 2 > 0 {
                write!(w, " ")?;
            }
            // write row
            for x in 0..BOARD_W {
                let pos = Pos::from_xy(x as u8, y as u8);
                write!(w, "{}{}",
                    if pos == dot_pos { '@' }
                    else if self.has_filled(pos) { 'o' }
                    else { '·' },
                    if x + 1 == BOARD_W { '\n' }
                    else { ' ' },
                )?;
            }
        }
        Ok(())
    }
}

fn clear_screen() {
    print!("\x1b[H\x1b[2J");
}

pub fn main() -> io::Result<()> {
    use strategy as s;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut rng = rand::thread_rng();
    let mut state = State::new(&mut rng);

    let mut placer_strategy = s::PlacerPredictive::new(rng, s::SmartPathfind);
    let mut dot_strategy = s::SmartPathfind;

    loop {
        // perform placer actions
        std::thread::sleep(
            std::time::Duration::from_millis(200));
        clear_screen();
        state.display(&mut stdout)?;

        if let Some(s) = placer_strategy.play(state) {
            state = s;
        } else {
            stdout.write_all(b"placer\n")?;
            stdout.flush()?;
            break
        }

        // perform dot actions
        std::thread::sleep(
            std::time::Duration::from_millis(200));
        clear_screen();
        state.display(&mut stdout)?;

        if let Some(s) = dot_strategy.play(state) {
            state = s;
        } else {
            stdout.write_all(b"dot\n")?;
            stdout.flush()?;
            break;
        }
    }

    Ok(())
}
