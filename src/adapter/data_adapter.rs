//! What the domain needs from the outside world.
//!
//! This module declares traits and implements none of them. The parser needs
//! the stat and item tables and must not read a file, so it states the need
//! here and `poe-trader-app` supplies it.
//!
//! Split into two traits rather than one. The stat matcher needs only stats,
//! and a test for it should not have to invent an item table.

use crate::types::item::BaseInfo;
use crate::types::stat::StatHit;
use crate::types::GameVersion;

/// Finding a stat by one of its literal forms.
pub trait StatLookup {
    /// Find the stat whose matcher equals this template.
    ///
    /// The template already has its rolls replaced by `#`. Matching is exact,
    /// because a near match is a different stat with a different trade id.
    fn stat_by_matcher(&self, template: &str) -> Option<StatHit<'_>>;
}

/// Which table an item lookup should search.
///
/// The same name can exist in several tables. `The Doctor` is a divination
/// card and could also name a unique. Searching the wrong table returns the
/// wrong item and the price check is silently wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    Item,
    Unique,
    Gem,
    DivinationCard,
    CapturedBeast,
}

impl Namespace {
    /// The spelling used in the data files.
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Item => "ITEM",
            Namespace::Unique => "UNIQUE",
            Namespace::Gem => "GEM",
            Namespace::DivinationCard => "DIVINATION_CARD",
            Namespace::CapturedBeast => "CAPTURED_BEAST",
        }
    }

    /// Read the spelling used in the data files.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ITEM" => Some(Namespace::Item),
            "UNIQUE" => Some(Namespace::Unique),
            "GEM" => Some(Namespace::Gem),
            "DIVINATION_CARD" => Some(Namespace::DivinationCard),
            "CAPTURED_BEAST" => Some(Namespace::CapturedBeast),
            _ => None,
        }
    }
}

/// Finding an item base by name.
pub trait ItemLookup {
    /// Find every base with this name in this namespace.
    ///
    /// Returns several because one name can cover several bases that differ
    /// only in a property. A Two-Stone Ring is fire and cold or fire and
    /// lightning, and only the implicit tells them apart.
    fn items_by_name(&self, name: &str, namespace: Namespace, game: GameVersion) -> Vec<BaseInfo>;
}

/// Everything the parser needs from the outside world.
pub trait GameData: StatLookup + ItemLookup + Send + Sync {}

/// Anything that can do both lookups is game data.
impl<T: StatLookup + ItemLookup + Send + Sync> GameData for T {}

/// Game data that knows nothing.
///
/// Every lookup misses. It exists so a test can exercise the stages that need
/// no data without inventing a table, and so `ParserState` has a default.
///
/// A parse run against this produces an item with every stat unknown, which is
/// exactly what an empty data directory should look like.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoData;

impl StatLookup for NoData {
    fn stat_by_matcher(&self, _template: &str) -> Option<StatHit<'_>> {
        None
    }
}

impl ItemLookup for NoData {
    fn items_by_name(
        &self,
        _name: &str,
        _namespace: Namespace,
        _game: GameVersion,
    ) -> Vec<BaseInfo> {
        Vec::new()
    }
}

/// The shared instance of `NoData`.
pub static NO_DATA: NoData = NoData;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_namespace_round_trips() {
        for ns in [
            Namespace::Item,
            Namespace::Unique,
            Namespace::Gem,
            Namespace::DivinationCard,
            Namespace::CapturedBeast,
        ] {
            assert_eq!(Namespace::parse(ns.as_str()), Some(ns), "{ns:?}");
        }
    }

    #[test]
    fn an_unknown_namespace_is_rejected() {
        // A typo in a data file must fail loudly rather than silently
        // searching the wrong table.
        assert_eq!(Namespace::parse("Item"), None);
        assert_eq!(Namespace::parse(""), None);
    }

    #[test]
    fn no_data_misses_every_lookup() {
        assert!(NO_DATA.stat_by_matcher("# to maximum Life").is_none());
        assert!(NO_DATA
            .items_by_name("Chaos Orb", Namespace::Item, GameVersion::Poe2)
            .is_empty());
    }

    #[test]
    fn the_data_file_spellings_are_upper_case_with_underscores() {
        assert_eq!(Namespace::DivinationCard.as_str(), "DIVINATION_CARD");
        assert_eq!(Namespace::CapturedBeast.as_str(), "CAPTURED_BEAST");
    }
}
