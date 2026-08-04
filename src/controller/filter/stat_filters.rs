//! Turning matched stats into trade stat filters.
//!
//! Ported from `create-stat-filters.ts`.
//!
//! # Why most stats start disabled
//!
//! A rare has six modifiers and almost nobody wants all six. A query that
//! requires all six returns the one listing that is this exact item, or none.
//!
//! So every stat filter travels with the query and starts disabled. The user
//! enables the two or three that matter. The trade site shows the rest greyed
//! out, ready to click.

use crate::adapter::data_adapter::StatLookup;
use crate::controller::aggregate::sum_stats_by_type;
use crate::controller::filter::pseudo::pseudo_totals;
use crate::controller::filter::rules::{
    hidden_reason, should_enable, ItemFacts, GOOD_ROLL_THRESHOLD,
};
use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::modifier::ModifierType;
use crate::types::query::{Range, StatFilter, StatGroup};
use crate::types::stat::{StatBetter, StatRoll};

/// How to build the stat filters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatFilterOptions {
    /// Widen every roll by this fraction.
    pub roll_tolerance: f64,
    /// Start every filter enabled.
    ///
    /// Off by default. A query that requires every modifier returns the one
    /// listing that is this exact item.
    pub enable_all: bool,
    /// What the item is, for the enable and hide rules.
    ///
    /// Without it every filter arrives disabled and the user switches on six
    /// by hand, which is most of the work the tool exists to save.
    pub facts: ItemFacts,
}

impl Default for StatFilterOptions {
    fn default() -> Self {
        Self {
            roll_tolerance: 0.1,
            enable_all: false,
            facts: ItemFacts {
                is_unique: false,
                is_modifiable: true,
                is_finished_magic: false,
            },
        }
    }
}

/// Build one stat group from an item's modifiers.
///
/// Stats are totalled across every modifier that grants them before a filter
/// is built. A ring with life on a prefix and life from a craft has one life
/// filter for the total, not two filters for the halves. Filtering on either
/// half finds almost nothing, because the split across modifiers is random.
///
/// Returns nothing when no modifier maps to a trade id, because an empty group
/// is a filter that matches everything and only slows the search.
pub fn build_stat_group(
    modifiers: &[ParsedModifier],
    data: &dyn StatLookup,
    options: StatFilterOptions,
) -> Option<StatGroup> {
    let mut filters = Vec::new();

    let totals = sum_stats_by_type(modifiers);

    // Pseudo totals first. They are what almost every real search is about,
    // and the panel shows filters in the order they arrive.
    for pseudo in pseudo_totals(&totals) {
        let Some(hit) = data.stat_by_matcher(pseudo.reference) else {
            // Our stat table is older than the pseudo rules. Skipping is
            // right: sending a filter with no trade id fails the whole query.
            continue;
        };

        let Some(ids) = hit.stat.trade.ids_for(ModifierType::Pseudo) else {
            continue;
        };

        let Some(id) = ids.first() else {
            continue;
        };

        let mut filter = StatFilter::range(id, Range::default());
        // A pseudo total's own rule decides, not the caller's enable_all.
        // Turning every pseudo on at once returns the one listing that is this
        // exact item.
        filter.disabled = pseudo.disabled && !options.enable_all;
        filter.range = bound_for(
            pseudo.roll,
            hit.stat.better,
            hit.stat.trade.inverted,
            options,
        );

        filters.push(filter);
    }

    for total in totals {
        // The lookup is by the literal form the game printed, so the matched
        // text is carried back from the modifier that produced the reference.
        let Some(matched) = matched_text(modifiers, &total.reference) else {
            continue;
        };

        let Some(filter) = build_one(&matched, total.roll, total.kind, data, options) else {
            continue;
        };

        filters.push(filter);
    }

    if filters.is_empty() {
        return None;
    }

    Some(StatGroup {
        filters,
        ..StatGroup::all(Vec::new())
    })
}

/// The literal form the game printed for a stat.
///
/// The total carries the canonical reference. The data lookup is keyed by the
/// literal form, so the first modifier that produced this reference supplies
/// it.
fn matched_text(modifiers: &[ParsedModifier], reference: &str) -> Option<String> {
    modifiers
        .iter()
        .flat_map(|m| &m.stats)
        .find(|s| s.reference == reference)
        .map(|s| s.matched.clone())
}

