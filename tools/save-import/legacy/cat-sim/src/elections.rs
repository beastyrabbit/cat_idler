//! Election and vote-kick rules ported from `lib/game/elections.ts`.

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet},
};

use serde::{Deserialize, Serialize};

/// Distinct players required to kick a leader.
pub const KICK_THRESHOLD: usize = 5;
/// How many cats stand for election.
pub const CANDIDATE_COUNT: usize = 5;
/// Leader term length in real milliseconds.
pub const TERM_MS: f64 = 24.0 * 3600.0 * 1000.0;
/// How long the polls stay open in real milliseconds.
pub const ELECTION_WINDOW_MS: f64 = 30.0 * 60.0 * 1000.0;
/// How long a vote-kick petition stays open in real milliseconds.
pub const KICK_WINDOW_MS: f64 = 10.0 * 60.0 * 1000.0;

/// Minimal cat shape used to choose election candidates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectionCandidate {
    #[serde(rename = "_id", alias = "id")]
    pub id: String,
    pub leadership: f64,
}

/// A player's ballot position for a cat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BallotVote {
    pub player_id: String,
    pub cat_id: String,
}

/// Vote counts per cat id.
pub type VoteTally = HashMap<String, u32>;

/// Set-like inputs accepted by [`candidates_for`].
pub trait BarredIds {
    fn contains_id(&self, id: &str) -> bool;
}

impl BarredIds for () {
    fn contains_id(&self, _id: &str) -> bool {
        false
    }
}

impl BarredIds for HashSet<String> {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(id)
    }
}

impl BarredIds for &HashSet<String> {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(id)
    }
}

impl BarredIds for BTreeSet<String> {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(id)
    }
}

impl BarredIds for &BTreeSet<String> {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(id)
    }
}

impl BarredIds for &[String] {
    fn contains_id(&self, id: &str) -> bool {
        self.iter().any(|barred| barred == id)
    }
}

impl<const N: usize> BarredIds for &[String; N] {
    fn contains_id(&self, id: &str) -> bool {
        self.iter().any(|barred| barred == id)
    }
}

impl BarredIds for &[&str] {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(&id)
    }
}

impl<const N: usize> BarredIds for &[&str; N] {
    fn contains_id(&self, id: &str) -> bool {
        self.contains(&id)
    }
}

impl<T> BarredIds for Option<T>
where
    T: BarredIds,
{
    fn contains_id(&self, id: &str) -> bool {
        self.as_ref().is_some_and(|barred| barred.contains_id(id))
    }
}

/// Numeric vote count accepted by [`election_winner`].
pub trait VoteCount {
    fn as_f64(&self) -> f64;
}

impl VoteCount for u32 {
    fn as_f64(&self) -> f64 {
        f64::from(*self)
    }
}

impl VoteCount for usize {
    fn as_f64(&self) -> f64 {
        *self as f64
    }
}

impl VoteCount for i32 {
    fn as_f64(&self) -> f64 {
        f64::from(*self)
    }
}

impl VoteCount for i64 {
    fn as_f64(&self) -> f64 {
        *self as f64
    }
}

impl VoteCount for f64 {
    fn as_f64(&self) -> f64 {
        *self
    }
}

/// Top candidates by leadership, excluding barred cats.
#[must_use]
pub fn candidates_for<B>(cats: &[ElectionCandidate], barred: B) -> Vec<String>
where
    B: BarredIds,
{
    let mut candidates = cats
        .iter()
        .filter(|cat| !barred.contains_id(&cat.id))
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| compare_leadership_desc(a.leadership, b.leadership));
    candidates.truncate(CANDIDATE_COUNT);
    candidates.iter().map(|cat| cat.id.clone()).collect()
}

/// Top candidates by leadership with no barred cats.
#[must_use]
pub fn candidates_for_unbarred(cats: &[ElectionCandidate]) -> Vec<String> {
    candidates_for(cats, ())
}

