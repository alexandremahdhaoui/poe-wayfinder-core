use crate::types::item::BaseInfo;
use crate::types::stat::StatHit;
use crate::types::GameVersion;

pub trait StatLookup {
    fn stat_by_matcher(&self, template: &str) -> Option<StatHit<'_>>;

    fn stat_count(&self) -> usize {
        0
    }
}

pub trait TradeStatLookup {
    fn trade_stat_exists(&self, text: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    Item,
    Unique,
    Gem,
    DivinationCard,
    CapturedBeast,
}

impl Namespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Item => "ITEM",
            Namespace::Unique => "UNIQUE",
            Namespace::Gem => "GEM",
            Namespace::DivinationCard => "DIVINATION_CARD",
            Namespace::CapturedBeast => "CAPTURED_BEAST",
        }
    }

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

pub trait ItemLookup {
    fn items_by_name(&self, name: &str, namespace: Namespace, game: GameVersion) -> Vec<BaseInfo>;

    fn augments(&self) -> &[crate::controller::filter::augments::Augment] {
        &[]
    }

    fn item_name_count(&self) -> usize {
        0
    }
}

pub trait GameData: StatLookup + ItemLookup + Send + Sync {}

impl<T: StatLookup + ItemLookup + Send + Sync> GameData for T {}

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
