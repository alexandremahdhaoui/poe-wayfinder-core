use crate::adapter::data_adapter::StatLookup;
use crate::controller::aggregate::sum_stats_by_type;
use crate::controller::filter::item_property::{armour_filters, weapon_filters};
use crate::controller::filter::pseudo::pseudo_totals;
use crate::controller::filter::rules::{
    hidden_reason, should_enable, ItemFacts, GOOD_ROLL_THRESHOLD,
};
use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::modifier::ModifierType;
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind};
use crate::types::stat::{StatBetter, StatRoll};

#[derive(Debug, Clone, PartialEq)]
pub struct StatFilterOptions {
    pub roll_tolerance: f64,
    pub enable_all: bool,
    pub facts: ItemFacts,
    pub hide_augments: bool,
    pub cluster_jewel_rules: bool,
    pub hide_flask_enchants: bool,
    pub hide_anointment: bool,
    pub valuable_rooms: Vec<String>,
}

impl Default for StatFilterOptions {
    fn default() -> Self {
        Self {
            roll_tolerance: 0.1,
            enable_all: false,
            hide_augments: false,
            cluster_jewel_rules: false,
            hide_flask_enchants: false,
            hide_anointment: false,
            valuable_rooms: Vec::new(),
            facts: ItemFacts {
                is_unique: false,
                is_modifiable: true,
                is_finished_magic: false,
            },
        }
    }
}

pub fn build_stat_group(
    modifiers: &[ParsedModifier],
    data: &dyn StatLookup,
    options: &StatFilterOptions,
) -> Option<StatGroup> {
    build_stat_group_for(modifiers, None, data, options)
}

