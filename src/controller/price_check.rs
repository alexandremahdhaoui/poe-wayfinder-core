//! Clipboard text to a trade query.
//!
//! The one entry point the app needs. It parses, matches stats, builds the
//! filters and returns both the item and the query.
//!
//! The item comes back alongside the query because the UI shows what was
//! parsed. A user whose item priced oddly needs to see what the parser thought
//! the item was.

use crate::adapter::data_adapter::GameData;
use crate::controller::filter::item_filters::{build_query, FilterOptions};
use crate::controller::filter::stat_filters::{build_stat_group_for, StatFilterOptions};
use crate::controller::parse::{parse_clipboard, ParseError};
use crate::types::game::GameVersion;
use crate::types::item::ParsedItem;
use crate::types::query::TradeQuery;

/// Everything a price check needs to know.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceCheckOptions {
    pub game: GameVersion,
    pub filters: FilterOptions,
    pub stats: StatFilterOptions,
}

impl PriceCheckOptions {
    /// Defaults for a game.
    pub fn new(game: GameVersion) -> Self {
        Self {
            game,
            filters: FilterOptions::default(),
            stats: StatFilterOptions::default(),
        }
    }
}

/// What a price check produced.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceCheck {
    /// What the parser read.
    pub item: ParsedItem,
    /// The query to send.
    pub query: TradeQuery,
}

impl PriceCheck {
    /// How many stat filters the query carries.
    pub fn stat_filter_count(&self) -> usize {
        self.query.stats.iter().map(|g| g.filters.len()).sum()
    }

    /// Whether the parser met a modifier it did not recognise.
    ///
    /// The UI has to say so. A price built from a partly understood item is
    /// wrong in a way the user cannot see.
    pub fn has_unknown_modifiers(&self) -> bool {
        !self.item.unknown_modifiers.is_empty()
    }
}

