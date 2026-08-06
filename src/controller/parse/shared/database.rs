//! Finding the item in the data tables.
//!
//! Ported from `normalizeName` and `findInDatabase` in
//! `renderer/src/parser/Parser.ts`.
//!
//! # Why this is two stages and not one
//!
//! The name the game prints is not always the name the data file uses. A
//! blighted map prints `Blighted Toxic Map` and the data file has `Toxic Map`.
//! A metamorph organ prints the monster's name and the data file has
//! `Metamorph Brain`.
//!
//! So the name is normalised first, then looked up. Merging the two would make
//! the lookup a pile of special cases.

use crate::adapter::data_adapter::Namespace;
use crate::controller::parse::{ParseError, ParserState};
use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{ItemRarity, MapBlighted};

/// Rewrite the name into the form the data file uses.
pub fn normalize_name(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    strip_blight_prefix(state);
    normalise_metamorph_organ(state);

    Ok(())
}

/// Take `Blighted ` or `Blight-ravaged ` off a map name.
///
/// The prefix is a property of the map and not part of its base, so it is
/// recorded on the item and removed from the name.
///
/// Only normal and rare maps carry it. A magic map's name is already rolled
/// and stripping a word off it would break the lookup.
fn strip_blight_prefix(state: &mut ParserState<'_>) {
    if !matches!(
        state.item.rarity,
        Some(ItemRarity::Normal) | Some(ItemRarity::Rare)
    ) {
        return;
    }

    // The base type carries the prefix when there is one. Otherwise the name
    // does, because a normal map prints no separate base type line.
    let target = match state.base_type.as_deref() {
        Some(base) => base.to_string(),
        None => state.name.clone(),
    };

    // Checked longest first. "Blight-ravaged " does not start with
    // "Blighted ", but checking the shorter one first is the kind of ordering
    // bug that is invisible until a specific map drops.
    let stripped = if let Some(rest) = target.strip_prefix(cs::MAP_BLIGHT_RAVAGED) {
        Some((rest.to_string(), MapBlighted::BlightRavaged))
    } else {
        target
            .strip_prefix(cs::MAP_BLIGHTED)
            .map(|rest| (rest.to_string(), MapBlighted::Blighted))
    };

    let Some((rest, blighted)) = stripped else {
        return;
    };

    state.item.map.blighted = Some(blighted);

    if state.base_type.is_some() {
        state.base_type = Some(rest);
    } else {
        state.name = rest;
    }
}

/// Turn a metamorph organ's monster name into its data file name.
///
/// The game prints `Ancient Guardian's Brain`. The data file has
/// `Metamorph Brain`, because the monster does not change the price.
fn normalise_metamorph_organ(state: &mut ParserState<'_>) {
    if state.item.category != Some(ItemCategory::MetamorphSample) {
        return;
    }

    for organ in cs::METAMORPH_ORGANS {
        if state.name.ends_with(organ) {
            state.name = format!("Metamorph {organ}");

            return;
        }
    }
}

/// Look the item up and fill in what the data file knows.
///
/// Missing the item is not an error. A base the data does not have still
/// prices by category and modifiers, and refusing to search because one table
/// is stale helps nobody.
pub fn find_in_database(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    let (namespace, key) = lookup_key(state);

    let found = state.data.items_by_name(&key, namespace, state.game);

    if let Some(base) = pick_variant(state, found) {
        // The category from the data file wins. The item class line names a
        // class, not a trade category, and the two do not line up.
        if let Some(category) = base.category {
            state.item.category = Some(category);
        }

        state.item.info = base;
    }

    // After the lookup, and outside it. A unique the table does not know yet
    // still printed its base, and that is still what the query needs. The
    // assignment above would otherwise overwrite it.
    remember_unique_base(state);

    Ok(())
}

/// Keep the base type a unique is built on.
///
/// The unique's own entry names the unique. The trade query needs the base, so
/// the line the game prints under the name is kept. Without it a unique
/// searches with its own name as the type and matches nothing.
fn remember_unique_base(state: &mut ParserState<'_>) {
    if state.item.rarity != Some(ItemRarity::Unique) || state.item.is_unidentified {
        return;
    }

    state.item.info.unique_base = state.base_type.clone();
}

