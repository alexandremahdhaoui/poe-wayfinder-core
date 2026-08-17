use crate::adapter::data_adapter::GameData;
use crate::controller::bulk::{endpoint_for, Endpoint, RouteFacts};
use crate::controller::filter::item_filters::{build_query, FilterOptions};
use crate::controller::filter::presets::{
    gem_level_filter, preset_for, trials_filter, uses_item_properties, uses_modifiers,
};
use crate::controller::filter::stat_filters::{build_stat_filters, StatFilterOptions};
use crate::controller::parse::{parse_clipboard, ParseError};
use crate::types::game::GameVersion;
use crate::types::item::ParsedItem;
use crate::types::query::TradeQuery;

#[derive(Debug, Clone, PartialEq)]
pub struct PriceCheckOptions {
    pub game: GameVersion,
    pub filters: FilterOptions,
    pub stats: StatFilterOptions,
}

impl PriceCheckOptions {
    pub fn new(game: GameVersion) -> Self {
        Self {
            game,
            filters: FilterOptions::default(),
            stats: StatFilterOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PriceCheck {
    pub item: ParsedItem,
    pub query: TradeQuery,
    pub endpoint: Endpoint,
    pub trade_tag: Option<String>,
    pub sources: Vec<crate::controller::filter::stat_filters::FilterSource>,
}

impl PriceCheck {
    pub fn constrains_something(&self) -> bool {
        self.query.name.is_some()
            || self.query.type_name.is_some()
            || self.query.stats.iter().any(|g| !g.filters.is_empty())
    }
}

impl PriceCheck {
    pub fn stat_filter_count(&self) -> usize {
        self.query.stats.iter().map(|g| g.filters.len()).sum()
    }

    pub fn has_unknown_modifiers(&self) -> bool {
        !self.item.unknown_modifiers.is_empty()
    }
}

pub fn price_check(
    clipboard: &str,
    data: &dyn GameData,
    options: &PriceCheckOptions,
) -> Result<PriceCheck, ParseError> {
    let item = parse_clipboard(clipboard, options.game, data)?;

    Ok(price_check_item(item, data, options))
}

pub fn price_check_item(
    item: ParsedItem,
    data: &dyn GameData,
    options: &PriceCheckOptions,
) -> PriceCheck {
    let mut query = build_query(&item, options.filters);

    let mut stats = options.stats.clone();
    stats.facts = crate::controller::filter::rules::ItemFacts {
        is_unique: item.rarity == Some(crate::types::item::ItemRarity::Unique),
        is_modifiable: item.is_modifiable(),
        is_finished_magic: item.rarity == Some(crate::types::item::ItemRarity::Magic)
            && !item.is_modifiable(),
    };

    stats.roll_tolerance = crate::controller::filter::rules::tolerance_for(stats.facts);

    let tweaks = crate::controller::filter::common::final_filter_tweaks(
        item.category,
        item.rarity,
        &item.info.reference_name,
        crate::controller::filter::slots::empty_slot(&item).is_some(),
    );

    stats.hide_augments = tweaks.hide_augments;
    stats.hide_anointment =
        crate::controller::filter::unique::anointment(&item).is_some() && item.is_modifiable();
    stats.valuable_rooms = crate::controller::filter::unique::valuable_rooms(&item.atzoatl_rooms)
        .into_iter()
        .map(str::to_string)
        .collect();
    stats.cluster_jewel_rules = tweaks.cluster_jewel_rules;

    if tweaks.flask_rules {
        let references: Vec<String> = item
            .modifiers
            .iter()
            .flat_map(|modifier| &modifier.stats)
            .map(|stat| stat.reference.clone())
            .collect();

        stats.hide_flask_enchants =
            !crate::controller::filter::slots::flask_enchant_is_useful(&references);
    }

    if tweaks.show_empty_modifier {
        if let Some(group) = crate::controller::filter::slots::empty_modifier_group(&item, data) {
            query.stats.push(group);
        }
    }

    let preset = preset_for(&item);

    if let Some(range) = gem_level_filter(&item, preset) {
        query.filters.misc_filters.gem_level = range;
    }

    {
        use crate::controller::filter::unique::{link_filter, unique_level_filter, unique_search};

        let search = unique_search(&item);

        if let Some(range) = unique_level_filter(&item, search) {
            query.filters.type_filters.ilvl = range;
        }

        if let Some(range) = link_filter(&item, search) {
            query.filters.misc_filters.gem_sockets = range;
        }
    }

    if let Some(range) = trials_filter(&item, preset) {
        query.filters.map_filters.map_revives = range;
    }

    if let Some(range) =
        crate::controller::filter::presets::ascendancy_area_level_filter(&item, preset)
    {
        query.filters.misc_filters.area_level = range;
    }

    let mut sources = Vec::new();

    if uses_modifiers(preset) {
        let properties = uses_item_properties(preset).then_some(&item);

        let built = build_stat_filters(&item.modifiers, properties, data, &stats);

        query.stats.extend(built.and);
        query.stats.extend(built.counts);
        sources = built.sources;
    }

    crate::controller::filter::mageblood::apply_legacy_rules(&item, &sources, &mut query);

    apply_heist_rules(&item, data, &mut query);
    apply_exclusions(&item, data, &mut query);

    if let Some(filter) = crate::controller::filter::exclusions::memory_strands_filter(&item) {
        query
            .stats
            .push(crate::types::query::StatGroup::all(vec![filter]));
    }

    let route = RouteFacts {
        trade_tag: item.info.trade_tag.clone(),
        category: item.category,
        stack_size_active: false,
        has_stack_size_filter: false,
        any_stat_enabled: query
            .stats
            .iter()
            .flat_map(|group| &group.filters)
            .any(|filter| !filter.disabled),
    };

    let endpoint = endpoint_for(&route);

    PriceCheck {
        trade_tag: match endpoint {
            Endpoint::Exchange => item.info.trade_tag.clone(),
            Endpoint::Search => None,
        },
        item,
        query,
        endpoint,
        sources,
    }
}

fn apply_exclusions(item: &ParsedItem, data: &dyn GameData, query: &mut TradeQuery) {
    use crate::controller::filter::exclusions::{
        flask_excludes_increased_effect, not_group, valdo_bad_mods, INCREASED_EFFECT,
    };

    let references: Vec<String> = item
        .modifiers
        .iter()
        .flat_map(|m| &m.stats)
        .map(|s| s.reference.clone())
        .collect();

    let mut wanted: Vec<&str> = Vec::new();

    if flask_excludes_increased_effect(item, &references) {
        wanted.push(INCREASED_EFFECT);
    }

    wanted.extend(valdo_bad_mods(item, &references));

    let ids: Vec<String> = wanted
        .into_iter()
        .filter_map(|reference| {
            data.stat_by_matcher(reference)
                .and_then(|hit| {
                    hit.stat
                        .trade
                        .ids_for(crate::types::modifier::ModifierType::Explicit)
                })
                .and_then(|ids| ids.first())
                .cloned()
        })
        .collect();

    if let Some(group) = not_group(ids) {
        query.stats.push(group);
    }
}

fn apply_heist_rules(item: &ParsedItem, data: &dyn GameData, query: &mut TradeQuery) {
    use crate::controller::filter::heist::{blueprint_exclusion, contract_filters, ENCHANT_MODS};

    let filters = contract_filters(item);

    if !filters.is_empty() {
        query
            .stats
            .push(crate::types::query::StatGroup::all(filters));
    }

    let enchant_id = data
        .stat_by_matcher(ENCHANT_MODS)
        .and_then(|hit| {
            hit.stat
                .trade
                .ids_for(crate::types::modifier::ModifierType::Pseudo)
        })
        .and_then(|ids| ids.first())
        .cloned();

    let has_enchant_filters = query
        .stats
        .iter()
        .flat_map(|group| &group.filters)
        .any(|filter| filter.id.starts_with("enchant."));

    if let Some(group) = blueprint_exclusion(item, has_enchant_filters, enchant_id.as_deref()) {
        query.stats.push(group);
    }
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
            &PriceCheckOptions::new(GameVersion::Poe2),
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
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(got.query.filters.misc_filters.corrupted, Some(true));
    }

    #[test]
    fn the_implicit_and_the_explicit_get_different_trade_ids() {
        let got = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
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
        let got = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert!(got.query.stats[0].filters.iter().all(|f| !f.disabled));
    }

    #[test]
    fn an_item_whose_stats_are_all_unknown_still_yields_a_query() {
        let empty = FakeData {
            stats: Vec::new(),
            items: Vec::new(),
        };

        let got = price_check(
            &ring_text(),
            &empty,
            &PriceCheckOptions::new(GameVersion::Poe2),
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
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert!(!got.has_unknown_modifiers());
    }

    #[test]
    fn text_that_is_not_an_item_fails() {
        let got = price_check(
            "hello world",
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        );

        assert_eq!(got.unwrap_err(), ParseError::BadNamePlate);
    }

    #[test]
    fn the_game_version_reaches_the_pipeline() {
        let poe1 = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe1),
        )
        .unwrap();

        let poe2 = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(poe1.item.item_level, poe2.item.item_level);
    }

    #[test]
    fn enabling_every_filter_reaches_the_stat_group() {
        let mut options = PriceCheckOptions::new(GameVersion::Poe2);
        options.stats.enable_all = true;

        let got = price_check(&ring_text(), &data(), &options).unwrap();

        assert!(got.query.stats[0].filters.iter().all(|f| !f.disabled));
    }

    #[test]
    fn the_roll_tolerance_reaches_the_stat_group() {
        let mut options = PriceCheckOptions::new(GameVersion::Poe2);
        options.stats.roll_tolerance = 0.0;

        let got = price_check(&ring_text(), &data(), &options).unwrap();

        let life = got.query.stats[0]
            .filters
            .iter()
            .find(|f| f.id == "explicit.stat_life")
            .unwrap();

        assert_eq!(life.range.min, Some(50.0));
    }

    #[test]
    fn the_parsed_item_comes_back_alongside_the_query() {
        let got = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .unwrap();

        assert_eq!(got.item.raw_text, ring_text());
    }

    fn currency_text() -> String {
        [
            "Item Class: Stackable Currency",
            "Rarity: Currency",
            "Divine Orb",
            "--------",
            "Stack Size: 5/10",
            "--------",
            "Randomises the numeric values of the modifiers of an item",
        ]
        .join("\n")
    }

    fn data_with_bulk_currency() -> FakeData {
        let mut out = data();
        out.items.push(BaseInfo {
            name: "Divine Orb".into(),
            reference_name: "Divine Orb".into(),
            trade_tag: Some("divine".into()),
            ..BaseInfo::default()
        });

        out
    }

    #[test]
    fn a_rare_goes_to_the_search_endpoint() {
        let got = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .expect("the item parses");

        assert_eq!(got.endpoint, Endpoint::Search);
    }

    #[test]
    fn a_search_check_carries_no_bulk_tag() {
        let got = price_check(
            &ring_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .expect("the item parses");

        assert_eq!(got.trade_tag, None);
    }

    #[test]
    fn a_currency_with_a_bulk_tag_goes_to_the_exchange_endpoint() {
        let got = price_check(
            &currency_text(),
            &data_with_bulk_currency(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .expect("the item parses");

        assert_eq!(got.endpoint, Endpoint::Exchange);
        assert_eq!(got.trade_tag.as_deref(), Some("divine"));
    }

    #[test]
    fn a_currency_our_data_has_no_tag_for_goes_to_the_search_endpoint() {
        let got = price_check(
            &currency_text(),
            &data(),
            &PriceCheckOptions::new(GameVersion::Poe2),
        )
        .expect("the item parses");

        assert_eq!(got.endpoint, Endpoint::Search);
    }
}