pub fn build_stat_group_for(
    modifiers: &[ParsedModifier],
    item: Option<&crate::types::item::ParsedItem>,
    data: &dyn StatLookup,
    options: &StatFilterOptions,
) -> Option<StatGroup> {
    build_stat_filters(modifiers, item, data, options).and
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterSource {
    pub id: String,
    pub text: String,
    pub roll: Option<StatRoll>,
    pub tier: Option<u32>,
    pub contributors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatFilters {
    pub and: Option<StatGroup>,
    pub counts: Vec<StatGroup>,
    pub sources: Vec<FilterSource>,
}

pub fn property_label(id: &str) -> String {
    match id {
        "item.armour" => "Armour",
        "item.evasion_rating" => "Evasion",
        "item.energy_shield" => "Energy Shield",
        "item.block" => "Block",
        "item.total_dps" => "DPS",
        "item.physical_dps" => "Physical DPS",
        "item.elemental_dps" => "Elemental DPS",
        "item.attack_speed" => "Attacks per Second",
        "item.crit_chance" => "Critical Chance",
        "item.rune_sockets" => "Rune Sockets",
        "item.spirit" => "Spirit",
        "item.reload_time" => "Reload Time",
        "item.base_percentile" => "Base Percentile",
        other => other,
    }
    .to_string()
}

pub fn build_stat_filters(
    modifiers: &[ParsedModifier],
    item: Option<&crate::types::item::ParsedItem>,
    data: &dyn StatLookup,
    options: &StatFilterOptions,
) -> StatFilters {
    let mut counts = Vec::new();
    let mut filters = Vec::new();
    let mut sources: Vec<FilterSource> = Vec::new();
    let totals = sum_stats_by_type(modifiers);

    if let Some(item) = item {
        let scaling = crate::controller::filter::item_property::Scaling {
            quality: f64::from(item.quality.unwrap_or(0)),
            modifiable: item.is_modifiable(),
            totals: &totals,
        };

        for property in armour_filters(item.armour, scaling)
            .into_iter()
            .chain(weapon_filters(item.category, item.weapon, scaling))
        {
            let roll = crate::types::stat::StatRoll {
                value: property.value,
                min: property.value,
                max: property.value,
                ..crate::types::stat::StatRoll::default()
            };

            let mut filter = StatFilter::range(
                property.id,
                bound_for(
                    roll,
                    crate::types::stat::StatBetter::PositiveRoll,
                    false,
                    options,
                ),
            );

            let unique = options.facts.is_unique;

            filter.disabled = !options.enable_all && (unique || !property.enabled);

            sources.push(FilterSource {
                id: property.id.to_string(),
                text: property_label(property.id),
                roll: Some(roll),
                tier: None,
                contributors: Vec::new(),
            });

            filters.push(filter);
        }

        filters.extend(influence_filters(
            &item.influences,
            data,
            options,
            &mut sources,
        ));
    }

    for pseudo in pseudo_totals(&totals) {
        let Some(hit) = data.stat_by_matcher(pseudo.reference) else {
            continue;
        };

        let Some(ids) = hit.stat.trade.ids_for(ModifierType::Pseudo) else {
            continue;
        };

        let Some(id) = ids.first() else {
            continue;
        };

        let mut filter = StatFilter::range(id, Range::default());
        filter.disabled = pseudo.disabled && !options.enable_all;
        filter.range = bound_for(
            pseudo.roll,
            hit.stat.better,
            hit.stat.trade.inverted,
            options,
        );

        sources.push(FilterSource {
            id: id.clone(),
            text: pseudo.reference.to_string(),
            roll: Some(pseudo.roll),
            tier: None,
            contributors: contributors_for(modifiers, pseudo.reference),
        });

        filters.push(filter);
    }

    let mut explicit_matches: Vec<(String, StatFilter, Option<StatRoll>)> = Vec::new();

    let mut wanted: Vec<String> = totals.iter().map(|t| t.reference.clone()).collect();

    if let Some(item) = item {
        use crate::controller::filter::item_property::{
            armour_stats, remove_used_stats, weapon_stats,
        };

        if !item.armour.is_empty() {
            remove_used_stats(&mut wanted, &armour_stats());
        }

        if !item.weapon.is_empty() {
            remove_used_stats(&mut wanted, &weapon_stats());
        }
    }

    for total in totals.iter().filter(|t| wanted.contains(&t.reference)) {
        let Some(matched) = matched_text(modifiers, &total.reference) else {
            continue;
        };

        let Some(built) = build_one(&matched, total.roll, total.kind, data, options) else {
            continue;
        };

        if total.kind == ModifierType::Explicit {
            explicit_matches.push((matched.clone(), built.filter.clone(), total.roll));
        }

        sources.push(FilterSource {
            id: built.filter.id.clone(),
            text: matched.clone(),
            roll: total.roll,
            tier: tier_for(modifiers, &total.reference),
            contributors: contributors_for(modifiers, &total.reference),
        });

        match built.alternates.is_empty() {
            true => filters.push(built.filter),
            false => counts.push(StatGroup {
                kind: StatGroupKind::Count,
                value: Range {
                    min: Some(1.0),
                    max: None,
                },
                disabled: built.filter.disabled,
                filters: std::iter::once(built.filter.clone())
                    .chain(built.alternates.iter().map(|id| {
                        let mut alt = built.filter.clone();
                        alt.id = id.clone();

                        alt
                    }))
                    .collect(),
            }),
        }
    }

    if let Some(item) = item {
        for fractured in missing_fractured_filters(item, modifiers, &explicit_matches, data) {
            sources.push(FilterSource {
                id: fractured.id.clone(),
                text: fractured.text.clone(),
                roll: fractured.roll,
                tier: None,
                contributors: Vec::new(),
            });

            filters.push(fractured.filter);
        }
    }

    let filters: Vec<StatFilter> = filters
        .into_iter()
        .filter(|filter| !is_property_id(&filter.id))
        .collect();

    StatFilters {
        and: (!filters.is_empty()).then(|| StatGroup {
            filters,
            ..StatGroup::all(Vec::new())
        }),
        counts,
        sources,
    }
}

pub fn is_property_id(id: &str) -> bool {
    id.starts_with("item.")
}

struct FracturedFilter {
    filter: StatFilter,
    id: String,
    text: String,
    roll: Option<StatRoll>,
}

fn missing_fractured_filters(
    item: &crate::types::item::ParsedItem,
    modifiers: &[ParsedModifier],
    explicit_matches: &[(String, StatFilter, Option<StatRoll>)],
    data: &dyn StatLookup,
) -> Vec<FracturedFilter> {
    if !item.is_fractured {
        return Vec::new();
    }

    if modifiers
        .iter()
        .any(|m| m.info.kind == Some(ModifierType::Fractured))
    {
        return Vec::new();
    }

    explicit_matches
        .iter()
        .filter_map(|(matched, filter, roll)| {
            let hit = data.stat_by_matcher(matched)?;

            let id = hit
                .stat
                .trade
                .ids_for(ModifierType::Fractured)
                .and_then(|ids| ids.first())?;

            let mut copy = filter.clone();
            copy.id = id.clone();
            copy.disabled = true;

            Some(FracturedFilter {
                filter: copy,
                id: id.clone(),
                text: matched.clone(),
                roll: *roll,
            })
        })
        .collect()
}

fn influence_filters(
    influences: &[crate::types::item::Influence],
    data: &dyn StatLookup,
    options: &StatFilterOptions,
    sources: &mut Vec<FilterSource>,
) -> Vec<StatFilter> {
    let mut out = Vec::new();

    for influence in crate::controller::filter::influence::filterable(influences) {
        let reference = crate::controller::filter::influence::stat_reference(*influence);

        let Some(hit) = data.stat_by_matcher(reference) else {
            continue;
        };

        let Some(id) = hit
            .stat
            .trade
            .ids_for(ModifierType::Pseudo)
            .and_then(|ids| ids.first())
        else {
            continue;
        };

        let mut filter = StatFilter::range(id, Range::default());
        filter.disabled = !options.enable_all;

        sources.push(FilterSource {
            id: id.clone(),
            text: reference.to_string(),
            roll: None,
            tier: None,
            contributors: Vec::new(),
        });

        out.push(filter);
    }

    out
}

fn tier_for(modifiers: &[ParsedModifier], reference: &str) -> Option<u32> {
    modifiers
        .iter()
        .find(|m| m.stats.iter().any(|s| s.reference == reference))
        .and_then(|m| m.info.tier)
}

fn contributors_for(modifiers: &[ParsedModifier], reference: &str) -> Vec<String> {
    let carrying: Vec<&ParsedModifier> = modifiers
        .iter()
        .filter(|m| m.stats.iter().any(|s| s.reference == reference))
        .collect();

    if carrying.len() < 2 {
        return Vec::new();
    }

    carrying
        .iter()
        .filter_map(|m| m.stats.iter().find(|s| s.reference == reference))
        .map(|s| s.matched.clone())
        .collect()
}

fn matched_text(modifiers: &[ParsedModifier], reference: &str) -> Option<String> {
    modifiers
        .iter()
        .flat_map(|m| &m.stats)
        .find(|s| s.reference == reference)
        .map(|s| s.matched.clone())
}

#[derive(Debug, Clone, PartialEq)]
struct BuiltFilter {
    filter: StatFilter,
    alternates: Vec<String>,
}

fn build_one(
    matched: &str,
    roll: Option<StatRoll>,
    kind: ModifierType,
    data: &dyn StatLookup,
    options: &StatFilterOptions,
) -> Option<BuiltFilter> {
    let hit = data.stat_by_matcher(matched)?;

    let lookup_kind = match kind {
        ModifierType::AddedAugment => ModifierType::Augment,
        other => other,
    };

    let ids = hit.stat.trade.ids_for(lookup_kind)?;
    let id = ids.first()?;
    let alternates: Vec<String> = ids.iter().skip(1).cloned().collect();

    let hidden = hidden_reason(matched, roll, options.facts);

    if hidden.is_some() {
        return None;
    }

    let mut filter = StatFilter::range(id, Range::default());

    let hidden_augment =
        options.hide_augments && crate::controller::filter::slots::is_hidden_by_default(kind);

    filter.disabled = !options.enable_all
        && (hidden_augment || !should_enable(roll, hit.stat.better, hidden, GOOD_ROLL_THRESHOLD));

    let Some(roll) = roll else {
        return Some(BuiltFilter { filter, alternates });
    };

    if hit.stat.trade.option {
        filter.option = roll.option.or(Some(roll.value));

        return Some(BuiltFilter { filter, alternates });
    }

    filter.range = bound_for(roll, hit.stat.better, hit.stat.trade.inverted, options);

    apply_category_rules(&mut filter, &hit.stat.reference, options);

    Some(BuiltFilter { filter, alternates })
}

fn apply_category_rules(filter: &mut StatFilter, reference: &str, options: &StatFilterOptions) {
    use crate::controller::filter::slots::{
        is_cluster_socket_stat, passive_bound, PassiveBound, ADDS_PASSIVES,
    };

    if options.hide_anointment && reference.starts_with("Allocates ") {
        filter.hidden = true;
        filter.disabled = true;

        return;
    }

    if !options.valuable_rooms.is_empty() && reference.ends_with(" (Open)") {
        let room = reference.trim_end_matches(" (Open)");

        filter.disabled = !options
            .valuable_rooms
            .iter()
            .any(|valuable| valuable == room);

        return;
    }

    if options.hide_flask_enchants && filter.id.starts_with("enchant.") {
        filter.hidden = true;
        filter.disabled = true;

        return;
    }

    if !options.cluster_jewel_rules {
        return;
    }

    if is_cluster_socket_stat(reference) {
        filter.hidden = true;
        filter.disabled = true;

        return;
    }

    if reference != ADDS_PASSIVES {
        return;
    }

    let Some(count) = filter.range.min.map(|value| value as u32) else {
        return;
    };

    filter.disabled = false;

    match passive_bound(count) {
        PassiveBound::UpTo(max) => filter.range.max = Some(f64::from(max)),
        PassiveBound::Exact => filter.range.max = filter.range.min,
        PassiveBound::AtLeast => filter.range.max = None,
        PassiveBound::AtMost => filter.range.min = None,
    }
}

fn bound_for(
    roll: StatRoll,
    better: StatBetter,
    inverted: bool,
    options: &StatFilterOptions,
) -> Range {
    let value = if inverted { -roll.value } else { roll.value };

    let effective = match (better, inverted) {
        (StatBetter::PositiveRoll, true) => StatBetter::NegativeRoll,
        (StatBetter::NegativeRoll, true) => StatBetter::PositiveRoll,
        (other, _) => other,
    };

    let widened = widen(value, options.roll_tolerance, effective);

    match effective {
        StatBetter::PositiveRoll => Range::at_least(widened),
        StatBetter::NegativeRoll => Range::at_most(widened),
        StatBetter::NotComparable => Range::default(),
    }
}

fn widen(value: f64, tolerance: f64, better: StatBetter) -> f64 {
    let magnitude = value.abs() * tolerance;

    let moved = match better {
        StatBetter::PositiveRoll => value - magnitude,
        StatBetter::NegativeRoll => value + magnitude,
        StatBetter::NotComparable => value,
    };

    match better {
        StatBetter::PositiveRoll => (moved * 10.0).floor() / 10.0,
        _ => (moved * 10.0).ceil() / 10.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::modifier::ModifierInfo;
    use crate::types::stat::{ParsedStat, Stat, StatHit, StatMatcher, TradeInfo};

    struct FakeStats {
        stats: Vec<Stat>,
    }

    impl StatLookup for FakeStats {
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

    fn stat_with(reference: &str, better: StatBetter, trade: TradeInfo) -> Stat {
        Stat {
            reference: reference.into(),
            better,
            matchers: vec![StatMatcher {
                string: reference.into(),
                ..StatMatcher::default()
            }],
            trade,
            ..Stat::default()
        }
    }

    fn trade_with(kind: &str, id: &str) -> TradeInfo {
        let mut t = TradeInfo::default();
        t.ids.insert(kind.into(), vec![id.into()]);

        t
    }

    fn life_table() -> FakeStats {
        FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade_with("explicit", "explicit.stat_3299347043"),
            )],
        }
    }

    fn modifier(kind: ModifierType, matched: &str, roll: Option<StatRoll>) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(kind),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: matched.into(),
                matched: matched.into(),
                roll,
            }],
        }
    }

    fn roll(value: f64) -> Option<StatRoll> {
        Some(StatRoll {
            value,
            min: value,
            max: value,
            ..StatRoll::default()
        })
    }

    fn influence_table() -> FakeStats {
        FakeStats {
            stats: vec![
                stat_with(
                    "Has Shaper Influence",
                    StatBetter::PositiveRoll,
                    trade_with("pseudo", "pseudo.pseudo_has_shaper_influence"),
                ),
                stat_with(
                    "Has Elder Influence",
                    StatBetter::PositiveRoll,
                    trade_with("pseudo", "pseudo.pseudo_has_elder_influence"),
                ),
            ],
        }
    }

    fn influenced(
        influences: Vec<crate::types::item::Influence>,
    ) -> crate::types::item::ParsedItem {
        crate::types::item::ParsedItem {
            influences,
            ..crate::types::item::ParsedItem::default()
        }
    }

    fn two_id_table() -> FakeStats {
        let mut trade = TradeInfo::default();
        trade.ids.insert(
            "explicit".into(),
            vec!["explicit.stat_a".into(), "explicit.stat_b".into()],
        );

        FakeStats {
            stats: vec![stat_with(
                "#% chance to Impale Enemies on Hit with Attacks",
                StatBetter::PositiveRoll,
                trade,
            )],
        }
    }

    fn impale() -> Vec<ParsedModifier> {
        vec![modifier(
            ModifierType::Explicit,
            "#% chance to Impale Enemies on Hit with Attacks",
            roll(25.0),
        )]
    }

    #[test]
    fn an_armour_mod_is_not_asked_for_twice_once_as_a_stat_and_once_as_the_total() {
        let mut item = crate::types::item::ParsedItem::default();

        item.armour.ar = Some(1000.0);

        let table = FakeStats {
            stats: vec![stat_with(
                "#% increased Armour",
                StatBetter::PositiveRoll,
                trade_with("explicit", "explicit.stat_armour_incr"),
            )],
        };

        let got = build_stat_filters(
            &[modifier(
                ModifierType::Explicit,
                "#% increased Armour",
                roll(30.0),
            )],
            Some(&item),
            &table,
            &StatFilterOptions::default(),
        );

        let asked: Vec<String> = got
            .and
            .map(|group| group.filters.iter().map(|f| f.id.clone()).collect())
            .unwrap_or_default();

        assert!(
            !asked.iter().any(|id| id == "explicit.stat_armour_incr"),
            "the armour total already covers it, and asking for both matches nothing"
        );
    }

    #[test]
    fn a_stat_that_is_not_a_defence_survives_on_an_armour_item() {
        let mut item = crate::types::item::ParsedItem::default();

        item.armour.ar = Some(1000.0);

        let got = build_stat_filters(
            &[modifier(ModifierType::Explicit, "# to maximum Life", roll(80.0))],
            Some(&item),
            &life_table(),
            &StatFilterOptions::default(),
        );

        let group = got.and.expect("a stat group");

        assert!(
            group
                .filters
                .iter()
                .any(|f| f.id == "explicit.stat_3299347043"),
            "life has nothing to do with armour and must still be asked for"
        );
    }

    #[test]
    fn a_cluster_jewel_with_four_passives_searches_four_to_five() {
        let mut filter = StatFilter::range("explicit.stat_x", Range::at_least(4.0));

        apply_category_rules(
            &mut filter,
            "Adds # Passive Skills",
            &StatFilterOptions {
                cluster_jewel_rules: true,
                ..StatFilterOptions::default()
            },
        );

        assert_eq!(filter.range.min, Some(4.0));
        assert_eq!(filter.range.max, Some(5.0), "four is worth five");
        assert!(!filter.disabled, "the passive count always matters");
    }

    #[test]
    fn a_cluster_jewel_with_five_passives_searches_exactly_five() {
        let mut filter = StatFilter::range("explicit.stat_x", Range::at_least(5.0));

        apply_category_rules(
            &mut filter,
            "Adds # Passive Skills",
            &StatFilterOptions {
                cluster_jewel_rules: true,
                ..StatFilterOptions::default()
            },
        );

        assert_eq!(filter.range.max, Some(5.0));
    }

    #[test]
    fn a_cluster_jewel_with_eight_passives_searches_at_most_eight() {
        let mut filter = StatFilter::range("explicit.stat_x", Range::at_least(8.0));

        apply_category_rules(
            &mut filter,
            "Adds # Passive Skills",
            &StatFilterOptions {
                cluster_jewel_rules: true,
                ..StatFilterOptions::default()
            },
        );

        assert_eq!(filter.range.min, None, "eight is a bad roll, so at most");
    }

    #[test]
    fn the_jewel_socket_count_is_hidden_because_it_never_varies() {
        let mut filter = StatFilter::range("explicit.stat_x", Range::at_least(2.0));

        apply_category_rules(
            &mut filter,
            "# Added Passive Skills are Jewel Sockets",
            &StatFilterOptions {
                cluster_jewel_rules: true,
                ..StatFilterOptions::default()
            },
        );

        assert!(filter.hidden & filter.disabled);
    }

    #[test]
    fn a_flask_enchant_is_hidden_unless_the_flask_gains_no_charges() {
        let mut filter = StatFilter::range("enchant.stat_x", Range::at_least(1.0));

        apply_category_rules(
            &mut filter,
            "#% increased Flask Effect",
            &StatFilterOptions {
                hide_flask_enchants: true,
                ..StatFilterOptions::default()
            },
        );

        assert!(filter.hidden & filter.disabled);
    }

    #[test]
    fn nothing_happens_to_a_filter_when_no_category_rule_applies() {
        let mut filter = StatFilter::range("explicit.stat_x", Range::at_least(4.0));
        let before = filter.clone();

        apply_category_rules(
            &mut filter,
            "Adds # Passive Skills",
            &StatFilterOptions::default(),
        );

        assert_eq!(filter, before);
    }

    #[test]
    fn a_stat_a_rune_granted_on_a_unique_starts_switched_off() {
        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade_with("rune", "rune.stat_3299347043"),
            )],
        };

        let options = StatFilterOptions {
            hide_augments: true,
            ..StatFilterOptions::default()
        };

        let got = build_stat_filters(
            &[modifier(ModifierType::Augment, "# to maximum Life", roll(80.0))],
            None,
            &table,
            &options,
        );

        let group = got.and.expect("a stat group");

        assert!(
            group.filters.iter().all(|f| f.disabled),
            "on a unique, a rune's own stat must not narrow the search"
        );
    }

    #[test]
    fn a_stat_a_rune_granted_on_a_rare_is_left_as_it_is() {
        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade_with("rune", "rune.stat_3299347043"),
            )],
        };

        let got = build_stat_filters(
            &[modifier(ModifierType::Augment, "# to maximum Life", roll(80.0))],
            None,
            &table,
            &StatFilterOptions::default(),
        );

        let group = got.and.expect("a stat group");

        assert!(
            group.filters.iter().any(|f| !f.disabled),
            "upstream only hides augments on uniques, so a rare keeps its rune stat"
        );
    }

    #[test]
    fn a_stat_the_item_rolled_itself_is_not_hidden_by_that_rule() {
        let got = build_stat_filters(
            &[modifier(ModifierType::Explicit, "# to maximum Life", roll(80.0))],
            None,
            &life_table(),
            &StatFilterOptions::default(),
        );

        let group = got.and.expect("a stat group");

        assert!(
            group.filters.iter().any(|f| !f.disabled),
            "a good explicit roll still narrows the search"
        );
    }

    #[test]
    fn a_stat_with_two_trade_ids_travels_as_a_count_group() {
        let got = build_stat_filters(
            &impale(),
            None,
            &two_id_table(),
            &StatFilterOptions::default(),
        );

        assert_eq!(got.counts.len(), 1);
        assert_eq!(got.counts[0].kind, StatGroupKind::Count);
    }

    #[test]
    fn a_count_group_needs_only_one_of_the_ids() {
        let got = build_stat_filters(
            &impale(),
            None,
            &two_id_table(),
            &StatFilterOptions::default(),
        );

        assert_eq!(got.counts[0].value.min, Some(1.0));
    }

    #[test]
    fn a_count_group_carries_every_id() {
        let got = build_stat_filters(
            &impale(),
            None,
            &two_id_table(),
            &StatFilterOptions::default(),
        );

        let ids: Vec<&str> = got.counts[0]
            .filters
            .iter()
            .map(|f| f.id.as_str())
            .collect();

        assert_eq!(ids, vec!["explicit.stat_a", "explicit.stat_b"]);
    }

    #[test]
    fn every_id_in_a_count_group_carries_the_same_roll() {
        let got = build_stat_filters(
            &impale(),
            None,
            &two_id_table(),
            &StatFilterOptions::default(),
        );

        let first = &got.counts[0].filters[0];

        for filter in &got.counts[0].filters {
            assert_eq!(filter.range, first.range);
        }
    }

    #[test]
    fn a_stat_with_two_ids_is_not_also_in_the_and_group() {
        let got = build_stat_filters(
            &impale(),
            None,
            &two_id_table(),
            &StatFilterOptions::default(),
        );

        assert_eq!(got.and, None);
    }

    #[test]
    fn a_stat_with_one_id_stays_in_the_and_group() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let got = build_stat_filters(&mods, None, &life_table(), &StatFilterOptions::default());

        assert!(got.counts.is_empty());
        assert_eq!(got.and.unwrap().filters.len(), 1);
    }

    #[test]
    fn an_influenced_base_gets_a_filter_on_its_influence() {
        let item = influenced(vec![crate::types::item::Influence::Shaper]);

        let group = build_stat_group_for(
            &[],
            Some(&item),
            &influence_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert_eq!(group.filters.len(), 1);
        assert_eq!(group.filters[0].id, "pseudo.pseudo_has_shaper_influence");
    }

    #[test]
    fn an_influence_filter_starts_off() {
        let item = influenced(vec![crate::types::item::Influence::Shaper]);

        let group = build_stat_group_for(
            &[],
            Some(&item),
            &influence_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert!(group.filters[0].disabled);
    }

    #[test]
    fn a_double_influenced_base_gets_both_filters() {
        let item = influenced(vec![
            crate::types::item::Influence::Shaper,
            crate::types::item::Influence::Elder,
        ]);

        let group = build_stat_group_for(
            &[],
            Some(&item),
            &influence_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert_eq!(group.filters.len(), 2);
    }

    #[test]
    fn an_uninfluenced_base_gets_no_influence_filter() {
        let item = influenced(vec![]);

        let group = build_stat_group_for(
            &[],
            Some(&item),
            &influence_table(),
            &StatFilterOptions::default(),
        );

        assert!(group.is_none(), "an empty group matches everything");
    }

    #[test]
    fn an_influence_the_data_does_not_know_sends_nothing() {
        let item = influenced(vec![crate::types::item::Influence::Hunter]);

        let group = build_stat_group_for(
            &[],
            Some(&item),
            &influence_table(),
            &StatFilterOptions::default(),
        );

        assert!(group.is_none());
    }

    fn fracture_table() -> FakeStats {
        let mut life = TradeInfo::default();
        life.ids
            .insert("explicit".into(), vec!["explicit.stat_life".into()]);
        life.ids
            .insert("fractured".into(), vec!["fractured.stat_life".into()]);

        let mut res = TradeInfo::default();
        res.ids
            .insert("explicit".into(), vec!["explicit.stat_res".into()]);
        res.ids
            .insert("fractured".into(), vec!["fractured.stat_res".into()]);

        FakeStats {
            stats: vec![
                stat_with("# to maximum Life", StatBetter::PositiveRoll, life),
                stat_with("#% to Fire Resistance", StatBetter::PositiveRoll, res),
            ],
        }
    }

    fn two_explicits() -> Vec<ParsedModifier> {
        vec![
            modifier(ModifierType::Explicit, "# to maximum Life", roll(79.0)),
            modifier(ModifierType::Explicit, "#% to Fire Resistance", roll(45.0)),
        ]
    }

    fn fractured_item(is_fractured: bool) -> crate::types::item::ParsedItem {
        crate::types::item::ParsedItem {
            is_fractured,
            ..crate::types::item::ParsedItem::default()
        }
    }

    #[test]
    fn a_fractured_item_offers_every_explicit_as_fractured_too() {
        let item = fractured_item(true);

        let group = build_stat_group_for(
            &two_explicits(),
            Some(&item),
            &fracture_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        let fractured: Vec<&StatFilter> = group
            .filters
            .iter()
            .filter(|f| f.id.starts_with("fractured."))
            .collect();

        assert_eq!(fractured.len(), 2);
        assert_eq!(group.filters.len(), 4);
    }

    #[test]
    fn an_unfractured_item_offers_no_fractured_filters() {
        let item = fractured_item(false);

        let group = build_stat_group_for(
            &two_explicits(),
            Some(&item),
            &fracture_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert!(!group.filters.iter().any(|f| f.id.starts_with("fractured.")));
    }

    #[test]
    fn an_item_whose_fractured_modifier_is_marked_gets_no_guesses() {
        let item = fractured_item(true);

        let mut modifiers = two_explicits();
        modifiers.push(modifier(
            ModifierType::Fractured,
            "# to maximum Life",
            roll(79.0),
        ));

        let group = build_stat_group_for(
            &modifiers,
            Some(&item),
            &fracture_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        let guessed = group
            .filters
            .iter()
            .filter(|f| f.id.starts_with("fractured."))
            .count();

        assert!(guessed <= 1, "{guessed} fractured filters were guessed");
    }

    #[test]
    fn a_guessed_fractured_filter_starts_off() {
        let item = fractured_item(true);

        let group = build_stat_group_for(
            &two_explicits(),
            Some(&item),
            &fracture_table(),
            &StatFilterOptions::default(),
        )
        .unwrap();

        for filter in group
            .filters
            .iter()
            .filter(|f| f.id.starts_with("fractured."))
        {
            assert!(filter.disabled, "{} is on by default", filter.id);
        }
    }

    #[test]
    fn a_stat_with_no_fractured_id_is_not_guessed_at() {
        let mut trade = TradeInfo::default();
        trade
            .ids
            .insert("explicit".into(), vec!["explicit.stat_life".into()]);

        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade,
            )],
        };

        let item = fractured_item(true);

        let group = build_stat_group_for(
            &[modifier(
                ModifierType::Explicit,
                "# to maximum Life",
                roll(79.0),
            )],
            Some(&item),
            &table,
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert!(!group.filters.iter().any(|f| f.id.starts_with("fractured.")));
    }

    #[test]
    fn a_matched_stat_becomes_a_filter_on_its_trade_id() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters.len(), 1);
        assert_eq!(group.filters[0].id, "explicit.stat_3299347043");
    }

    #[test]
    fn a_badly_rolled_modifier_starts_disabled() {
        let poor = Some(StatRoll {
            value: 55.0,
            min: 50.0,
            max: 100.0,
            ..StatRoll::default()
        });

        let mods = vec![modifier(ModifierType::Explicit, "# to maximum Life", poor)];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].disabled);
    }

    #[test]
    fn a_well_rolled_modifier_starts_enabled() {
        let good = Some(StatRoll {
            value: 98.0,
            min: 50.0,
            max: 100.0,
            ..StatRoll::default()
        });

        let mods = vec![modifier(ModifierType::Explicit, "# to maximum Life", good)];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert!(!group.filters[0].disabled);
    }

    #[test]
    fn a_uniques_fixed_stat_yields_no_filter_at_all() {
        let options = StatFilterOptions {
            facts: crate::controller::filter::rules::ItemFacts {
                is_unique: true,
                is_modifiable: false,
                is_finished_magic: false,
            },
            ..StatFilterOptions::default()
        };

        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        assert!(build_stat_group(&mods, &life_table(), &options).is_none());
    }

    #[test]
    fn filters_can_start_enabled() {
        let options = StatFilterOptions {
            enable_all: true,
            ..StatFilterOptions::default()
        };
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), &options).unwrap();

        assert!(!group.filters[0].disabled);
    }

    #[test]
    fn a_stat_that_is_better_high_gets_a_floor() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.min, Some(45.0));
        assert_eq!(group.filters[0].range.max, None);
    }

    #[test]
    fn a_stat_that_is_better_low_gets_a_ceiling() {
        let table = FakeStats {
            stats: vec![stat_with(
                "# to Strength Requirement",
                StatBetter::NegativeRoll,
                trade_with("explicit", "explicit.stat_req"),
            )],
        };
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to Strength Requirement",
            roll(100.0),
        )];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.max, Some(110.0));
        assert_eq!(group.filters[0].range.min, None);
    }

    #[test]
    fn a_stat_with_no_direction_gets_no_bound() {
        let table = FakeStats {
            stats: vec![stat_with(
                "Grants Skill: #",
                StatBetter::NotComparable,
                trade_with("explicit", "explicit.stat_skill"),
            )],
        };
        let mods = vec![modifier(
            ModifierType::Explicit,
            "Grants Skill: #",
            roll(3.0),
        )];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].range.is_empty());
    }

    #[test]
    fn an_inverted_stat_has_its_sign_and_its_direction_flipped() {
        let mut trade = trade_with("explicit", "explicit.stat_inv");
        trade.inverted = true;

        let table = FakeStats {
            stats: vec![stat_with(
                "#% increased Cost",
                StatBetter::PositiveRoll,
                trade,
            )],
        };
        let mods = vec![modifier(
            ModifierType::Explicit,
            "#% increased Cost",
            roll(20.0),
        )];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.max, Some(-18.0));
        assert_eq!(group.filters[0].range.min, None);
    }

    #[test]
    fn an_option_stat_sends_an_option_and_no_range() {
        let mut trade = trade_with("explicit", "explicit.stat_opt");
        trade.option = true;

        let table = FakeStats {
            stats: vec![stat_with("Allocates #", StatBetter::NotComparable, trade)],
        };
        let mods = vec![modifier(ModifierType::Explicit, "Allocates #", roll(42.0))];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].option, Some(42.0));
        assert!(group.filters[0].range.is_empty());
    }

    #[test]
    fn an_option_stat_prefers_the_matchers_baked_value() {
        let mut trade = trade_with("explicit", "explicit.stat_opt");
        trade.option = true;

        let table = FakeStats {
            stats: vec![stat_with("Allocates #", StatBetter::NotComparable, trade)],
        };

        let mods = vec![modifier(
            ModifierType::Explicit,
            "Allocates #",
            Some(StatRoll {
                value: 42.0,
                option: Some(7.0),
                ..StatRoll::default()
            }),
        )];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].option, Some(7.0));
    }

    #[test]
    fn a_stat_with_no_roll_becomes_a_presence_check() {
        let table = FakeStats {
            stats: vec![stat_with(
                "Cannot be Frozen",
                StatBetter::NotComparable,
                trade_with("explicit", "explicit.stat_freeze"),
            )],
        };
        let mods = vec![modifier(ModifierType::Explicit, "Cannot be Frozen", None)];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].range.is_empty());
        assert_eq!(group.filters[0].option, None);
    }

    #[test]
    fn the_filter_uses_the_namespace_the_modifier_came_from() {
        let mut trade = trade_with("explicit", "explicit.stat_life");
        trade
            .ids
            .insert("implicit".into(), vec!["implicit.stat_life".into()]);

        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade,
            )],
        };

        let explicit = build_stat_group(
            &[modifier(
                ModifierType::Explicit,
                "# to maximum Life",
                roll(50.0),
            )],
            &table,
            &StatFilterOptions::default(),
        )
        .unwrap();
        assert_eq!(explicit.filters[0].id, "explicit.stat_life");

        let implicit = build_stat_group(
            &[modifier(
                ModifierType::Implicit,
                "# to maximum Life",
                roll(50.0),
            )],
            &table,
            &StatFilterOptions::default(),
        )
        .unwrap();
        assert_eq!(implicit.filters[0].id, "implicit.stat_life");
    }

    #[test]
    fn an_added_rune_is_filed_under_rune() {
        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade_with("rune", "rune.stat_life"),
            )],
        };

        let group = build_stat_group(
            &[modifier(
                ModifierType::AddedAugment,
                "# to maximum Life",
                roll(50.0),
            )],
            &table,
            &StatFilterOptions::default(),
        )
        .unwrap();

        assert_eq!(group.filters[0].id, "rune.stat_life");
    }

    #[test]
    fn a_stat_with_no_id_in_its_namespace_is_dropped() {
        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade_with("explicit", "explicit.stat_life"),
            )],
        };

        let got = build_stat_group(
            &[modifier(
                ModifierType::Enchant,
                "# to maximum Life",
                roll(50.0),
            )],
            &table,
            &StatFilterOptions::default(),
        );

        assert!(got.is_none());
    }

    #[test]
    fn an_unknown_stat_is_dropped() {
        let got = build_stat_group(
            &[modifier(
                ModifierType::Explicit,
                "# to maximum Sanity",
                roll(50.0),
            )],
            &life_table(),
            &StatFilterOptions::default(),
        );

        assert!(got.is_none());
    }

    #[test]
    fn a_modifier_with_no_type_is_dropped() {
        let mods = vec![ParsedModifier {
            info: ModifierInfo::default(),
            stats: vec![ParsedStat {
                reference: "# to maximum Life".into(),
                matched: "# to maximum Life".into(),
                roll: roll(50.0),
            }],
        }];

        assert!(build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).is_none());
    }

    #[test]
    fn an_item_with_no_modifiers_yields_no_group() {
        assert!(build_stat_group(&[], &life_table(), &StatFilterOptions::default()).is_none());
    }

    #[test]
    fn two_modifiers_of_one_stat_become_one_filter_on_the_total() {
        let mods = vec![
            modifier(ModifierType::Explicit, "# to maximum Life", roll(20.0)),
            modifier(ModifierType::Crafted, "# to maximum Life", roll(15.0)),
        ];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters.len(), 1);
        assert_eq!(group.filters[0].range.min, Some(31.5));
    }

    #[test]
    fn several_modifiers_land_in_one_group() {
        let mut trade = trade_with("explicit", "explicit.stat_life");
        trade
            .ids
            .insert("implicit".into(), vec!["implicit.stat_life".into()]);

        let table = FakeStats {
            stats: vec![stat_with(
                "# to maximum Life",
                StatBetter::PositiveRoll,
                trade,
            )],
        };

        let mods = vec![
            modifier(ModifierType::Explicit, "# to maximum Life", roll(50.0)),
            modifier(ModifierType::Implicit, "# to maximum Life", roll(20.0)),
        ];

        let group = build_stat_group(&mods, &table, &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters.len(), 2);
    }

    #[test]
    fn a_zero_tolerance_pins_the_bound_to_the_roll() {
        let options = StatFilterOptions {
            roll_tolerance: 0.0,
            ..StatFilterOptions::default()
        };
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), &options).unwrap();

        assert_eq!(group.filters[0].range.min, Some(50.0));
    }

    #[test]
    fn a_negative_roll_widens_away_from_zero_in_the_helpful_direction() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(-50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), &StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.min, Some(-55.0));
    }
}
