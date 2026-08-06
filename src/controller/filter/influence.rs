//! Influence filters.
//!
//! Ported from the `filters.influences` block in
//! `web/price-check/filters/create-item-filters.ts` and the loop that turns it
//! into stats in `web/price-check/trade/pathofexile-trade.ts`, both in
//! Awakened PoE Trade.
//!
//! # Why this is not in the other reference
//!
//! Exiled Exchange 2 is a PoE2 fork and PoE2 has no influence, so its copy of
//! that loop is commented out. Porting only that reference left PoE1 without
//! influence filters at all.
//!
//! # Why it matters
//!
//! Influence is most of what a PoE1 base is worth. A Shaper ring can take
//! modifiers no other ring can, and a search that ignores the influence prices
//! it against every plain ring of the same base. That is off by an order of
//! magnitude, not by a little.
//!
//! # How the trade site indexes it
//!
//! Not as a real modifier. The influence line is printed on its own and the
//! site files it under `pseudo`, as `Has Shaper Influence` and its five
//! siblings.

use crate::types::item::Influence;

/// The stat reference the trade site files an influence under.
///
/// Exact spellings from the live `/data/stats` response. A wrong one produces
/// no filter rather than an error, which is why they are pinned by a test
/// against the real data file.
pub fn stat_reference(influence: Influence) -> &'static str {
    match influence {
        Influence::Crusader => "Has Crusader Influence",
        Influence::Elder => "Has Elder Influence",
        Influence::Hunter => "Has Hunter Influence",
        Influence::Redeemer => "Has Redeemer Influence",
        Influence::Shaper => "Has Shaper Influence",
        Influence::Warlord => "Has Warlord Influence",
    }
}

/// Which influences get a filter.
///
/// One or two. Never more.
///
/// # Why three is none
///
/// An item with three influences is an Elevated or otherwise special base, and
/// requiring all three matches only itself. The reference draws the line at
/// two and so does this.
pub fn filterable(influences: &[Influence]) -> &[Influence] {
    if influences.is_empty() || influences.len() > 2 {
        return &[];
    }

    influences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_influence_is_filterable() {
        assert_eq!(filterable(&[Influence::Shaper]), &[Influence::Shaper]);
    }

    #[test]
    fn two_influences_are_filterable() {
        // A Shaper and Elder base is the classic double influenced item and
        // both halves are why it is worth anything.
        let both = [Influence::Shaper, Influence::Elder];

        assert_eq!(filterable(&both), &both);
    }

    #[test]
    fn three_influences_are_not_filterable() {
        // Requiring all three matches only this item.
        let three = [Influence::Shaper, Influence::Elder, Influence::Hunter];

        assert!(filterable(&three).is_empty());
    }

    #[test]
    fn no_influence_is_not_filterable() {
        assert!(filterable(&[]).is_empty());
    }

    #[test]
    fn every_influence_has_a_reference() {
        // A missing spelling produces no filter and no error, so the whole set
        // is walked rather than trusted.
        for influence in [
            Influence::Crusader,
            Influence::Elder,
            Influence::Hunter,
            Influence::Redeemer,
            Influence::Shaper,
            Influence::Warlord,
        ] {
            let reference = stat_reference(influence);

            assert!(reference.starts_with("Has "), "{reference}");
            assert!(reference.ends_with(" Influence"), "{reference}");
        }
    }

    #[test]
    fn no_two_influences_share_a_reference() {
        // A copied line would file two influences under one pseudo id and
        // search for the wrong one.
        let all = [
            Influence::Crusader,
            Influence::Elder,
            Influence::Hunter,
            Influence::Redeemer,
            Influence::Shaper,
            Influence::Warlord,
        ];

        let mut seen: Vec<&str> = all.iter().map(|i| stat_reference(*i)).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();

        assert_eq!(seen.len(), before);
    }
}
