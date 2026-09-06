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
