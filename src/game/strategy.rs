use rand::Rng;

use super::State;

/// Represents a strategy that could be used by the dot.
pub trait DotStrategy {
    /// Given a non-empty list of potential states, return
    /// the index of the state which is most preferred.
    fn preferred_state(&mut self, choices: &[State]) -> usize;
}

/// A dumb strategy for the dot, causing it to move
/// towards whichever state gives it the smallest
/// distance to the edge.
pub struct DumbPathfind;

impl DotStrategy for DumbPathfind {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        (0..choices.len())
            .min_by_key(|&i| choices[i].dot().dist_to_edge())
            .unwrap()
    }
}

/// A slightly less dumb strategy for the dot. It knows
/// to take into account obstacles in its distance
/// calculation.
pub struct SmartPathfind;

impl DotStrategy for SmartPathfind {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        (0..choices.len())
            // if `dist_to_reach_edge` returns 1, the decision
            // is arbitrary as there's no way we can win anyway
            .min_by_key(|&i| choices[i].dist_to_reach_edge())
            .unwrap()
    }
}

/// Represents a strategy that could be used by the placer.
pub trait PlacerStrategy {
    /// Given a non-empty list of potential states, return
    /// the index of the state which is most preferred.
    fn preferred_state(&mut self, choices: &[State]) -> usize;
}

/// A strategy for the placer which is parameterized by
/// an assumption about what strategy the dot will use
/// next turn. It simply conducts a brute-force search
/// to look for the best move down to a given depth in
/// the game tree.
pub struct PlacerPredictive<R, S> {
    rng: R,
    dot_strategy: S,
}

impl<R, S> PlacerPredictive<R, S> {
    pub fn new(rng: R, dot_strategy: S) -> Self {
        Self { rng, dot_strategy }
    }
}

const SEARCH_DEPTH: u8 = 4;

impl<R: Rng, S: DotStrategy> PlacerStrategy for PlacerPredictive<R, S> {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        /// The best outcome of a given branch in the game tree.
        #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
        enum Outcome {
            /// Losing in a given number of turns.
            Lose(u8),
            /// Playing, resulting in the dot being a given
            /// distance away from the edge
            Play(u8),
            /// Winning in a given number of turns (this is
            /// represented as negative so that winning in
            /// less time is considered more of a success)
            Win(i8),
        }

        impl Outcome {
            /// Convert an outcome for this turn into
            /// an outcome for the next turn.
            fn inc(self) -> Self {
                match self {
                    Self::Lose(n) => Self::Lose(n + 1),
                    Self::Play(n) => Self::Play(n),
                    Self::Win(n) => Self::Win(n - 1),
                }
            }
        }

        /// Determine the best outcome reachable within `n` turns.
        /// Assume it is the dot's turn and that it will move
        /// according to `s`.
        fn search<S: DotStrategy>(
            state: State,
            dot_strategy: &mut S,
            n: u8
        ) -> Outcome {
            if n == 0 {
                Outcome::Play(state.dot().dist_to_edge())
            } else {
                // find all possible dot actions using a smallvec
                let mut dot_actions = [State::default(); 6];
                let mut dot_actions_len = 0;
                for &ns in &state.branch_dot() {
                    match ns {
                        None => {}
                        Some(None) => return Outcome::Lose(0),
                        Some(Some(new_state)) => {
                            dot_actions[dot_actions_len] = new_state;
                            dot_actions_len += 1;
                        }
                    }
                }

                // determine which action the dot will most prefer
                let dot_action_idx = dot_strategy.preferred_state(
                    &dot_actions[0..dot_actions_len]);
                let dot_state = dot_actions[dot_action_idx];

                // recursively determine: what the best way
                // to respond to this?
                dot_state.branch_placer()
                    .map(|ns| match ns {
                        None => Outcome::Win(0),
                        Some(new_state) => search(
                            new_state,
                            &mut *dot_strategy,
                            n - 1
                        ).inc()
                    })
                    .max()
                    .unwrap()
            }
        }

        (0..choices.len())
            // tiebreak using a random value to avoid always
            // choosing the last option
            .max_by_key(|&i| (search(
                choices[i],
                &mut self.dot_strategy,
                SEARCH_DEPTH
            ), self.rng.gen::<u8>()))
            .unwrap()
    }
}