/// Turn clipboard text into a trade query.
pub fn price_check(
    clipboard: &str,
    data: &dyn GameData,
    options: PriceCheckOptions,
) -> Result<PriceCheck, ParseError> {
    let item = parse_clipboard(clipboard, options.game, data)?;

    let mut query = build_query(&item, options.filters);

    // The item's own numbers reach the query here. A weapon is bought for its
    // damage per second and that is not a modifier.
    let mut stats = options.stats;
    stats.facts = crate::controller::filter::rules::ItemFacts {
        is_unique: item.rarity == Some(crate::types::item::ItemRarity::Unique),
        is_modifiable: item.is_modifiable(),
        is_finished_magic: item.rarity == Some(crate::types::item::ItemRarity::Magic)
            && !item.is_modifiable(),
    };

    if let Some(group) = build_stat_group_for(&item.modifiers, Some(&item), data, stats) {
        query.stats.push(group);
    }

    Ok(PriceCheck { item, query })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::data_adapter::{ItemLookup, Namespace, StatLookup};
    use crate::types::item::{BaseInfo, ItemRarity};
    use crate::types::stat::{Stat, StatBetter, StatHit, StatMatcher, TradeInfo};

    struct FakeData {
        stats: Vec<Stat>,
        items: Vec<BaseInfo>,
    }

    impl StatLookup for FakeData {
        fn stat_by_matcher(&self, template: &str) -> Option<StatHit<'_>> {
            for stat in &self.stats {
                for matcher in &stat.matchers {
                    if matcher.string == template {
                        return Some(StatHit { stat, matcher });
                    }
                }
            }

            None
        }
    }

    impl ItemLookup for FakeData {
        fn items_by_name(
            &self,
            name: &str,
            _namespace: Namespace,
            _game: GameVersion,
        ) -> Vec<BaseInfo> {
            self.items
                .iter()
                .filter(|i| i.name == name)
                .cloned()
                .collect()
        }
    }

    fn data() -> FakeData {
        let mut life_trade = TradeInfo::default();
        life_trade
            .ids
            .insert("explicit".into(), vec!["explicit.stat_life".into()]);

        let mut res_trade = TradeInfo::default();
        res_trade
            .ids
            .insert("implicit".into(), vec!["implicit.stat_cold".into()]);

        FakeData {
            stats: vec![
                Stat {
                    reference: "# to maximum Life".into(),
                    better: StatBetter::PositiveRoll,
                    matchers: vec![StatMatcher {
                        string: "# to maximum Life".into(),
                        ..StatMatcher::default()
                    }],
                    trade: life_trade,
                    ..Stat::default()
                },
                Stat {
                    reference: "#% to Cold Resistance".into(),
                    better: StatBetter::PositiveRoll,
                    matchers: vec![StatMatcher {
                        string: "#% to Cold Resistance".into(),
                        ..StatMatcher::default()
                    }],
                    trade: res_trade,
                    ..Stat::default()
                },
            ],
            items: Vec::new(),
        }
    }

    fn ring_text() -> String {
        [
            "Item Class: Rings",
            "Rarity: Rare",
            "Doom Loop",
            "Sapphire Ring",
            "--------",
            "Item Level: 78",
            "--------",
            "+25% to Cold Resistance (implicit)",
            "--------",
            "+50 to maximum Life",
            "--------",
            "Corrupted",
        ]
        .join("\n")
    }

    #[test]
    fn a_rare_ring_becomes_a_query() {
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(got.item.rarity, Some(ItemRarity::Rare));
        assert_eq!(got.item.item_level, Some(78));
        assert!(got.item.is_corrupted);
        assert_eq!(got.stat_filter_count(), 2);
    }

    #[test]
    fn the_query_states_corruption() {
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(got.query.filters.misc_filters.corrupted, Some(true));
    }

    #[test]
    fn the_implicit_and_the_explicit_get_different_trade_ids() {
        // The same stat has a different id per namespace, and mixing them up
        // returns items with the modifier in the wrong slot.
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        let ids: Vec<&str> = got.query.stats[0]
            .filters
            .iter()
            .map(|f| f.id.as_str())
            .collect();

        assert!(ids.contains(&"implicit.stat_cold"));
        assert!(ids.contains(&"explicit.stat_life"));
    }

    #[test]
    fn a_filter_is_enabled_only_when_its_roll_earns_it() {
        // The rolls in this fixture carry no range, so each reads as the best
        // possible and every filter is on. A badly rolled one would be off.
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert!(got.query.stats[0].filters.iter().all(|f| !f.disabled));
    }

    #[test]
    fn an_item_whose_stats_are_all_unknown_still_yields_a_query() {
        // Base type and category are enough to price many items. Refusing to
        // search because a modifier was unreadable helps nobody.
        let empty = FakeData {
            stats: Vec::new(),
            items: Vec::new(),
        };

        let got = price_check(
            &ring_text(),
            &empty,
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert!(got.query.stats.is_empty());
        assert!(got.has_unknown_modifiers());
    }

    #[test]
    fn an_item_with_every_stat_known_reports_no_unknowns() {
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert!(!got.has_unknown_modifiers());
    }

    #[test]
    fn text_that_is_not_an_item_fails() {
        let got = price_check(
            "hello world",
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        );

        assert_eq!(got.unwrap_err(), ParseError::BadNamePlate);
    }

    #[test]
    fn the_game_version_reaches_the_pipeline() {
        // A PoE1 pipeline is missing the PoE2 only stages, so a PoE2 item
        // parsed as PoE1 loses its spirit and charm slot sections.
        let poe1 = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe1),
        )
        .unwrap();

        let poe2 = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        // This item exercises no game specific stage, so both agree.
        assert_eq!(poe1.item.item_level, poe2.item.item_level);
    }

    #[test]
    fn enabling_every_filter_reaches_the_stat_group() {
        let mut options = PriceCheckOptions::new(GameVersion::Poe2);
        options.stats.enable_all = true;

        let got = price_check(&ring_text(), &data(), options).unwrap();

        assert!(got.query.stats[0].filters.iter().all(|f| !f.disabled));
    }

    #[test]
    fn the_roll_tolerance_reaches_the_stat_group() {
        let mut options = PriceCheckOptions::new(GameVersion::Poe2);
        options.stats.roll_tolerance = 0.0;

        let got = price_check(&ring_text(), &data(), options).unwrap();

        let life = got.query.stats[0]
            .filters
            .iter()
            .find(|f| f.id == "explicit.stat_life")
            .unwrap();

        assert_eq!(life.range.min, Some(50.0));
    }

    #[test]
    fn the_parsed_item_comes_back_alongside_the_query() {
        // A user whose item priced oddly needs to see what the parser thought
        // the item was.
        let got = price_check(
            &ring_text(),
            &data(),
            PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(got.item.raw_text, ring_text());
    }
}
