use crate::types::category::ItemCategory;
use crate::types::item::WeaponStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuneElement {
    Cold,
    Lightning,
    Fire,
}

pub fn rune_element(name: &str) -> Option<RuneElement> {
    let base = name
        .strip_prefix("Lesser ")
        .or_else(|| name.strip_prefix("Greater "))
        .unwrap_or(name);

    match base {
        "Glacial Rune" => Some(RuneElement::Cold),
        "Storm Rune" => Some(RuneElement::Lightning),
        "Desert Rune" => Some(RuneElement::Fire),
        _ => None,
    }
}

pub fn apply_elemental_rune(
    weapon: &mut WeaponStats,
    category: Option<ItemCategory>,
    element: Option<RuneElement>,
    values: &[f64],
) {
    if !category.is_some_and(ItemCategory::is_martial_weapon) {
        return;
    }

    if values.len() != 2 {
        return;
    }

    let adding = (values[0] + values[1]) / 2.0;

    add(&mut weapon.elemental, adding);

    match element {
        Some(RuneElement::Cold) => add(&mut weapon.cold, adding),
        Some(RuneElement::Lightning) => add(&mut weapon.lightning, adding),
        Some(RuneElement::Fire) => add(&mut weapon.fire, adding),
        None => {}
    }
}

fn add(field: &mut Option<f64>, amount: f64) {
    *field = Some(field.unwrap_or(0.0) + amount);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bow() -> WeaponStats {
        WeaponStats {
            physical: Some(100.0),
            ..WeaponStats::default()
        }
    }

    #[test]
    fn every_tier_of_a_rune_adds_the_same_element() {
        for name in [
            "Glacial Rune",
            "Lesser Glacial Rune",
            "Greater Glacial Rune",
        ] {
            assert_eq!(rune_element(name), Some(RuneElement::Cold), "{name}");
        }
    }

    #[test]
    fn each_rune_maps_to_its_own_element() {
        assert_eq!(rune_element("Storm Rune"), Some(RuneElement::Lightning));
        assert_eq!(rune_element("Desert Rune"), Some(RuneElement::Fire));
    }

    #[test]
    fn a_rune_that_adds_no_element_reports_none() {
        assert_eq!(rune_element("Iron Rune"), None);
        assert_eq!(rune_element(""), None);
    }

    #[test]
    fn a_rune_adds_the_average_of_its_range() {
        let mut weapon = bow();

        apply_elemental_rune(
            &mut weapon,
            Some(ItemCategory::Bow),
            Some(RuneElement::Cold),
            &[5.0, 15.0],
        );

        assert_eq!(weapon.cold, Some(10.0));
        assert_eq!(weapon.elemental, Some(10.0));
    }

    #[test]
    fn a_rune_adds_to_what_is_already_there() {
        let mut weapon = WeaponStats {
            cold: Some(20.0),
            elemental: Some(20.0),
            ..bow()
        };

        apply_elemental_rune(
            &mut weapon,
            Some(ItemCategory::Bow),
            Some(RuneElement::Cold),
            &[5.0, 15.0],
        );

        assert_eq!(weapon.cold, Some(30.0));
        assert_eq!(weapon.elemental, Some(30.0));
    }

    #[test]
    fn a_rune_moves_only_its_own_element() {
        let mut weapon = bow();

        apply_elemental_rune(
            &mut weapon,
            Some(ItemCategory::Bow),
            Some(RuneElement::Fire),
            &[5.0, 15.0],
        );

        assert_eq!(weapon.fire, Some(10.0));
        assert_eq!(weapon.cold, None);
        assert_eq!(weapon.lightning, None);
    }

    #[test]
    fn armour_is_left_alone() {
        let mut weapon = WeaponStats::default();

        apply_elemental_rune(
            &mut weapon,
            Some(ItemCategory::BodyArmour),
            Some(RuneElement::Cold),
            &[5.0, 15.0],
        );

        assert_eq!(weapon, WeaponStats::default());
    }

    #[test]
    fn an_item_with_no_category_is_left_alone() {
        let mut weapon = WeaponStats::default();

        apply_elemental_rune(&mut weapon, None, Some(RuneElement::Cold), &[5.0, 15.0]);

        assert_eq!(weapon, WeaponStats::default());
    }

    #[test]
    fn a_malformed_range_adds_nothing() {
        for values in [vec![5.0], vec![5.0, 10.0, 15.0], vec![]] {
            let mut weapon = bow();

            apply_elemental_rune(
                &mut weapon,
                Some(ItemCategory::Bow),
                Some(RuneElement::Cold),
                &values,
            );

            assert_eq!(weapon.cold, None, "{values:?}");
        }
    }

    #[test]
    fn an_unrecognised_element_still_moves_the_total() {
        let mut weapon = bow();

        apply_elemental_rune(&mut weapon, Some(ItemCategory::Bow), None, &[5.0, 15.0]);

        assert_eq!(weapon.elemental, Some(10.0));
        assert_eq!(weapon.cold, None);
    }

}