/// Build one stat filter.
fn build_one(
    matched: &str,
    roll: Option<StatRoll>,
    kind: ModifierType,
    data: &dyn StatLookup,
    options: StatFilterOptions,
) -> Option<StatFilter> {
    let hit = data.stat_by_matcher(matched)?;

    // An added rune is filed under rune on the trade site. Looking up
    // "added-rune" finds nothing and the filter is silently dropped.
    let lookup_kind = match kind {
        ModifierType::AddedAugment => ModifierType::Augment,
        other => other,
    };

    let ids = hit.stat.trade.ids_for(lookup_kind)?;
    let id = ids.first()?;

    // A unique's fixed stat narrows nothing and fills the panel with a row
    // nobody clicks.
    let hidden = hidden_reason(matched, roll, options.facts);

    if hidden.is_some() {
        return None;
    }

    let mut filter = StatFilter::range(id, Range::default());

    // A modifier that rolled near the top of its range is why the item is
    // worth anything. One that rolled badly is noise, and enabling it excludes
    // every item that rolled it worse.
    filter.disabled =
        !options.enable_all && !should_enable(roll, hit.stat.better, hidden, GOOD_ROLL_THRESHOLD);

    let Some(roll) = roll else {
        // A stat with no roll is a presence check. It needs no range.
        return Some(filter);
    };

    // A stat the trade site stores as an option takes the option and no range.
    // Sending a range on one returns nothing.
    if hit.stat.trade.option {
        filter.option = roll.option.or(Some(roll.value));

        return Some(filter);
    }

    filter.range = bound_for(roll, hit.stat.better, hit.stat.trade.inverted, options);

    Some(filter)
}

/// Decide which end of the roll to constrain.
///
/// A resistance is better high, so the filter is a floor. An attribute
/// requirement is better low, so it is a ceiling. Constraining the wrong end
/// returns every item worse than this one.
fn bound_for(
    roll: StatRoll,
    better: StatBetter,
    inverted: bool,
    options: StatFilterOptions,
) -> Range {
    // The trade site stores some stats with the opposite sign, so the value it
    // will compare against is negated.
    let value = if inverted { -roll.value } else { roll.value };

    // Inverting also flips which end is better.
    let effective = match (better, inverted) {
        (StatBetter::PositiveRoll, true) => StatBetter::NegativeRoll,
        (StatBetter::NegativeRoll, true) => StatBetter::PositiveRoll,
        (other, _) => other,
    };

    let widened = widen(value, options.roll_tolerance, effective);

    match effective {
        StatBetter::PositiveRoll => Range::at_least(widened),
        StatBetter::NegativeRoll => Range::at_most(widened),
        // No direction means no useful bound. The filter is a presence check.
        StatBetter::NotComparable => Range::default(),
    }
}

