use common::islandtrait::island::{Island, IslandInit, IslandResponse, MoorResponse};
use common::islandtrait::types::{Clue, Round, ShipId};
use std::collections::{BTreeMap, BTreeSet};

/// How many Ships may be moored here at the same time.
// TODO: Have to choose the berth count.
const BERTHS: usize = 2;

/// Rounds a Ship must stay moored before it is answered.
// TODO: what does the price depend on? Need to discuss
const BASE_WAIT: u32 = 3;
/// Guarantees:
/// - it never lies: every Clue it answers with is its original Clue (no weakening yet);
/// - a Ship that moors and waits `BASE_WAIT` rounds always receives the full-strength
///   Clue, after which the Island has nothing more to offer it;
/// - at most `BERTHS` Ships are moored at once;
/// - it is deterministic and never panics.
#[derive(Debug)]
pub struct InRustWeTrust1 {
    // using BTree because Hashmap iterates in different order on different runs, BTree iterates in ascending order on ShipId
    /// The clue handed over by the Chart at construction.
    clue: Clue,
    /// Ships currently moored here.
    moored: BTreeSet<ShipId>,
    /// Rounds each moored Ship must still wait before being answered.
    waiting: BTreeMap<ShipId, u32>,
    /// Ships that already received the full-strength Clue.
    served_full: BTreeSet<ShipId>,
}

impl Island for InRustWeTrust1 {
    fn new(init: IslandInit) -> Self {
        // TODO: use `init.seed` for a PRNG and `init.params` if the policy needs them.
        //stores clue and starts empty sets
        Self {
            clue: init.clue,
            moored: BTreeSet::new(),
            waiting: BTreeMap::new(),
            served_full: BTreeSet::new(),
        }
    }

    fn moor(&mut self, ship: ShipId) {
        // add the ship to moored, if the insert is new start countdown at BASE_WAIT
        if self.moored.insert(ship) {
            // TODO : Need to decide whether earlier waiting progress carries over
            self.waiting.insert(ship, BASE_WAIT);
        }
    }

    fn can_moor(&self, ship: ShipId) -> MoorResponse {
        // Accepted if the Ship is already moored or a berth is free, else NoBerth
        // TODO: Have to decide criteria for MoorResponse::Refused.
        if self.moored.contains(&ship) || self.moored.len() < BERTHS {
            MoorResponse::Accepted
        } else {
            MoorResponse::NoBerth
        }
    }

    fn is_moored(&self, ship: ShipId) -> bool {
        self.moored.contains(&ship)
    }

    fn leave_berth(&mut self, ship: ShipId) {
        // remove ship from moored and drop countdown
        self.moored.remove(&ship);
        // TODO: Have to decide if we want to keep the progress
        self.waiting.remove(&ship);
    }

    fn ask_clue(&mut self, ship: ShipId) -> IslandResponse {
        if self.served_full.contains(&ship) {
            return IslandResponse::NothingMore;
        }
        if !self.moored.contains(&ship) {
            // The Chart should never ask for a Ship that is not moored; answer, don't panic.
            return IslandResponse::Refused;
        }
        match self.waiting.get(&ship).copied().unwrap_or(0) {
            0 => {
                // TODO: Have to decide if we answer with a weakened Clue (same kind) on early visits.
                self.served_full.insert(ship);
                IslandResponse::Respond(self.clue)
            }
            n => IslandResponse::Wait(n),
        }
    }

    fn remaining_wait(&self, ship: ShipId) -> Option<u32> {
        if self.served_full.contains(&ship) {
            return None;
        }
        self.waiting.get(&ship).copied()
    }

    fn has_more_to_offer(&self, ship: ShipId) -> bool {
        !self.served_full.contains(&ship)
    }

