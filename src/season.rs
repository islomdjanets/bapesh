//! How a season's pool is split: who is paid, in what tier, how much.
//!
//! Pure arithmetic over a ranked field, shared by every service that runs a
//! season — tasker's local and global seasons first, and gg_arcade's, which is
//! where this was written. It moved here so the prize screen and the payout of
//! every season in the ecosystem describe one split, and so a change to the
//! shape of the curve is one edit rather than one per service.
//!
//! Nothing here reads a table or a clock. A caller ranks its own standings
//! (points descending, ties broken deterministically), decides its own floor,
//! and hands both in. What comes back is exact: `distribute` always sums to
//! the pool, and `awards` pays exactly the band the screen showed.
//!
//! # The two rules the numbers encode
//!
//! **The paid band is a share of the qualified field.** Thirty percent of the
//! people who cleared the floor, never of everyone with a row — a hundred alts
//! worth a point each would otherwise widen the band and drag a real account
//! inside it. The floor is what makes each alt cost a genuine effort.
//!
//! **Tiers are weights, not percentages.** A table of percentages must sum to
//! 100 and every empty tier — a small season empties several — leaves part of
//! the pool needing a redistribution rule. Weights normalise themselves: an
//! empty tier contributes nothing to the divisor and costs nothing.

use serde::Serialize;

/// Places paid in a field too small for a percentage to describe.
///
/// At thirty percent a field of four pays one place, which is not a prize pool,
/// it is a raffle. Below this size the share is ignored and the top three are
/// paid — bounded by the field, so a season with two entrants pays two.
pub const MIN_PAID_PLACES: i64 = 3;

/// Tier thresholds, as a percentage of the qualified field, narrowest first.
///
/// The paid band is wide by design — a third of the field — and a flat split
/// across it would leave nothing to compete for above the cut line. These are
/// what keep a reason to climb after "eligible" is already won, and what give
/// every row a next threshold near enough to aim at.
///
/// Everything paid but below the last of these is `Bronze`, which runs to
/// whatever the season's `reward_share` is set to.
pub const TIERS: &[(i64, Tier)] = &[
    (1, Tier::Champion),
    (5, Tier::Elite),
    (10, Tier::Gold),
    (20, Tier::Silver),
];

/// Where a paid place sits within the band.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Champion,
    Elite,
    Gold,
    Silver,
    Bronze,
}

impl Tier {
    /// This tier's share of the pool, *per member*, relative to the others.
    ///
    /// What these say is the ratio between places. A champion is worth five
    /// bronzes; the shape of the curve is the whole of the reward design and it
    /// is all here, in five numbers.
    pub fn weight(self) -> i64 {
        match self {
            Tier::Champion => 50,
            Tier::Elite => 30,
            Tier::Gold => 20,
            Tier::Silver => 14,
            Tier::Bronze => 10,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tier::Champion => "champion",
            Tier::Elite => "elite",
            Tier::Gold => "gold",
            Tier::Silver => "silver",
            Tier::Bronze => "bronze",
        }
    }
}

/// One rung of the ladder the prize screen shows.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Band {
    pub name: &'static str,
    pub top_percent: i64,
    pub weight: i64,
}

/// One person's prize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Award {
    pub user_id: i64,
    pub rank: i64,
    pub tier: Tier,
    pub amount: i64,
}

/// Ceiling division, on the integers the shares are computed in.
///
/// Rounding up rather than down throughout: a field of ten at thirty percent
/// pays three, not two, and a tier boundary lands *on* the rank that reaches it
/// rather than one past it.
fn ceil_div(n: i64, d: i64) -> i64 {
    if d == 0 { return 0; }
    (n + d - 1) / d
}

/// How many places a pool pays, given how many entrants qualified.
pub fn paid_places(qualified: i64, reward_share: i64) -> i64 {
    if qualified <= 0 {
        return 0;
    }
    ceil_div(qualified * reward_share, 100)
        .max(MIN_PAID_PLACES)
        .min(qualified)
}

