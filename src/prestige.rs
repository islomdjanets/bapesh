//! Where the prestige service lives.
//!
//! One address, in one place. It was previously written out as a literal in nine
//! places across four repositories — `currency` four times, `energy` three,
//! plus a private copy in gg_arcade and another in raffle_bot — so moving
//! prestige meant finding all nine by hand, with no compiler to say when one had
//! been missed. A staging deployment was not expressible at all.
//!
//! **Ungated on purpose.** `currency` and `energy` are separate features and
//! both talk to prestige, so this cannot live inside either without making one
//! depend on the other. It carries no dependencies of its own — a `OnceLock` and
//! an env read — so being always-compiled costs nothing.
//!
//! Shaped after `tasker::host()`, deliberately: two service addresses that
//! behave differently under a missing variable is one more thing to remember
//! than anybody needs.

use std::sync::OnceLock;

use crate::env;

static PRESTIGE: OnceLock<String> = OnceLock::new();

/// The deployed prestige, used when `PRESTIGE_HOST` says nothing.
///
/// Public over the internet rather than on Railway's private network, unlike
/// `tasker::DEFAULT_HOST`: prestige is reached from services that are not
/// necessarily deployed beside it, and the internal-secret header is what
/// protects the traffic rather than the network boundary.
pub const DEFAULT_HOST: &str = "https://prestige.up.railway.app";

/// Where this deployment's prestige lives.
///
/// Falls back to `DEFAULT_HOST` rather than panicking on a missing
/// `PRESTIGE_HOST`, for the same reason `tasker::host` does: every existing
/// caller hardcoded this exact string and none of them set a variable, so
/// requiring one now would take down every service on the next deploy to buy
/// nothing.
///
/// A trailing slash is trimmed so callers can append `/energy/spend` without
/// each having to think about it — a `//` in the path is the kind of thing that
/// works against one router and 404s against the next.
pub fn host() -> &'static str {
    PRESTIGE.get_or_init(|| match env::get("PRESTIGE_HOST") {
        Ok(h) if !h.trim().is_empty() => h.trim().trim_end_matches('/').to_string(),
        _ => DEFAULT_HOST.to_string(),
    })
}

// ── the subscription ────────────────────────────────────────────────────────
//
// One ladder for the whole ecosystem. prestige owns it — `subscriptions` is its
// table — and every other service reads the same answer from
// `/internal/tier/{user_id}`, which is what makes a plan bought inside one game
// count inside all of them.
//
// The *vocabulary* lives here rather than in each service because three copies
// of one enum is three chances to disagree with the authority, with no compiler
// between them. When prestige grows a plan, a service holding its own copy
// reads the new name as `Free` — and because that is the correct fail-closed
// answer for an unknown name, nothing anywhere reports a problem: a paying
// subscriber is simply, silently, downgraded.
//
// What does *not* live here is policy. What a plan costs, how wide it makes the
// meter, what it multiplies season points by, how many games it lets you
// publish — those belong to the service that enforces them. This module knows
// which plans exist and how to ask who is on one.

/// A plan, or the absence of one.
///
/// Ordered, and the order is the product: a perk gated at PLUS is one a PRO
/// subscriber has too, so a gate is a `>=` against a floor rather than a list of
/// plans that has to be edited again the next time one is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Plus,
    Pro,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Plus => "plus",
            Tier::Pro => "pro",
        }
    }

    /// A plan name read back off the wire or out of a column.
    ///
    /// Anything unrecognised is `Free`: a name this build has never heard of
    /// must not hand out the top rung by accident. See the module note above
    /// for why that default is also the reason this type is shared rather than
    /// copied.
    pub fn from_name(name: &str) -> Tier {
        match name.trim() {
            "plus" => Tier::Plus,
            "pro" => Tier::Pro,
            _ => Tier::Free,
        }
    }

    /// On any paid plan.
    pub fn is_paid(self) -> bool {
        self != Tier::Free
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How long to wait on prestige before giving up.
///
/// This sits inside request paths that are doing something else — starting a
/// game, completing a task — and reqwest applies no timeout of its own. A
/// prestige that hangs must cost the caller a perk, not the whole request.
#[cfg(feature = "energy")]
const TIER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Which plan this person is on, asked of prestige.
///
/// **Fails closed.** Unreachable, a 5xx, or a body that made no sense all read
/// as `Free`. That costs a subscriber their perks for the length of an outage;
/// the opposite default would make an outage the cheapest way to get them, which
/// is the failure worth avoiding.
///
/// A caller that needs to tell "not a subscriber" from "prestige did not answer"
/// — one is a fact worth caching, the other is not — wants `tier_status`.
///
/// Behind the `energy` feature rather than one of its own: the services that ask
/// this are the services that spend energy, and a second feature flag to remember
/// buys nothing. The `Tier` type above stays ungated, so a service can speak the
/// vocabulary without taking on reqwest.
#[cfg(feature = "energy")]
pub async fn tier(user_id: i64, client: &reqwest::Client, internal_secret: &str) -> Tier {
    tier_status(user_id, client, internal_secret).await.unwrap_or(Tier::Free)
}

/// prestige's answer, or `None` when it gave none.
#[cfg(feature = "energy")]
pub async fn tier_status(
    user_id: i64,
    client: &reqwest::Client,
    internal_secret: &str,
) -> Option<Tier> {
    let resp = client
        .get(format!("{}/internal/tier/{user_id}", host()))
        .header("X-Internal-Secret", internal_secret)
        .timeout(TIER_TIMEOUT)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.ok()?;
            match body["tier"].as_str() {
                Some(name) => Some(Tier::from_name(name)),
                // A prestige that still answers the older `{plus: bool}` — the
                // shape from before there were two plans — reads as PLUS. The
                // best guess available, and it never over-grants, because PRO is
                // not in that vocabulary.
                None => body["plus"].as_bool().map(|paid| {
                    if paid { Tier::Plus } else { Tier::Free }
                }),
            }
        }
        Ok(r) => {
            eprintln!("tier lookup for {user_id}: prestige answered {}", r.status());
            None
        }
        Err(e) => {
            eprintln!("tier lookup for {user_id} failed: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_name_reads_back_and_anything_else_is_free() {
        assert_eq!(Tier::from_name("pro"), Tier::Pro);
        assert_eq!(Tier::from_name(" plus "), Tier::Plus);
        assert_eq!(Tier::from_name("platinum"), Tier::Free);
        assert_eq!(Tier::from_name(""), Tier::Free);
    }

    #[test]
    fn the_ladder_is_ordered_so_a_gate_can_be_a_floor() {
        assert!(Tier::Pro > Tier::Plus);
        assert!(Tier::Plus > Tier::Free);
        assert!(Tier::Pro.is_paid() && Tier::Plus.is_paid());
        assert!(!Tier::Free.is_paid());
    }

    #[test]
    fn a_name_survives_a_round_trip() {
        for tier in [Tier::Free, Tier::Plus, Tier::Pro] {
            assert_eq!(Tier::from_name(tier.as_str()), tier);
            assert_eq!(tier.to_string(), tier.as_str());
        }
    }
}
