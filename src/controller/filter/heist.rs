//! Heist filters.
//!
//! Ported from `applyHeistRules`, `applyContractRules` and
//! `applyBlueprintRules` in Awakened PoE Trade's
//! `web/price-check/filters/pseudo/heist.ts`.
//!
//! # Why these are not in the other reference
//!
//! Heist is PoE1. Exiled Exchange 2 dropped the whole module.
//!
//! # What a heist item is bought for
//!
//! A contract is bought for one job at one level. Somebody levelling
//! Lockpicking wants a Lockpicking contract and nothing else, so a contract
//! searched without the job filter returns every contract in the game and
//! prices against all of them.
//!
//! A blueprint is bought for the enchant modifiers it will reveal, so one that
//! already has them is a different item at a different price. The reference
//! excludes those with a `not` group rather than filtering them in.

use crate::types::client_strings as cs;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{Range, StatFilter, StatGroup, StatGroupKind};

/// The stat reference the trade site files a blueprint's enchant count under.
pub const ENCHANT_MODS: &str = "# Enchant Modifiers";

/// Filters for a heist contract's job and target.
///
/// Enabled, unlike almost every other filter. A contract without its job is
/// not a narrower search, it is the wrong search.
pub fn contract_filters(item: &ParsedItem) -> Vec<StatFilter> {
    // A unique contract is bought for being that unique. Its job is incidental.
    if item.rarity == Some(ItemRarity::Unique) {
        return Vec::new();
    }

    let Some(contract) = &item.heist_contract else {
        return Vec::new();
    };

    let mut out = Vec::new();

    if let (Some(job), Some(level)) = (&contract.required_job, contract.job_level) {
        if let Some(id) = job_trade_id(job) {
            // A floor and no ceiling. A higher level of the same job still
            // does the work the buyer wants done.
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

/// The trade id for a job name.
///
/// None for anything that is not one of the nine. Sending an id the site does
/// not know fails the whole query, so an unknown job sends nothing.
pub fn job_trade_id(job: &str) -> Option<&'static str> {
    cs::HEIST_JOBS
        .iter()
        .find(|(name, _)| *name == job)
        .map(|(_, id)| *id)
}

/// The group excluding blueprints that already carry enchant modifiers.
///
/// A blueprint is bought for the enchants it will reveal. One that already has
/// them is a finished item at a different price, and leaving them in the
/// results makes the price read high.
///
/// Returns nothing when the item is not a plain blueprint, or when the item
/// already has enchant filters of its own, because then the buyer is pricing
/// the enchants rather than the blueprint.
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
        // Without it the search returns every contract in the game.
        let got = contract_filters(&contract("Lockpicking", 3));

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "item.heist_job_lockpicking");
    }

    #[test]
    fn the_job_level_is_a_floor_and_not_an_exact_match() {
        // A higher level of the same job still does the work.
        let got = contract_filters(&contract("Agility", 4));

        assert_eq!(got[0].range.min, Some(4.0));
        assert_eq!(got[0].range.max, None);
    }

    #[test]
    fn a_job_filter_is_on_by_default() {
        // A contract without its job is not a narrower search, it is the wrong
        // search.
        let got = contract_filters(&contract("Perception", 2));

        assert!(!got[0].disabled);
    }

    #[test]
    fn a_unique_contract_gets_no_job_filter() {
        // It is bought for being that unique. Its job is incidental.
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
        // An id the site does not know fails the whole query.
        let got = contract_filters(&contract("Juggling", 3));

        assert!(got.is_empty());
    }

    #[test]
    fn a_contract_with_no_level_sends_no_job_filter() {
        // A job with no level would send an unbounded filter, which is every
        // contract of that job at any level, including level one.
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

    // -----------------------------------------------------------------
    // Blueprints
    // -----------------------------------------------------------------

    fn blueprint() -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::HeistBlueprint),
            ..ParsedItem::default()
        }
    }

    #[test]
    fn a_blueprint_excludes_ones_that_already_have_enchants() {
        // It is bought for the enchants it will reveal. One that has them is a
        // finished item at a different price.
        let got = blueprint_exclusion(&blueprint(), false, Some("pseudo.x")).unwrap();

        assert_eq!(got.kind, StatGroupKind::Not);
        assert_eq!(got.filters[0].id, "pseudo.x");
    }

    #[test]
    fn a_blueprint_the_user_is_pricing_for_its_enchants_excludes_nothing() {
        // Then the enchants are the point and excluding them empties the
        // result.
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
        // Sending a group with no trade id fails the whole query.
        assert_eq!(blueprint_exclusion(&blueprint(), false, None), None);
    }
}
