//! The stat table.
//!
//! Ported from the `Stat` and `StatMatcher` interfaces in
//! `renderer/src/assets/data/interfaces.ts`.
//!
//! One stat is one idea, such as "adds maximum life". It has a canonical
//! reference form and a list of every literal string the game can print for
//! it, because the game prints a different sentence for a positive roll than
//! for a negative one and a different one again when the roll is exactly one.

use std::collections::BTreeMap;

use crate::types::modifier::ModifierType;

/// Which direction of roll is better for the buyer.
///
/// Resistance is better high. Attribute requirement is better low. Some stats
/// cannot be ordered at all, such as the name of a granted skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatBetter {
    NegativeRoll,
    #[default]
    PositiveRoll,
    NotComparable,
}

impl StatBetter {
    /// Read the integer form used in the data files.
    ///
    /// Anything else reads as not comparable, which makes the UI show the roll
    /// without an opinion rather than guessing the wrong direction.
    pub fn from_i8(v: i8) -> Self {
        match v {
            -1 => StatBetter::NegativeRoll,
            1 => StatBetter::PositiveRoll,
            _ => StatBetter::NotComparable,
        }
    }
}

/// One literal string the game can print for a stat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatMatcher {
    /// The template, with `#` for each roll.
    pub string: String,
    /// The Advanced Item Description form, when it differs.
    pub advanced: Option<String>,
    /// The game prints a reduction as a positive number for this form.
    ///
    /// `10% reduced Attack Speed` is the same stat as `-10% increased Attack
    /// Speed`. Missing this flips the sign on the whole filter.
    pub negate: bool,
    /// The value this form bakes into its text.
    ///
    /// `Adds 1 Passive Skill` carries no `#` because the value is always one.
    pub value: Option<f64>,
}

/// How a stat is expressed on the trade site.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeInfo {
    /// The trade site stores this stat with the opposite sign.
    pub inverted: bool,
    /// The filter takes a fixed option rather than a numeric range.
    pub option: bool,
    /// The filter counts occurrences rather than summing rolls.
    pub count: bool,
    /// Trade ids per modifier namespace.
    pub ids: BTreeMap<String, Vec<String>>,
}

impl TradeInfo {
    /// The ids for one namespace.
    pub fn ids_for(&self, kind: ModifierType) -> Option<&[String]> {
        self.ids.get(kind.as_str()).map(Vec::as_slice)
    }
}

/// One stat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stat {
    /// The canonical form, with `#` for each roll.
    pub reference: String,
    /// The roll carries decimals.
    pub decimals: bool,
    /// Every literal form the game can print.
    pub matchers: Vec<StatMatcher>,
    pub better: StatBetter,
    pub trade: TradeInfo,
}

/// A stat matched to one of its literal forms.
#[derive(Debug, Clone, PartialEq)]
pub struct StatHit<'a> {
    pub stat: &'a Stat,
    pub matcher: &'a StatMatcher,
}

/// A roll read off an item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatRoll {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    /// The value cannot be scaled by quality or by increase modifiers.
    pub unscalable: bool,
    /// The roll sits outside the range the game says is possible.
    ///
    /// That means the item predates a balance change. It cannot be crafted
    /// again and it is usually worth more, so the UI has to say so.
    pub legacy: bool,
    /// The roll carries decimals.
    pub decimals: bool,
    /// The fixed option a matcher baked into its text.
    pub option: Option<f64>,
}

impl Default for StatRoll {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 0.0,
            unscalable: false,
            legacy: false,
            decimals: false,
            option: None,
        }
    }
}

/// One stat line read off an item.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedStat {
    /// The canonical reference of the stat that matched.
    pub reference: String,
    /// The literal form that matched.
    pub matched: String,
    pub roll: Option<StatRoll>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_file_integers_map_to_the_right_direction() {
        assert_eq!(StatBetter::from_i8(1), StatBetter::PositiveRoll);
        assert_eq!(StatBetter::from_i8(-1), StatBetter::NegativeRoll);
        assert_eq!(StatBetter::from_i8(0), StatBetter::NotComparable);
    }

    #[test]
    fn an_unknown_direction_reads_as_not_comparable() {
        // Guessing a direction makes the UI recommend the wrong roll. Saying
        // nothing is the honest answer.
        assert_eq!(StatBetter::from_i8(7), StatBetter::NotComparable);
        assert_eq!(StatBetter::from_i8(-9), StatBetter::NotComparable);
    }

    #[test]
    fn trade_ids_are_looked_up_by_namespace() {
        let mut trade = TradeInfo::default();
        trade
            .ids
            .insert("explicit".into(), vec!["explicit.stat_1".into()]);

        assert_eq!(
            trade.ids_for(ModifierType::Explicit),
            Some(["explicit.stat_1".to_string()].as_slice())
        );
        assert_eq!(trade.ids_for(ModifierType::Implicit), None);
    }

    #[test]
    fn a_stat_defaults_to_positive_being_better() {
        // Most stats are. A default of not comparable would silence the UI on
        // almost every roll.
        assert_eq!(Stat::default().better, StatBetter::PositiveRoll);
    }

    #[test]
    fn a_roll_defaults_to_scalable_and_current() {
        let r = StatRoll::default();

        assert!(!r.unscalable);
        assert!(!r.legacy);
        assert_eq!(r.option, None);
    }
}