/// Which tier a rank falls in, or `None` when it is outside the paid band.
///
/// `rank` is 1-based and counted over the qualified field alone — which a
/// caller gets for free by ranking on points, because every qualified row
/// therefore outranks every unqualified one.
pub fn tier_for(rank: i64, qualified: i64, reward_share: i64) -> Option<Tier> {
    if rank < 1 || rank > paid_places(qualified, reward_share) {
        return None;
    }

    for (pct, tier) in TIERS {
        // A tier narrower than the season pays is a tier that exists; one wider
        // than the paid band does not, and the rows it would have covered fall
        // through to Bronze.
        if *pct > reward_share {
            break;
        }
        if rank <= ceil_div(qualified * pct, 100).max(1) {
            return Some(*tier);
        }
    }

    Some(Tier::Bronze)
}

/// The reward ladder, in the order a player climbs it.
///
/// Built from `TIERS` and the weights rather than written out, so the screen
/// explaining the prize split cannot describe a different split from the one
/// `distribute` performs.
///
/// Bronze is whatever is paid *below* the narrowest named tier, so it is listed
/// only when the season pays wider than that tier reaches. When `reward_share`
/// equals a named tier's own percent — 1, 5, 10 or 20 — `tier_for` stops at that
/// tier for every paid rank and never returns Bronze; listing it there would
/// show players a band that pays nobody.
pub fn reward_ladder(reward_share: i64) -> Vec<Band> {
    let named: Vec<_> = TIERS.iter().filter(|(pct, _)| *pct <= reward_share).collect();

    // TIERS is ascending, so the last kept entry is the widest one that exists.
    let bronze_pays = match named.last() {
        Some((widest, _)) => *widest < reward_share,
        None => true,
    };

    named
        .iter()
        .map(|(pct, tier)| Band { name: tier.name(), top_percent: *pct, weight: tier.weight() })
        .chain(bronze_pays.then(|| Band {
            name: Tier::Bronze.name(),
            top_percent: reward_share,
            weight: Tier::Bronze.weight(),
        }))
        .collect()
}

/// Splits `pool` across the given places by tier weight, exactly.
///
/// Integer arithmetic, with the rounding remainder handed to the first place,
/// so the amounts always sum to `pool` and no unit is invented or lost. An
/// empty ladder owes nobody and a pool of nothing pays nobody; neither is an
/// error, and both happen — the first season to close may be either.
pub fn distribute(pool: i64, tiers: &[Tier]) -> Vec<i64> {
    if tiers.is_empty() {
        return Vec::new();
    }
    if pool <= 0 {
        return vec![0; tiers.len()];
    }

    let total: i64 = tiers.iter().map(|t| t.weight()).sum();
    let mut out: Vec<i64> = tiers.iter().map(|t| pool * t.weight() / total).collect();

    let assigned: i64 = out.iter().sum();
    out[0] += pool - assigned;

    out
}