/// One position per player: their latest vote wins, counted per cat.
#[must_use]
pub fn tally_votes(votes: &[BallotVote]) -> VoteTally {
    let mut latest = HashMap::new();
    for vote in votes {
        latest.insert(vote.player_id.clone(), vote.cat_id.clone());
    }

    let mut tally = HashMap::new();
    for cat_id in latest.values() {
        *tally.entry(cat_id.clone()).or_insert(0) += 1;
    }
    tally
}

/// Winner = most votes among candidates; ties break toward higher leadership.
///
/// Zero votes fall back to the most leaderly candidate in the candidate list.
#[must_use]
pub fn election_winner<V>(
    candidates: &[ElectionCandidate],
    tally: &HashMap<String, V>,
) -> Option<String>
where
    V: VoteCount,
{
    let mut winner = candidates.first()?;
    let mut winner_votes = tally.get(&winner.id).map_or(0.0, VoteCount::as_f64);

    for candidate in &candidates[1..] {
        let votes = tally.get(&candidate.id).map_or(0.0, VoteCount::as_f64);
        if votes > winner_votes
            || (votes == winner_votes && candidate.leadership > winner.leadership)
        {
            winner = candidate;
            winner_votes = votes;
        }
    }

    Some(winner.id.clone())
}

/// A kick succeeds only with enough distinct players behind it.
#[must_use]
pub fn should_trigger_kick(votes: &[BallotVote]) -> bool {
    let distinct_players = votes
        .iter()
        .map(|vote| vote.player_id.as_str())
        .collect::<HashSet<_>>();

    distinct_players.len() >= KICK_THRESHOLD
}

/// A new election is due when none has happened or the term expired.
#[must_use]
pub fn election_due(last_resolved_at: Option<f64>, now: f64, term_ms: f64) -> bool {
    match last_resolved_at {
        None => true,
        Some(last_resolved_at) => now - last_resolved_at >= term_ms,
    }
}

