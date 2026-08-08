use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind};

pub const INCREASED_EFFECT: &str = "#% increased effect";

pub const INCREASED_CHARGE_RECOVERY: &str = "#% increased Charge Recovery";

pub const VALDO_LETHAL_STATS: [&str; 1] = ["Players who Die in area are sent to the Void"];

pub fn not_group(ids: Vec<String>) -> Option<StatGroup> {
    if ids.is_empty() {
        return None;
    }

    Some(StatGroup {
        kind: StatGroupKind::Not,
        value: Range::default(),
        filters: ids
            .iter()
            .map(|id| StatFilter::range(id, Range::default()))
            .collect(),
        disabled: false,
    })
}

pub fn flask_excludes_increased_effect(item: &ParsedItem, references: &[String]) -> bool {
    if item.rarity != Some(ItemRarity::Magic) {
        return false;
    }

    let has = |wanted: &str| references.iter().any(|r| r == wanted);

    has(INCREASED_CHARGE_RECOVERY) && !has(INCREASED_EFFECT)
}

pub fn valdo_bad_mods(item: &ParsedItem, references: &[String]) -> Vec<&'static str> {
    if item.map_completion_reward.is_none() {
        return Vec::new();
    }

    VALDO_LETHAL_STATS
        .into_iter()
        .filter(|lethal| !references.iter().any(|r| r == lethal))
        .collect()
}

pub fn memory_strands_filter(item: &ParsedItem) -> Option<StatFilter> {
    let strands = item.memory_strands?;

    let mut filter = StatFilter::range(
        "item.memory_strands",
        Range {
            min: Some(strands as f64),
            max: None,
        },
    );

    filter.disabled = strands < 60;

    Some(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;

    fn flask(rarity: ItemRarity) -> ParsedItem {
        ParsedItem {
            rarity: Some(rarity),
            category: Some(ItemCategory::Flask),
            ..ParsedItem::default()
        }
    }

    fn refs(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_magic_flask_with_charge_recovery_excludes_increased_effect() {
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Magic),
            &refs(&[INCREASED_CHARGE_RECOVERY]),
        );

        assert!(got);
    }

    #[test]
    fn a_flask_that_already_has_the_effect_excludes_nothing() {
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Magic),
            &refs(&[INCREASED_CHARGE_RECOVERY, INCREASED_EFFECT]),
        );

        assert!(!got);
    }

    #[test]
    fn a_rare_flask_excludes_nothing() {
        let got = flask_excludes_increased_effect(
            &flask(ItemRarity::Rare),
            &refs(&[INCREASED_CHARGE_RECOVERY]),
        );

        assert!(!got);
    }

    #[test]
    fn a_flask_without_charge_recovery_excludes_nothing() {
        let got = flask_excludes_increased_effect(&flask(ItemRarity::Magic), &refs(&[]));

        assert!(!got);
    }

    fn valdo() -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::Map),
            map_completion_reward: Some("Headhunter".into()),
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_valdo_map_excludes_the_lethal_modifier_it_does_not_have() {
        let got = valdo_bad_mods(&valdo(), &refs(&[]));

        assert_eq!(got, VALDO_LETHAL_STATS.to_vec());
    }

    #[test]
    fn a_valdo_map_that_has_the_lethal_modifier_excludes_nothing() {
        let got = valdo_bad_mods(&valdo(), &refs(&VALDO_LETHAL_STATS));

        assert!(got.is_empty());
    }

    #[test]
    fn an_ordinary_map_excludes_nothing() {
        let mut item = valdo();
        item.map_completion_reward = None;

        assert!(valdo_bad_mods(&item, &refs(&[])).is_empty());
    }

    #[test]
    fn a_not_group_excludes_every_id_in_it() {
        let got = not_group(vec!["explicit.a".into(), "explicit.b".into()]).unwrap();

        assert_eq!(got.kind, StatGroupKind::Not);
        assert_eq!(got.filters.len(), 2);
    }

    #[test]
    fn an_empty_not_group_is_no_group() {
        assert_eq!(not_group(Vec::new()), None);
    }

    #[test]
    fn a_not_group_is_enabled() {
        let got = not_group(vec!["explicit.a".into()]).unwrap();

        assert!(!got.disabled);
        assert!(!got.filters[0].disabled);
    }

    fn amulet(strands: Option<u32>) -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::Amulet),
            memory_strands: strands,
            ..ParsedItem::default()
        }
    }

    #[test]
    fn an_accessory_with_strands_gets_a_filter() {
        let got = memory_strands_filter(&amulet(Some(80))).unwrap();

        assert_eq!(got.id, "item.memory_strands");
        assert_eq!(got.range.min, Some(80.0));
    }

    #[test]
    fn a_high_strand_count_is_on_by_default() {
        assert!(!memory_strands_filter(&amulet(Some(80))).unwrap().disabled);
    }

    #[test]
    fn a_low_strand_count_starts_off() {
        assert!(memory_strands_filter(&amulet(Some(20))).unwrap().disabled);
    }

    #[test]
    fn sixty_is_the_line() {
        assert!(memory_strands_filter(&amulet(Some(59))).unwrap().disabled);
        assert!(!memory_strands_filter(&amulet(Some(60))).unwrap().disabled);
    }

    #[test]
    fn an_accessory_with_no_strands_gets_no_filter() {
        assert_eq!(memory_strands_filter(&amulet(None)), None);
    }
}
