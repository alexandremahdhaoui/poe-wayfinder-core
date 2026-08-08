use crate::types::client_strings as cs;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind};

pub const ENCHANT_MODS: &str = "# Enchant Modifiers";

pub fn contract_filters(item: &ParsedItem) -> Vec<StatFilter> {
    if item.rarity == Some(ItemRarity::Unique) {
        return Vec::new();
    }

    let Some(contract) = &item.heist_contract else {
        return Vec::new();
    };

    let mut out = Vec::new();

    if let (Some(job), Some(level)) = (&contract.required_job, contract.job_level) {
        if let Some(id) = job_trade_id(job) {
            out.push(StatFilter::range(
                id,
                Range {
                    min: Some(level as f64),
                    max: None,
                },
            ));
        }
    }

    if contract.priceless {
        let mut filter = StatFilter::range(cs::HEIST_TARGET_PRICELESS_ID, Range::default());
        filter.disabled = false;

        out.push(filter);
    }

    out
}

pub fn job_trade_id(job: &str) -> Option<&'static str> {
    cs::HEIST_JOBS
        .iter()
        .find(|(name, _)| *name == job)
        .map(|(_, id)| *id)
}

pub fn blueprint_exclusion(
    item: &ParsedItem,
    has_enchant_filters: bool,
    enchant_mods_id: Option<&str>,
) -> Option<StatGroup> {
    if item.category != Some(crate::types::category::ItemCategory::HeistBlueprint) {
        return None;
    }

    if item.rarity == Some(ItemRarity::Unique) || has_enchant_filters {
        return None;
    }

    let id = enchant_mods_id?;

    Some(StatGroup {
        kind: StatGroupKind::Not,
        value: Range::default(),
        filters: vec![StatFilter::range(id, Range::default())],
        disabled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;
    use crate::types::item::HeistContract;

    fn contract(job: &str, level: u32) -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::HeistContract),
            heist_contract: Some(HeistContract {
                required_job: Some(job.into()),
                job_level: Some(level),
                priceless: false,
            }),
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_contract_filters_on_its_job() {
        let got = contract_filters(&contract("Lockpicking", 3));

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "item.heist_job_lockpicking");
    }

    #[test]
    fn the_job_level_is_a_floor_and_not_an_exact_match() {
        let got = contract_filters(&contract("Agility", 4));

        assert_eq!(got[0].range.min, Some(4.0));
        assert_eq!(got[0].range.max, None);
    }

    #[test]
    fn a_job_filter_is_on_by_default() {
        let got = contract_filters(&contract("Perception", 2));

        assert!(!got[0].disabled);
    }

    #[test]
    fn a_unique_contract_gets_no_job_filter() {
        let mut item = contract("Perception", 2);
        item.rarity = Some(ItemRarity::Unique);

        assert!(contract_filters(&item).is_empty());
    }

    #[test]
    fn a_priceless_target_gets_its_own_filter() {
        let mut item = contract("Deception", 3);
        item.heist_contract.as_mut().unwrap().priceless = true;

        let got = contract_filters(&item);

        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|f| f.id == cs::HEIST_TARGET_PRICELESS_ID));
    }

    #[test]
    fn an_ordinary_target_gets_no_priceless_filter() {
        let got = contract_filters(&contract("Deception", 3));

        assert!(!got.iter().any(|f| f.id == cs::HEIST_TARGET_PRICELESS_ID));
    }

    #[test]
    fn a_job_the_site_does_not_index_sends_nothing() {
        let got = contract_filters(&contract("Juggling", 3));

        assert!(got.is_empty());
    }

    #[test]
    fn a_contract_with_no_level_sends_no_job_filter() {
        let mut item = contract("Agility", 3);
        item.heist_contract.as_mut().unwrap().job_level = None;

        assert!(contract_filters(&item).is_empty());
    }

    #[test]
    fn every_job_maps_to_an_id() {
        for (name, id) in cs::HEIST_JOBS {
            assert_eq!(job_trade_id(name), Some(id));
        }
    }

    fn blueprint() -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::HeistBlueprint),
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_blueprint_excludes_ones_that_already_have_enchants() {
        let got = blueprint_exclusion(&blueprint(), false, Some("pseudo.x")).unwrap();

        assert_eq!(got.kind, StatGroupKind::Not);
        assert_eq!(got.filters[0].id, "pseudo.x");
    }

    #[test]
    fn a_blueprint_the_user_is_pricing_for_its_enchants_excludes_nothing() {
        assert_eq!(
            blueprint_exclusion(&blueprint(), true, Some("pseudo.x")),
            None
        );
    }

    #[test]
    fn a_unique_blueprint_excludes_nothing() {
        let mut item = blueprint();
        item.rarity = Some(ItemRarity::Unique);

        assert_eq!(blueprint_exclusion(&item, false, Some("pseudo.x")), None);
    }

    #[test]
    fn a_non_blueprint_excludes_nothing() {
        let item = contract("Agility", 3);

        assert_eq!(blueprint_exclusion(&item, false, Some("pseudo.x")), None);
    }

    #[test]
    fn a_data_file_without_the_enchant_count_stat_excludes_nothing() {
        assert_eq!(blueprint_exclusion(&blueprint(), false, None), None);
    }
}