    fn round_advance(&mut self, _round: Round) {
        // decrease countdowwn by 1 for each moored ship, using saturating_sub so it stops at 0
        for (ship, left) in &mut self.waiting {
            if self.moored.contains(ship) {
                *left = left.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::clues::{Axis, Bound};
    use common::islandtrait::types::Params;

    const CLUE: Clue = Clue::HalfPlane {
        axis: Axis::Q,
        bound: Bound::AtLeast(1),
    };

    fn island() -> InRustWeTrust1 {
        InRustWeTrust1::new(IslandInit {
            clue: CLUE,
            seed: 42,
            params: Params::default(),
        })
    }

    // A moored Ship that waits BASE_WAIT rounds gets the full Clue, then NothingMore.
    #[test]
    fn full_clue_after_waiting_then_nothing_more() {
        let mut island = island();
        let ship = ShipId(0);
        island.moor(ship);

        for round in 2..2 + BASE_WAIT {
            assert!(matches!(island.ask_clue(ship), IslandResponse::Wait(_)));
            island.round_advance(round);
        }

        assert_eq!(island.ask_clue(ship), IslandResponse::Respond(CLUE));
        assert!(!island.has_more_to_offer(ship));
        assert_eq!(island.ask_clue(ship), IslandResponse::NothingMore);
        assert_eq!(island.remaining_wait(ship), None);
    }

    // Once every berth is taken new Ships get `NoBerth`, and leaving frees a berth.
    #[test]
    fn berths_are_limited_and_freed_on_leave() {
        let mut island = island();
        let ships: Vec<ShipId> = (0..=u8::try_from(BERTHS).unwrap_or(u8::MAX)).map(ShipId).collect();
        for &ship in &ships[..BERTHS] {
            island.moor(ship);
        }

        let late = ships[BERTHS];
        assert_eq!(island.can_moor(late), MoorResponse::NoBerth);
        island.leave_berth(ships[0]);
        assert_eq!(island.can_moor(late), MoorResponse::Accepted);
    }

    // Asking can_moor does not moor the Ship.
    #[test]
    fn can_moor_does_not_change_state() {
        let island = island();
        let ship = ShipId(1);
        assert_eq!(island.can_moor(ship), MoorResponse::Accepted);
        assert!(!island.is_moored(ship));
    }

    // A Ship that is not moored is refused the Clue, and nothing panics.
    #[test]
    fn unmoored_ship_is_refused_without_panicking() {
        let mut island = island();
        assert_eq!(island.ask_clue(ShipId(3)), IslandResponse::Refused);
        island.leave_berth(ShipId(3));
        assert!(island.has_more_to_offer(ShipId(3)));
    }

    // Plays a fixed script of calls and records every observable answer.
    fn play_script(island: &mut InRustWeTrust1) -> Vec<String> {
        let (a, b, c) = (ShipId(0), ShipId(1), ShipId(2));
        let mut log = Vec::new();
        log.push(format!("{:?}", island.can_moor(a)));
        island.moor(a);
        island.moor(b);
        log.push(format!("{:?}", island.can_moor(c)));
        for round in 2..10 {
            for ship in [a, b, c] {
                log.push(format!("{:?}", island.ask_clue(ship)));
                log.push(format!("{:?}", island.remaining_wait(ship)));
                log.push(format!("{:?}", island.has_more_to_offer(ship)));
            }
            if round == 4 {
                island.leave_berth(b);
                island.moor(c);
            }
            island.round_advance(round);
        }
        log
    }

    // Two Islands built from the same init and given the same calls answer identically.
    #[test]
    fn same_init_and_calls_give_same_results() {
        let first = play_script(&mut island());
        let second = play_script(&mut island());
        assert_eq!(first, second);
    }

    // A Ship that leaves before being served starts the full wait again when it returns.
    #[test]
    fn leaving_resets_waiting_progress() {
        let mut island = island();
        let ship = ShipId(0);
        island.moor(ship);
        island.round_advance(2);
        island.round_advance(3);
        assert_eq!(island.remaining_wait(ship), Some(BASE_WAIT - 2));

        island.leave_berth(ship);
        assert_eq!(island.remaining_wait(ship), None);

        island.moor(ship);
        assert_eq!(island.remaining_wait(ship), Some(BASE_WAIT));
        assert_eq!(island.ask_clue(ship), IslandResponse::Wait(BASE_WAIT));
    }

    // Rounds that pass before a Ship moors do not shorten its wait.
    #[test]
    fn rounds_at_sea_do_not_count() {
        let mut island = island();
        let ship = ShipId(0);
        for round in 2..20 {
            island.round_advance(round);
        }
        island.moor(ship);
        assert_eq!(island.remaining_wait(ship), Some(BASE_WAIT));
        assert_eq!(island.ask_clue(ship), IslandResponse::Wait(BASE_WAIT));
    }

    // Each Ship pays its own wait one Ship's waiting does not help another.
    #[test]
    fn each_ship_has_its_own_countdown() {
        let mut island = island();
        let (early, late) = (ShipId(0), ShipId(1));
        island.moor(early);
        island.round_advance(2);
        island.round_advance(3);
        island.moor(late);

        assert_eq!(island.remaining_wait(early), Some(BASE_WAIT - 2));
        assert_eq!(island.remaining_wait(late), Some(BASE_WAIT));

        island.round_advance(4);
        assert_eq!(island.ask_clue(early), IslandResponse::Respond(CLUE));
        assert_eq!(island.ask_clue(late), IslandResponse::Wait(BASE_WAIT - 1));
    }

    // Repeated calls to the read-only methods leave the Island's state unchanged.
    #[test]
    fn read_only_methods_do_not_change_state() {
        let mut island = island();
        island.moor(ShipId(0));
        island.round_advance(2);
        let before = format!("{island:?}");

        for _ in 0..3 {
            for ship in (0..6).map(ShipId) {
                let _ = island.can_moor(ship);
                let _ = island.is_moored(ship);
                let _ = island.remaining_wait(ship);
                let _ = island.has_more_to_offer(ship);
                let _ = island.ask_history(ship);
            }
        }

        assert_eq!(format!("{island:?}"), before);
    }

    // Unusual call orders (double moor/leave, extra rounds, Round::MAX) do not panic.
    #[test]
    fn odd_call_orders_do_not_panic() {
        let mut island = island();
        let ship = ShipId(0);

        // Rounds with nobody moored.
        island.round_advance(2);

        // Mooring twice keeps one berth and does not reset progress.
        island.moor(ship);
        island.round_advance(3);
        island.moor(ship);
        assert_eq!(island.remaining_wait(ship), Some(BASE_WAIT - 1));
        assert!(island.is_moored(ship));

        // Many more rounds than needed: the countdown stops at zero.
        for round in 4..50 {
            island.round_advance(round);
        }
        assert_eq!(island.remaining_wait(ship), Some(0));
        assert_eq!(island.ask_clue(ship), IslandResponse::Respond(CLUE));
        assert_eq!(island.ask_clue(ship), IslandResponse::NothingMore);
        assert_eq!(island.ask_clue(ship), IslandResponse::NothingMore);

        // Leaving twice, and leaving a Ship that never moored.
        island.leave_berth(ship);
        island.leave_berth(ship);
        island.leave_berth(ShipId(5));
        assert!(!island.is_moored(ship));

        // Extreme round number.
        island.round_advance(Round::MAX);
    }

    // A Ship that already has the full Clue gets NothingMore when it comes back.
    #[test]
    fn served_ship_returning_gets_nothing_more() {
        let mut island = island();
        let ship = ShipId(0);
        island.moor(ship);
        for round in 2..2 + BASE_WAIT {
            island.round_advance(round);
        }
        assert_eq!(island.ask_clue(ship), IslandResponse::Respond(CLUE));
        island.leave_berth(ship);

        assert_eq!(island.can_moor(ship), MoorResponse::Accepted);
        island.moor(ship);
        assert_eq!(island.ask_clue(ship), IslandResponse::NothingMore);
        assert_eq!(island.remaining_wait(ship), None);
        assert!(!island.has_more_to_offer(ship));
    }

    // This Island offers neither hull repairs nor mooring history.
    #[test]
    fn optional_services_are_not_offered() {
        let mut island = island();
        let ship = ShipId(0);
        island.moor(ship);
        assert!(island.repair_hull(ship).is_none());
        assert!(island.ask_history(ship).is_none());
    }
}
