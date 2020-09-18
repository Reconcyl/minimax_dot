use super::State;

/// The number of recursion levels used by strategies
/// to search when using `approx_utility_dot`.
const UTILITY_APPROX_LVL: u8 = 5;

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

/// A smart strategy for the dot, causing it to move
/// towards whichever state gives it the best utility
/// as decided by `approx_utility_dot`.
pub struct DotUtilityMax;

impl DotStrategy for DotUtilityMax {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        (0..choices.len())
            .min_by_key(|&i| choices[i].approx_utility_dot(UTILITY_APPROX_LVL))
            .unwrap()
    }
}

/// Represents a strategy that could be used by the placer.
pub trait PlacerStrategy {
    /// Given a non-empty list of potential states, return
    /// the index of the state which is most preferred.
    fn preferred_state(&mut self, choices: &[State]) -> usize;
}

/// A smart strategy for the placer, causing it to move
/// towards whichever state gives it the best utility
/// as decided by `approx_utility_placer`.
pub struct PlacerUtilityMax;

impl PlacerStrategy for PlacerUtilityMax {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        (0..choices.len())
            .min_by_key(|&i| choices[i].approx_utility_placer(UTILITY_APPROX_LVL))
            .unwrap()
    }
}

/// A strategy for the placer which is parameterized by
/// an assumption about what strategy the dot will use
/// next turn. It moves towards whichever state gives the
/// dot the lowest utility after its turn.
pub struct PlacerPredictive<S> {
    dot_strategy: S,
}

impl<S: DotStrategy + Send> PlacerStrategy for PlacerPredictive<S> {
    fn preferred_state(&mut self, choices: &[State]) -> usize {
        (0..choices.len())
            .min_by_key(|&i| {
                let choice = &choices[i];

                // find all possible reactions
                // (emulate a smallvec)
                let mut dot_reactions = [State::default(); 6];
                let mut dot_reactions_len = 0;
                for &i in &choice.branch_dot() {
                    match i {
                        None => {}

                        // TODO: ensure this is treated as +inf
                        Some(None) => return 100,

                        Some(Some(new_state)) => {
                            let new_state = new_state;
                            dot_reactions[dot_reactions_len] = new_state;
                            dot_reactions_len += 1;
                        }
                    }
                }

                // determine which reaction the dot most prefers
                let dot_reaction_idx = self.dot_strategy
                    .preferred_state(&dot_reactions[0..dot_reactions_len]);
                let dot_reaction = dot_reactions[dot_reaction_idx];

                // what is the dot's utility for this reaction?
                dot_reaction.approx_utility_dot(UTILITY_APPROX_LVL)
            })
            .unwrap()
    }
}
