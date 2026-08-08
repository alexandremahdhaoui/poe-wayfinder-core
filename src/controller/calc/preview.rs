use crate::types::category::ItemCategory;
use crate::types::item::{ArmourStats, WeaponStats};

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

pub fn rescale(value: Option<f64>, old_increase: f64, new_increase: f64) -> Option<f64> {
    let value = value?;

    if old_increase <= -100.0 {
        return Some(value);
    }

    let base = value / (1.0 + old_increase / 100.0);

    Some(base * (1.0 + new_increase / 100.0))
}

pub fn rescale_armour(
    armour: &mut ArmourStats,
    which: ArmourKind,
    old_increase: f64,
    new_increase: f64,
) {
    let field = match which {
        ArmourKind::Armour => &mut armour.ar,
        ArmourKind::Evasion => &mut armour.ev,
        ArmourKind::EnergyShield => &mut armour.es,
    };

    *field = rescale(*field, old_increase, new_increase);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmourKind {
    Armour,
    Evasion,
    EnergyShield,
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

    #[test]
    fn a_number_moves_with_its_increase() {
        assert_eq!(rescale(Some(200.0), 100.0, 150.0), Some(250.0));
    }

    #[test]
    fn a_number_with_no_change_stays_where_it_is() {
        assert_eq!(rescale(Some(200.0), 100.0, 100.0), Some(200.0));
    }

    #[test]
    fn a_number_the_item_does_not_have_stays_absent() {
        assert_eq!(rescale(None, 0.0, 50.0), None);
    }

    #[test]
    fn an_unmodified_number_scales_from_its_own_value() {
        assert_eq!(rescale(Some(100.0), 0.0, 50.0), Some(150.0));
    }

    #[test]
    fn a_total_reduction_does_not_divide_by_zero() {
        assert_eq!(rescale(Some(200.0), -100.0, 50.0), Some(200.0));
        assert_eq!(rescale(Some(200.0), -150.0, 50.0), Some(200.0));
    }

    #[test]
    fn rescaling_back_returns_the_original() {
        let once = rescale(Some(200.0), 100.0, 150.0).expect("a value rescales");

        assert_eq!(rescale(Some(once), 150.0, 100.0), Some(200.0));
    }

    #[test]
    fn only_the_defence_that_changed_moves() {
        let mut armour = ArmourStats {
            ar: Some(200.0),
            ev: Some(300.0),
            es: Some(50.0),
            ..ArmourStats::default()
        };

        rescale_armour(&mut armour, ArmourKind::Armour, 100.0, 150.0);

        assert_eq!(armour.ar, Some(250.0));
        assert_eq!(armour.ev, Some(300.0));
        assert_eq!(armour.es, Some(50.0));
    }

    #[test]
    fn each_defence_can_be_rescaled_on_its_own() {
        for which in [
            ArmourKind::Armour,
            ArmourKind::Evasion,
            ArmourKind::EnergyShield,
        ] {
            let mut armour = ArmourStats {
                ar: Some(100.0),
                ev: Some(100.0),
                es: Some(100.0),
                ..ArmourStats::default()
            };

            rescale_armour(&mut armour, which, 0.0, 100.0);

            let moved = [armour.ar, armour.ev, armour.es]
                .iter()
                .filter(|v| **v == Some(200.0))
                .count();

            assert_eq!(moved, 1, "{which:?}");
        }
    }
}