/// Move a value away from the roll by the tolerance, in the helpful direction.
///
/// Rounded to one decimal so a value that renders with one decimal still
/// matches itself.
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

    #[test]
    fn a_matched_stat_becomes_a_filter_on_its_trade_id() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters.len(), 1);
        assert_eq!(group.filters[0].id, "explicit.stat_3299347043");
    }

    #[test]
    fn a_badly_rolled_modifier_starts_disabled() {
        // Enabling it excludes every item that rolled it worse, which is most
        // of them.
        let poor = Some(StatRoll {
            value: 55.0,
            min: 50.0,
            max: 100.0,
            ..StatRoll::default()
        });

        let mods = vec![modifier(ModifierType::Explicit, "# to maximum Life", poor)];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].disabled);
    }

    #[test]
    fn a_well_rolled_modifier_starts_enabled() {
        // It is why the item is worth anything, and leaving every filter off
        // means the user switches on six by hand.
        let good = Some(StatRoll {
            value: 98.0,
            min: 50.0,
            max: 100.0,
            ..StatRoll::default()
        });

        let mods = vec![modifier(ModifierType::Explicit, "# to maximum Life", good)];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert!(!group.filters[0].disabled);
    }

    #[test]
    fn a_uniques_fixed_stat_yields_no_filter_at_all() {
        // Filtering on it narrows nothing and fills the panel with a row
        // nobody clicks.
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

        assert!(build_stat_group(&mods, &life_table(), options).is_none());
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

        let group = build_stat_group(&mods, &life_table(), options).unwrap();

        assert!(!group.filters[0].disabled);
    }

    #[test]
    fn a_stat_that_is_better_high_gets_a_floor() {
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.min, Some(45.0));
        assert_eq!(group.filters[0].range.max, None);
    }

    #[test]
    fn a_stat_that_is_better_low_gets_a_ceiling() {
        // Constraining the wrong end returns every item worse than this one.
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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.max, Some(110.0));
        assert_eq!(group.filters[0].range.min, None);
    }

    #[test]
    fn a_stat_with_no_direction_gets_no_bound() {
        // Guessing a direction would filter out items that are better.
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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].range.is_empty());
    }

    #[test]
    fn an_inverted_stat_has_its_sign_and_its_direction_flipped() {
        // The trade site stores these with the opposite sign, so both the
        // value and which end is better have to flip together.
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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

        // Value becomes -20, direction becomes better low, so a ceiling.
        assert_eq!(group.filters[0].range.max, Some(-18.0));
        assert_eq!(group.filters[0].range.min, None);
    }

    #[test]
    fn an_option_stat_sends_an_option_and_no_range() {
        // Sending a range on an option stat returns nothing.
        let mut trade = trade_with("explicit", "explicit.stat_opt");
        trade.option = true;

        let table = FakeStats {
            stats: vec![stat_with("Allocates #", StatBetter::NotComparable, trade)],
        };
        let mods = vec![modifier(ModifierType::Explicit, "Allocates #", roll(42.0))];

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

        assert!(group.filters[0].range.is_empty());
        assert_eq!(group.filters[0].option, None);
    }

    #[test]
    fn the_filter_uses_the_namespace_the_modifier_came_from() {
        // The same stat has a different id as an implicit than as an explicit.
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
            StatFilterOptions::default(),
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
            StatFilterOptions::default(),
        )
        .unwrap();
        assert_eq!(implicit.filters[0].id, "implicit.stat_life");
    }

    #[test]
    fn an_added_rune_is_filed_under_rune() {
        // The trade site has no added-rune namespace. Looking one up finds
        // nothing and the filter is silently dropped.
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
            StatFilterOptions::default(),
        )
        .unwrap();

        assert_eq!(group.filters[0].id, "rune.stat_life");
    }

    #[test]
    fn a_stat_with_no_id_in_its_namespace_is_dropped() {
        // Sending a filter with no id makes the whole query fail.
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
            StatFilterOptions::default(),
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
            StatFilterOptions::default(),
        );

        assert!(got.is_none());
    }

    #[test]
    fn a_modifier_with_no_type_is_dropped() {
        // Without a type there is no namespace and no id to send.
        let mods = vec![ParsedModifier {
            info: ModifierInfo::default(),
            stats: vec![ParsedStat {
                reference: "# to maximum Life".into(),
                matched: "# to maximum Life".into(),
                roll: roll(50.0),
            }],
        }];

        assert!(build_stat_group(&mods, &life_table(), StatFilterOptions::default()).is_none());
    }

    #[test]
    fn an_item_with_no_modifiers_yields_no_group() {
        // An empty group matches everything and only slows the search.
        assert!(build_stat_group(&[], &life_table(), StatFilterOptions::default()).is_none());
    }

    #[test]
    fn two_modifiers_of_one_stat_become_one_filter_on_the_total() {
        // A ring with life on a prefix and life from a craft has 35 life. Two
        // filters for 20 and 15 find almost nothing, because the split across
        // modifiers is random.
        let mods = vec![
            modifier(ModifierType::Explicit, "# to maximum Life", roll(20.0)),
            modifier(ModifierType::Crafted, "# to maximum Life", roll(15.0)),
        ];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters.len(), 1);
        // 35 minus ten percent.
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

        let group = build_stat_group(&mods, &table, StatFilterOptions::default()).unwrap();

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

        let group = build_stat_group(&mods, &life_table(), options).unwrap();

        assert_eq!(group.filters[0].range.min, Some(50.0));
    }

    #[test]
    fn a_negative_roll_widens_away_from_zero_in_the_helpful_direction() {
        // A negative resistance is still better high, so the floor has to go
        // further negative and not closer to zero.
        let mods = vec![modifier(
            ModifierType::Explicit,
            "# to maximum Life",
            roll(-50.0),
        )];

        let group = build_stat_group(&mods, &life_table(), StatFilterOptions::default()).unwrap();

        assert_eq!(group.filters[0].range.min, Some(-55.0));
    }
}