fn compare_leadership_desc(a: f64, b: f64) -> Ordering {
    let delta = b - a;
    if delta.is_nan() || delta == 0.0 {
        Ordering::Equal
    } else if delta < 0.0 {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::{
        BallotVote, CANDIDATE_COUNT, ELECTION_WINDOW_MS, ElectionCandidate, KICK_THRESHOLD,
        KICK_WINDOW_MS, TERM_MS, candidates_for, candidates_for_unbarred, election_due,
        election_winner, should_trigger_kick, tally_votes,
    };

    fn cat(id: &str, leadership: f64) -> ElectionCandidate {
        ElectionCandidate {
            id: id.to_owned(),
            leadership,
        }
    }

    fn vote(player_id: &str, cat_id: &str) -> BallotVote {
        BallotVote {
            player_id: player_id.to_owned(),
            cat_id: cat_id.to_owned(),
        }
    }

    fn tally<const N: usize>(entries: [(&str, u32); N]) -> HashMap<String, u32> {
        entries
            .into_iter()
            .map(|(cat_id, count)| (cat_id.to_owned(), count))
            .collect()
    }

    #[test]
    fn constants_match_elections_ts() {
        assert_eq!(KICK_THRESHOLD, 5);
        assert_eq!(CANDIDATE_COUNT, 5);
        assert_eq!(TERM_MS, 86_400_000.0);
        assert_eq!(ELECTION_WINDOW_MS, 1_800_000.0);
        assert_eq!(KICK_WINDOW_MS, 600_000.0);
    }

    #[test]
    fn candidates_for_picks_top_five_by_leadership() {
        let cats = [
            cat("a", 10.0),
            cat("b", 90.0),
            cat("c", 50.0),
            cat("d", 70.0),
            cat("e", 30.0),
            cat("f", 80.0),
            cat("g", 60.0),
        ];

        assert_eq!(
            candidates_for_unbarred(&cats),
            ["b", "f", "d", "g", "c"].map(str::to_owned)
        );
    }

    #[test]
    fn candidates_for_excludes_barred_cats() {
        let cats = [cat("a", 90.0), cat("b", 80.0), cat("c", 70.0)];
        let barred = HashSet::from(["a".to_owned()]);

        assert_eq!(
            candidates_for(&cats, &barred),
            ["b", "c"].map(str::to_owned)
        );
    }

    #[test]
    fn candidates_for_handles_fewer_than_five_cats() {
        assert_eq!(candidates_for_unbarred(&[cat("a", 10.0)]), ["a".to_owned()]);
        assert!(candidates_for_unbarred(&[]).is_empty());
    }

    #[test]
    fn candidates_for_keeps_input_order_for_equal_or_nan_comparisons_like_stable_js_sort() {
        let cats = [
            cat("a", 10.0),
            cat("nan", f64::NAN),
            cat("b", 10.0),
            cat("c", 20.0),
        ];

        assert_eq!(
            candidates_for(&cats, ["nobody"].as_slice()),
            ["a", "nan", "c", "b"].map(str::to_owned)
        );
    }

    #[test]
    fn tally_votes_counts_votes_per_candidate() {
        let actual = tally_votes(&[vote("p1", "a"), vote("p2", "a"), vote("p3", "b")]);

        assert_eq!(actual, tally([("a", 2), ("b", 1)]));
    }

    #[test]
    fn tally_votes_collapses_duplicate_players_to_latest_vote_position() {
        let actual = tally_votes(&[vote("p1", "a"), vote("p1", "b")]);

        assert_eq!(actual, tally([("b", 1)]));
    }

    #[test]
    fn election_winner_picks_candidate_with_most_votes() {
        let candidates = [cat("a", 40.0), cat("b", 90.0), cat("c", 60.0)];

        assert_eq!(
            election_winner(&candidates, &tally([("a", 3), ("b", 1), ("c", 2)])),
            Some("a".to_owned())
        );
    }

    #[test]
    fn election_winner_breaks_ties_by_higher_leadership() {
        let candidates = [cat("a", 40.0), cat("b", 90.0), cat("c", 60.0)];

        assert_eq!(
            election_winner(&candidates, &tally([("a", 2), ("c", 2)])),
            Some("c".to_owned())
        );
    }

    #[test]
    fn election_winner_falls_back_to_highest_leadership_with_zero_votes() {
        let candidates = [cat("a", 40.0), cat("b", 90.0), cat("c", 60.0)];

        assert_eq!(
            election_winner(&candidates, &tally([])),
            Some("b".to_owned())
        );
    }

    #[test]
    fn election_winner_ignores_votes_for_non_candidates() {
        let candidates = [cat("a", 40.0), cat("b", 90.0), cat("c", 60.0)];

        assert_eq!(
            election_winner(&candidates, &tally([("z", 10), ("a", 1)])),
            Some("a".to_owned())
        );
    }

    #[test]
    fn election_winner_returns_none_with_no_candidates() {
        assert_eq!(election_winner(&[], &tally([("a", 1)])), None);
    }

    #[test]
    fn should_trigger_kick_requires_threshold_of_distinct_voters() {
        let four = (0..KICK_THRESHOLD - 1)
            .map(|index| vote(&format!("p{index}"), "leader"))
            .collect::<Vec<_>>();
        assert!(!should_trigger_kick(&four));

        let five = (0..KICK_THRESHOLD)
            .map(|index| vote(&format!("p{index}"), "leader"))
            .collect::<Vec<_>>();
        assert!(should_trigger_kick(&five));
    }

    #[test]
    fn should_trigger_kick_does_not_count_same_player_twice() {
        let stuffed = (0..KICK_THRESHOLD + 3)
            .map(|_| vote("same", "leader"))
            .collect::<Vec<_>>();

        assert!(!should_trigger_kick(&stuffed));
    }

    #[test]
    fn election_due_is_due_when_there_has_never_been_an_election() {
        assert!(election_due(None, 1_000_000.0, TERM_MS));
    }

    #[test]
    fn election_due_is_due_once_the_term_expires() {
        assert!(election_due(Some(1_000.0), 1_000.0 + TERM_MS, TERM_MS));
        assert!(!election_due(
            Some(1_000.0),
            1_000.0 + TERM_MS - 1.0,
            TERM_MS
        ));
    }
}