/// Who a pool owes, and how much.
///
/// `standings` is `(user_id, rank, points)`, ranked — rank 1 first, points
/// descending. `floor` is what a row must have earned to count as a
/// participant; rows below it are neither paid nor counted in the field the
/// band is a share of. The people paid are exactly the rows the screen marked
/// eligible while the season ran; nothing is decided here that a player could
/// not already see.
pub fn awards(
    pool_amount: i64,
    standings: &[(i64, i64, i64)],
    floor: i64,
    reward_share: i64,
) -> Vec<Award> {
    let qualified = standings.iter().filter(|(_, _, points)| *points >= floor).count() as i64;
    let paid = paid_places(qualified, reward_share);
    if paid <= 0 {
        return Vec::new();
    }

    let winners = &standings[..(paid as usize).min(standings.len())];

    // `Bronze` as the fallback is unreachable — `tier_for` returns `Some` for
    // every rank inside `paid_places` — but a prize is not the place to unwrap.
    let tiers: Vec<Tier> = winners
        .iter()
        .map(|(_, rank, _)| tier_for(*rank, qualified, reward_share).unwrap_or(Tier::Bronze))
        .collect();

    let amounts = distribute(pool_amount, &tiers);

    winners
        .iter()
        .enumerate()
        .map(|(i, (user_id, rank, _))| Award {
            user_id: *user_id,
            rank: *rank,
            tier: tiers[i],
            amount: amounts[i],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: i64 = 100;

    #[test]
    fn the_pool_is_distributed_exactly() {
        let shapes: Vec<Vec<Tier>> = vec![
            vec![Tier::Champion],
            vec![Tier::Champion, Tier::Bronze],
            vec![Tier::Champion, Tier::Elite, Tier::Gold, Tier::Silver, Tier::Bronze],
            vec![Tier::Bronze; 97],
            [vec![Tier::Champion; 3], vec![Tier::Silver; 11], vec![Tier::Bronze; 40]].concat(),
        ];

        for pool in [0i64, 1, 7, 99, 1_000, 100_000, 1_000_003, 987_654_321] {
            for shape in &shapes {
                let out = distribute(pool, shape);
                assert_eq!(out.len(), shape.len());
                assert_eq!(
                    out.iter().sum::<i64>(),
                    pool,
                    "pool {pool} over {} places did not add back up",
                    shape.len()
                );
                assert!(out.iter().all(|a| *a >= 0), "a negative prize");
            }
        }
    }

    /// A higher tier is never worth less than a lower one.
    #[test]
    fn better_places_are_worth_more() {
        let ladder = [Tier::Champion, Tier::Elite, Tier::Gold, Tier::Silver, Tier::Bronze];
        let out = distribute(1_000_000, &ladder);
        for pair in out.windows(2) {
            assert!(pair[0] >= pair[1], "the ladder is not monotonic: {out:?}");
        }
        // And the ratio is the authored one: a champion is five bronzes.
        assert_eq!(Tier::Champion.weight(), 5 * Tier::Bronze.weight());
    }

    #[test]
    fn nothing_to_pay_is_not_a_failure() {
        assert!(distribute(1000, &[]).is_empty());
        assert_eq!(distribute(0, &[Tier::Champion, Tier::Bronze]), vec![0, 0]);
        assert!(awards(1000, &[], FLOOR, 30).is_empty());
    }

    /// Only qualified rows are paid, and only down to the cut line the screen
    /// showed all season.
    #[test]
    fn only_the_eligible_are_paid() {
        // Ten players, six of them above the floor.
        let standings: Vec<(i64, i64, i64)> = (0..10i64)
            .map(|i| (100 + i, i + 1, if i < 6 { FLOOR + 100 - i } else { 5 }))
            .collect();

        let out = awards(10_000, &standings, FLOOR, 30);

        // Six qualified, 30% of six is 1.8 -> 2, floored up to MIN_PAID_PLACES.
        assert_eq!(out.len(), paid_places(6, 30) as usize);
        assert_eq!(out.len(), MIN_PAID_PLACES as usize);
        assert!(out.iter().all(|a| a.rank <= 6), "an unqualified row was paid: {out:?}");
        assert_eq!(out.iter().map(|a| a.amount).sum::<i64>(), 10_000);
    }

    #[test]
    fn the_paid_band_is_a_share_of_the_field() {
        assert_eq!(paid_places(100, 30), 30);
        assert_eq!(paid_places(1000, 30), 300);
        assert_eq!(paid_places(50, 40), 20);
        // Rounded up: a field of ten pays three, not two.
        assert_eq!(paid_places(10, 25), 3);
    }

    /// A percentage of a tiny field is a raffle, so the floor takes over — but
    /// never pays more places than there are people.
    #[test]
    fn a_small_field_pays_the_floor_and_no_more() {
        assert_eq!(paid_places(4, 30), MIN_PAID_PLACES);
        assert_eq!(paid_places(2, 30), 2, "cannot pay more places than entrants");
        assert_eq!(paid_places(1, 30), 1);
        assert_eq!(paid_places(0, 30), 0, "an empty pool pays nobody");
    }

    /// Tiers narrow as the field grows, which is what keeps a reason to climb
    /// after the cut line is already cleared.
    #[test]
    fn tiers_band_the_paid_places() {
        let (q, share) = (1000, 30);
        assert_eq!(tier_for(1, q, share), Some(Tier::Champion));
        assert_eq!(tier_for(10, q, share), Some(Tier::Champion), "top 1% is 10 of 1000");
        assert_eq!(tier_for(11, q, share), Some(Tier::Elite));
        assert_eq!(tier_for(50, q, share), Some(Tier::Elite), "top 5%");
        assert_eq!(tier_for(51, q, share), Some(Tier::Gold));
        assert_eq!(tier_for(100, q, share), Some(Tier::Gold), "top 10%");
        assert_eq!(tier_for(101, q, share), Some(Tier::Silver));
        assert_eq!(tier_for(200, q, share), Some(Tier::Silver), "top 20%");
        assert_eq!(tier_for(201, q, share), Some(Tier::Bronze));
        assert_eq!(tier_for(300, q, share), Some(Tier::Bronze), "the cut line");
        assert_eq!(tier_for(301, q, share), None, "one place outside pays nothing");
    }

    /// `reward_ladder` is what the prize screen shows; `tier_for` is what
    /// `distribute` pays. Assert both ways: nothing advertised is unreachable,
    /// and nothing paid goes unlisted. A large field on purpose —
    /// `MIN_PAID_PLACES` widens a tiny field past any percentage ladder.
    #[test]
    fn the_ladder_and_the_payout_describe_the_same_split() {
        let qualified = 1000;

        for share in [1, 3, 5, 10, 20, 30, 50, 100] {
            let ladder = reward_ladder(share);

            let reached: std::collections::HashSet<&str> = (1..=paid_places(qualified, share))
                .filter_map(|rank| tier_for(rank, qualified, share))
                .map(Tier::name)
                .collect();

            for band in &ladder {
                assert!(
                    reached.contains(band.name),
                    "reward_share={share} advertises {}, but no paid rank lands in it",
                    band.name
                );
            }
            for name in &reached {
                assert!(
                    ladder.iter().any(|band| band.name == *name),
                    "reward_share={share} pays {name}, but the ladder never lists it",
                );
            }
        }
    }

    /// At a share equal to a named tier's own percent, that tier absorbs every
    /// paid rank and Bronze pays nobody — so it is not listed.
    #[test]
    fn bronze_is_listed_only_when_something_falls_to_it() {
        let names = |share| -> Vec<&'static str> {
            reward_ladder(share).iter().map(|b| b.name).collect()
        };

        for share in [1, 5, 10, 20] {
            assert!(
                !names(share).contains(&Tier::Bronze.name()),
                "reward_share={share} ends exactly on a named tier, so Bronze has no band",
            );
        }

        assert!(names(30).contains(&Tier::Bronze.name()));
        assert_eq!(names(30).len(), TIERS.len() + 1, "four named tiers, then Bronze");
    }

    /// Paying only the top 3% leaves Elite, Gold and Silver with no field to
    /// describe; the places below Champion fall through to Bronze rather than
    /// being handed a name that overstates them.
    #[test]
    fn a_tier_wider_than_the_band_does_not_appear() {
        let (q, share) = (1000, 3);
        let seen: Vec<Tier> = (1..=paid_places(q, share))
            .map(|rank| tier_for(rank, q, share).expect("inside the band"))
            .collect();

        assert!(seen.contains(&Tier::Champion), "the top 1% still exists");
        assert!(seen.contains(&Tier::Bronze), "and the rest of the band is Bronze");
        for wider in [Tier::Elite, Tier::Gold, Tier::Silver] {
            assert!(!seen.contains(&wider), "{wider:?} is wider than a 3% band and must not be awarded");
        }
    }

    /// Nobody outside the paid band carries a tier, however the field is shaped.
    #[test]
    fn outside_the_band_is_never_tiered() {
        for qualified in [0i64, 1, 5, 37, 500] {
            for share in [1i64, 30, 100] {
                let paid = paid_places(qualified, share);
                assert_eq!(tier_for(paid + 1, qualified, share), None);
                assert_eq!(tier_for(0, qualified, share), None, "rank is 1-based");
                assert_eq!(tier_for(-3, qualified, share), None);
                for rank in 1..=paid {
                    assert!(
                        tier_for(rank, qualified, share).is_some(),
                        "rank {rank} of {qualified} at {share}% is paid but untiered"
                    );
                }
            }
        }
    }

    /// The farm the floor exists to close, stated as arithmetic: a hundred
    /// one-point alts would widen a fifty-player band from 15 to 45 places if
    /// they counted. They do not, because the denominator is the qualified
    /// count — which `awards` computes from the floor, never from row count.
    #[test]
    fn padding_the_field_only_pays_if_the_padding_qualifies() {
        assert!(20 > paid_places(50, 30), "rank 20 is outside a real field of 50");
        assert!(20 <= paid_places(150, 30), "and inside a padded one -- the farm");

        let mut standings: Vec<(i64, i64, i64)> = Vec::new();
        for i in 0..50i64 {
            standings.push((1000 + i, i + 1, FLOOR + 50 - i));
        }
        for i in 0..100i64 {
            standings.push((i, 51 + i, 1));
        }

        let out = awards(10_000, &standings, FLOOR, 30);
        assert_eq!(out.len(), 15, "the band did not widen");
        assert!(out.iter().all(|a| a.rank <= 50));
    }
}
