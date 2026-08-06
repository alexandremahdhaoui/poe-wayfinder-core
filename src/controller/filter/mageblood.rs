//! Searching for a Mageblood with duplicated legacy modifiers.
//!
//! Ported from `buildMageBloodNotFilter`, `buildFilterWithValue` and
//! `EASY_LEGACY_DUPLICATE_TO_FILTER`.
//!
//! # Why this belt needs its own logic
//!
//! A Mageblood carries up to four legacy of the vaal modifiers. The trade site
//! indexes each one as a separate option of one stat and gives no way to say
//! "four modifiers, two of them the same".
//!
//! A duplicated modifier is not a modifier the site can name. The user knows
//! their belt has two of something without knowing which, and a plain filter
//! on the ones they can read returns belts that have those four and nothing
//! duplicated.
//!
//! So the duplicates become a count filter: at least this many of the possible
//! options must be present at these strengths. That is the only shape the site
//! offers that expresses the question.

/// What a count filter has to ask for.
#[derive(Debug, Clone, PartialEq)]
pub struct CountFilter {
    /// How many of the listed conditions must hold.
    pub at_least: u32,
    /// The floors each option is checked against.
    pub floors: Vec<u32>,
}

/// What to send for a belt with this many known and duplicated modifiers.
#[derive(Debug, Clone, PartialEq)]
pub enum Duplicates {
    /// Send the readable modifiers and nothing else.
    Plain,
    /// Send the readable modifiers, each required at this strength.
    AtStrength(u32),
    /// Send the readable modifiers plus a count filter for the duplicates.
    Counted(CountFilter),
}

/// The most legacy modifiers a Mageblood can carry.
///
/// Four. Asking for more describes a belt that cannot exist, and the site
/// answers an empty result rather than an error, so the user sees nothing and
/// no reason why.
pub const MAX_LEGACY: u32 = 4;

/// Work out what to send.
///
/// Ported from `buildMageBloodNotFilter`.
///
/// Returns nothing when the belt described cannot exist, so the caller can say
/// so rather than sending a search that silently finds nothing.
pub fn duplicates_filter(known: u32, duplicates: u32, active: bool) -> Option<Duplicates> {
    if known + duplicates > MAX_LEGACY {
        return None;
    }

    // The user switched the duplicate filter off, so they want the readable
    // modifiers and no constraint on the rest.
    if !active || duplicates == 0 {
        return Some(Duplicates::Plain);
    }

    if known + duplicates == MAX_LEGACY {
        return Some(match duplicates {
            // One readable modifier and three duplicates means all four slots
            // hold the same modifier, so it must be at full strength.
            3 => Duplicates::AtStrength(4),
            // Two or three readable modifiers. Each duplicate raises one of
            // them, and the count filter says how far.
            2 => Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }),
            1 => Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }),
            _ => Duplicates::AtStrength(1),
        });
    }

    // The belt has an empty slot as well as duplicates, so nothing about the
    // strengths is pinned down. One option above a floor is all that can be
    // asked.
    Some(Duplicates::Counted(CountFilter {
        at_least: 1,
        floors: vec![duplicates + 1],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_belt_with_no_duplicates_sends_its_readable_modifiers() {
        assert_eq!(duplicates_filter(4, 0, true), Some(Duplicates::Plain));
        assert_eq!(duplicates_filter(1, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn switching_the_duplicate_filter_off_sends_the_readable_modifiers() {
        // The user wants no constraint on what they cannot read.
        assert_eq!(duplicates_filter(2, 2, false), Some(Duplicates::Plain));
    }

    #[test]
    fn one_modifier_filling_all_four_slots_is_asked_for_at_full_strength() {
        assert_eq!(
            duplicates_filter(1, 3, true),
            Some(Duplicates::AtStrength(4))
        );
    }

    #[test]
    fn two_duplicates_alongside_two_readable_becomes_a_count_filter() {
        // The site gives no way to say "four modifiers, two the same", so the
        // duplicates become a count of options above floors.
        assert_eq!(
            duplicates_filter(2, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }))
        );
    }

    #[test]
    fn one_duplicate_alongside_three_readable_becomes_a_count_filter() {
        assert_eq!(
            duplicates_filter(3, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }))
        );
    }

    #[test]
    fn a_belt_with_an_empty_slot_asks_only_for_a_floor() {
        // Nothing about the strengths is pinned down, so one option above a
        // floor is all that can be asked.
        assert_eq!(
            duplicates_filter(1, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![2],
            }))
        );
        assert_eq!(
            duplicates_filter(1, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![3],
            }))
        );
    }

    #[test]
    fn a_belt_that_cannot_exist_reports_nothing() {
        // The site answers an empty result rather than an error, so the user
        // would see nothing and no reason why.
        assert_eq!(duplicates_filter(3, 3, true), None);
        assert_eq!(duplicates_filter(5, 0, true), None);
    }

    #[test]
    fn the_boundary_at_four_is_allowed() {
        assert!(duplicates_filter(4, 0, true).is_some());
        assert!(duplicates_filter(0, 4, true).is_some());
        assert!(duplicates_filter(4, 1, true).is_none());
    }

    #[test]
    fn a_belt_with_nothing_on_it_sends_nothing_special() {
        assert_eq!(duplicates_filter(0, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn every_count_filter_asks_for_at_least_one() {
        // A count of zero holds for every listing and narrows nothing.
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                assert!(count.at_least >= 1, "{known} known {duplicates} dup");
                assert!(!count.floors.is_empty(), "{known} known {duplicates} dup");
            }
        }
    }

    #[test]
    fn every_floor_is_within_what_a_belt_can_carry() {
        // A floor above four asks for a strength no modifier reaches.
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                for floor in count.floors {
                    assert!(
                        (1..=MAX_LEGACY).contains(&floor),
                        "{known} known {duplicates} dup gave floor {floor}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_valid_combination_is_left_without_an_answer() {
        // A combination that returns nothing when the belt is real would drop
        // the search with no explanation.
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=(MAX_LEGACY - known) {
                assert!(
                    duplicates_filter(known, duplicates, true).is_some(),
                    "{known} known {duplicates} dup"
                );
            }
        }
    }
}
