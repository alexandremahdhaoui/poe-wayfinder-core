use std::collections::BTreeMap;

use crate::types::modifier::ModifierType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatBetter {
    NegativeRoll,
    #[default]
    PositiveRoll,
    NotComparable,
}

impl StatBetter {
    pub fn from_i8(v: i8) -> Self {
        match v {
            -1 => StatBetter::NegativeRoll,
            1 => StatBetter::PositiveRoll,
            _ => StatBetter::NotComparable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatMatcher {
    pub string: String,
    pub advanced: Option<String>,
    pub negate: bool,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeInfo {
    pub inverted: bool,
    pub option: bool,
    pub count: bool,
    pub ids: BTreeMap<String, Vec<String>>,
}

impl TradeInfo {
    pub fn ids_for(&self, kind: ModifierType) -> Option<&[String]> {
        self.ids.get(kind.as_str()).map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stat {
    pub reference: String,
    pub decimals: bool,
    pub matchers: Vec<StatMatcher>,
    pub better: StatBetter,
    pub trade: TradeInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatHit<'a> {
    pub stat: &'a Stat,
    pub matcher: &'a StatMatcher,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatRoll {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub unscalable: bool,
    pub legacy: bool,
    pub decimals: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedStat {
    pub reference: String,
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