/// Which table to search and with what name.
fn lookup_key(state: &ParserState<'_>) -> (Namespace, String) {
    let name = state.name.clone();
    let base_or_name = state.base_type.clone().unwrap_or_else(|| name.clone());

    match state.item.category {
        Some(ItemCategory::DivinationCard) => (Namespace::DivinationCard, name),
        Some(ItemCategory::CapturedBeast) => (Namespace::CapturedBeast, base_or_name),
        Some(c) if c.is_gem() => (Namespace::Gem, name),
        Some(ItemCategory::MetamorphSample) => (Namespace::Item, name),
        // Every voidstone is the same base. The game gives each a unique name
        // and the trade site files them all under one.
        Some(ItemCategory::Voidstone) => (Namespace::Item, "Charged Compass".to_string()),
        _ if state.item.rarity == Some(ItemRarity::Unique) && !state.item.is_unidentified => {
            (Namespace::Unique, name)
        }
        _ => (Namespace::Item, base_or_name),
    }
}

/// Pick one base when the name covers several.
///
/// A Two-Stone Ring is fire and cold or fire and lightning, and only the
/// implicit tells them apart. Until the implicits are compared, the first is
/// taken, which is what the reference does before its own variant pass.
fn pick_variant(
    _state: &ParserState<'_>,
    mut found: Vec<crate::types::item::BaseInfo>,
) -> Option<crate::types::item::BaseInfo> {
    if found.is_empty() {
        return None;
    }

    Some(found.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::data_adapter::{ItemLookup, StatLookup};
    use crate::types::item::BaseInfo;
    use crate::types::stat::StatHit;
    use crate::types::GameVersion;

    struct FakeItems {
        items: Vec<(Namespace, BaseInfo)>,
    }

    impl StatLookup for FakeItems {
        fn stat_by_matcher(&self, _template: &str) -> Option<StatHit<'_>> {
            None
        }
    }

    impl ItemLookup for FakeItems {
        fn items_by_name(
            &self,
            name: &str,
            namespace: Namespace,
            _game: GameVersion,
        ) -> Vec<BaseInfo> {
            self.items
                .iter()
                .filter(|(ns, i)| *ns == namespace && i.name == name)
                .map(|(_, i)| i.clone())
                .collect()
        }
    }

    fn base(name: &str, category: Option<ItemCategory>) -> BaseInfo {
        BaseInfo {
            name: name.into(),
            reference_name: name.into(),
            category,
            ..BaseInfo::default()
        }
    }

    fn table(items: Vec<(Namespace, BaseInfo)>) -> FakeItems {
        FakeItems { items }
    }

    fn state_with<'a>(data: &'a FakeItems, name: &str) -> ParserState<'a> {
        ParserState {
            name: name.into(),
            data,
            ..ParserState::default()
        }
    }

    // -----------------------------------------------------------------
    // Name normalisation
    // -----------------------------------------------------------------

    #[test]
    fn a_blighted_map_loses_its_prefix_and_records_the_flag() {
        let data = table(Vec::new());
        let mut s = state_with(&data, "Blighted Toxic Map");
        s.item.rarity = Some(ItemRarity::Normal);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.name, "Toxic Map");
        assert_eq!(s.item.map.blighted, Some(MapBlighted::Blighted));
    }

    #[test]
    fn a_blight_ravaged_map_is_not_read_as_merely_blighted() {
        // Checking the shorter prefix first is invisible until a specific map
        // drops, and then the lookup misses.
        let data = table(Vec::new());
        let mut s = state_with(&data, "Blight-ravaged Toxic Map");
        s.item.rarity = Some(ItemRarity::Normal);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.name, "Toxic Map");
        assert_eq!(s.item.map.blighted, Some(MapBlighted::BlightRavaged));
    }

    #[test]
    fn the_prefix_comes_off_the_base_type_when_there_is_one() {
        let data = table(Vec::new());
        let mut s = state_with(&data, "Doom Reach");
        s.base_type = Some("Blighted Toxic Map".into());
        s.item.rarity = Some(ItemRarity::Rare);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.base_type.as_deref(), Some("Toxic Map"));
        assert_eq!(s.name, "Doom Reach");
    }

    #[test]
    fn a_magic_map_keeps_its_name() {
        // A magic map's name is already rolled and stripping a word off it
        // would break the lookup.
        let data = table(Vec::new());
        let mut s = state_with(&data, "Blighted Toxic Map of Impotence");
        s.item.rarity = Some(ItemRarity::Magic);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.name, "Blighted Toxic Map of Impotence");
        assert_eq!(s.item.map.blighted, None);
    }

    #[test]
    fn a_map_with_no_prefix_is_left_alone() {
        let data = table(Vec::new());
        let mut s = state_with(&data, "Toxic Map");
        s.item.rarity = Some(ItemRarity::Normal);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.name, "Toxic Map");
        assert_eq!(s.item.map.blighted, None);
    }

    #[test]
    fn every_metamorph_organ_becomes_its_data_file_name() {
        // The monster does not change the price, so the data file has one
        // record per organ.
        for organ in ["Brain", "Eye", "Lung", "Heart", "Liver"] {
            let data = table(Vec::new());
            let mut s = state_with(&data, &format!("Ancient Guardian's {organ}"));
            s.item.category = Some(ItemCategory::MetamorphSample);

            normalize_name(&mut s).unwrap();

            assert_eq!(s.name, format!("Metamorph {organ}"), "{organ}");
        }
    }

    #[test]
    fn a_non_metamorph_ending_in_an_organ_word_is_left_alone() {
        let data = table(Vec::new());
        let mut s = state_with(&data, "Kaom's Heart");
        s.item.rarity = Some(ItemRarity::Unique);

        normalize_name(&mut s).unwrap();

        assert_eq!(s.name, "Kaom's Heart");
    }

    // -----------------------------------------------------------------
    // Lookup
    // -----------------------------------------------------------------

    #[test]
    fn a_rare_is_looked_up_by_its_base_type() {
        let data = table(vec![(
            Namespace::Item,
            base("Spine Bow", Some(ItemCategory::Bow)),
        )]);
        let mut s = state_with(&data, "Doom Fletch");
        s.base_type = Some("Spine Bow".into());
        s.item.rarity = Some(ItemRarity::Rare);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Spine Bow");
        assert_eq!(s.item.category, Some(ItemCategory::Bow));
    }

    #[test]
    fn a_normal_item_is_looked_up_by_its_name() {
        // It prints no separate base type line.
        let data = table(vec![(
            Namespace::Item,
            base("Spine Bow", Some(ItemCategory::Bow)),
        )]);
        let mut s = state_with(&data, "Spine Bow");
        s.item.rarity = Some(ItemRarity::Normal);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Spine Bow");
    }

    #[test]
    fn a_unique_is_looked_up_in_the_unique_table() {
        let data = table(vec![
            (
                Namespace::Unique,
                base("Kaom's Heart", Some(ItemCategory::BodyArmour)),
            ),
            (
                Namespace::Item,
                base("Kaom's Heart", Some(ItemCategory::Ring)),
            ),
        ]);
        let mut s = state_with(&data, "Kaom's Heart");
        s.base_type = Some("Glorious Plate".into());
        s.item.rarity = Some(ItemRarity::Unique);

        find_in_database(&mut s).unwrap();

        // The unique table won, not the item table.
        assert_eq!(s.item.category, Some(ItemCategory::BodyArmour));
    }

    #[test]
    fn a_unique_keeps_the_base_type_printed_under_its_name() {
        // The unique's own entry names the unique twice over. The base is only
        // in the clipboard, and the trade query needs it.
        let data = table(vec![(
            Namespace::Unique,
            base("Kaom's Heart", Some(ItemCategory::BodyArmour)),
        )]);
        let mut s = state_with(&data, "Kaom's Heart");
        s.base_type = Some("Glorious Plate".into());
        s.item.rarity = Some(ItemRarity::Unique);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.unique_base.as_deref(), Some("Glorious Plate"));
    }

    #[test]
    fn a_unique_the_data_does_not_know_still_keeps_its_base() {
        // A new league's uniques are missing from the table for a few days.
        // The base is still on the clipboard and still worth sending.
        let data = table(vec![]);
        let mut s = state_with(&data, "Brand New Unique");
        s.base_type = Some("Glorious Plate".into());
        s.item.rarity = Some(ItemRarity::Unique);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.unique_base.as_deref(), Some("Glorious Plate"));
    }

    #[test]
    fn a_rare_keeps_no_unique_base() {
        // The field decides how the query is typed. Setting it on a rare would
        // send the base as a unique's base and drop the rarity filter.
        let data = table(vec![(
            Namespace::Item,
            base("Glorious Plate", Some(ItemCategory::BodyArmour)),
        )]);
        let mut s = state_with(&data, "Vengeance Wall");
        s.base_type = Some("Glorious Plate".into());
        s.item.rarity = Some(ItemRarity::Rare);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.unique_base, None);
    }

    #[test]
    fn an_unidentified_unique_keeps_no_base_of_its_own() {
        // It is already looked up by its base, so its info is the base. Naming
        // it again as a unique base would search for a unique that has no name
        // yet.
        let data = table(vec![(
            Namespace::Item,
            base("Glorious Plate", Some(ItemCategory::BodyArmour)),
        )]);
        let mut s = state_with(&data, "Glorious Plate");
        s.item.rarity = Some(ItemRarity::Unique);
        s.item.is_unidentified = true;

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.unique_base, None);
    }

    #[test]
    fn an_unidentified_unique_is_looked_up_by_its_base_type() {
        // Its own name is not printed yet.
        let data = table(vec![(
            Namespace::Item,
            base("Glorious Plate", Some(ItemCategory::BodyArmour)),
        )]);
        let mut s = state_with(&data, "Glorious Plate");
        s.base_type = Some("Glorious Plate".into());
        s.item.rarity = Some(ItemRarity::Unique);
        s.item.is_unidentified = true;

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Glorious Plate");
    }

    #[test]
    fn a_divination_card_is_looked_up_in_its_own_table() {
        // The same name can exist as a unique. Searching the wrong table makes
        // the price check silently wrong.
        let data = table(vec![
            (Namespace::DivinationCard, base("The Doctor", None)),
            (
                Namespace::Unique,
                base("The Doctor", Some(ItemCategory::Ring)),
            ),
        ]);
        let mut s = state_with(&data, "The Doctor");
        s.item.category = Some(ItemCategory::DivinationCard);

        find_in_database(&mut s).unwrap();

        // The card has no category of its own, so the one set stays.
        assert_eq!(s.item.category, Some(ItemCategory::DivinationCard));
        assert_eq!(s.item.info.reference_name, "The Doctor");
    }

    #[test]
    fn a_gem_is_looked_up_in_the_gem_table() {
        let data = table(vec![(
            Namespace::Gem,
            base("Fireball", Some(ItemCategory::SkillGem)),
        )]);
        let mut s = state_with(&data, "Fireball");
        s.item.category = Some(ItemCategory::Gem);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Fireball");
    }

    #[test]
    fn a_captured_beast_is_looked_up_by_its_base_type() {
        let data = table(vec![(
            Namespace::CapturedBeast,
            base("Craicic Chimeral", None),
        )]);
        let mut s = state_with(&data, "Some Named Beast");
        s.base_type = Some("Craicic Chimeral".into());
        s.item.category = Some(ItemCategory::CapturedBeast);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Craicic Chimeral");
    }

    #[test]
    fn every_voidstone_looks_up_the_same_base() {
        // The game gives each a unique name and the trade site files them all
        // under one.
        let data = table(vec![(
            Namespace::Item,
            base("Charged Compass", Some(ItemCategory::Voidstone)),
        )]);
        let mut s = state_with(&data, "Decaying Voidstone");
        s.item.category = Some(ItemCategory::Voidstone);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "Charged Compass");
    }

    #[test]
    fn an_item_the_data_does_not_have_is_not_an_error() {
        // A base the data is missing still prices by category and modifiers.
        // Refusing to search because one table is stale helps nobody.
        let data = table(Vec::new());
        let mut s = state_with(&data, "Brand New Base");
        s.item.rarity = Some(ItemRarity::Rare);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.info.reference_name, "");
    }

    #[test]
    fn the_data_file_category_beats_whatever_was_set() {
        // The item class line names a class, not a trade category.
        let data = table(vec![(
            Namespace::Item,
            base("Spine Bow", Some(ItemCategory::Bow)),
        )]);
        let mut s = state_with(&data, "Spine Bow");
        s.item.category = Some(ItemCategory::Unknown);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.category, Some(ItemCategory::Bow));
    }

    #[test]
    fn a_base_with_no_category_leaves_the_current_one_alone() {
        let data = table(vec![(Namespace::Item, base("Chaos Orb", None))]);
        let mut s = state_with(&data, "Chaos Orb");
        s.item.category = Some(ItemCategory::Currency);

        find_in_database(&mut s).unwrap();

        assert_eq!(s.item.category, Some(ItemCategory::Currency));
    }

    #[test]
    fn the_first_variant_is_taken_when_a_name_covers_several() {
        let data = table(vec![
            (
                Namespace::Item,
                BaseInfo {
                    name: "Two-Stone Ring".into(),
                    reference_name: "Two-Stone Ring".into(),
                    trade_discriminator: Some("fire_cold".into()),
                    category: Some(ItemCategory::Ring),
                    ..BaseInfo::default()
                },
            ),
            (
                Namespace::Item,
                BaseInfo {
                    name: "Two-Stone Ring".into(),
                    reference_name: "Two-Stone Ring".into(),
                    trade_discriminator: Some("fire_lightning".into()),
                    category: Some(ItemCategory::Ring),
                    ..BaseInfo::default()
                },
            ),
        ]);
        let mut s = state_with(&data, "Two-Stone Ring");
        s.item.rarity = Some(ItemRarity::Normal);

        find_in_database(&mut s).unwrap();

        assert_eq!(
            s.item.info.trade_discriminator.as_deref(),
            Some("fire_cold")
        );
    }
}
