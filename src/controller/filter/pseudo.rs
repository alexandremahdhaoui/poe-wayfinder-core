use crate::controller::aggregate::StatTotal;
use crate::types::stat::StatRoll;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contributor {
    pub reference: &'static str,
    pub multiplier: f64,
    pub required: bool,
}

impl Contributor {
    const fn plain(reference: &'static str) -> Self {
        Self {
            reference,
            multiplier: 1.0,
            required: false,
        }
    }

    const fn scaled(reference: &'static str, multiplier: f64) -> Self {
        Self {
            reference,
            multiplier,
            required: false,
        }
    }

    const fn required(reference: &'static str) -> Self {
        Self {
            reference,
            multiplier: 1.0,
            required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PseudoRule {
    pub pseudo: &'static str,
    pub disabled: bool,
    pub stats: &'static [Contributor],
}

const FIRE_SOURCES: &[Contributor] = &[
    Contributor::plain("#% to All Resistances"),
    Contributor::plain("#% to all Elemental Resistances"),
    Contributor::plain("#% to Fire Resistance"),
    Contributor::plain("#% to Fire and Lightning Resistances"),
    Contributor::plain("#% to Fire and Cold Resistances"),
    Contributor::plain("#% to Fire and Chaos Resistances"),
];

const COLD_SOURCES: &[Contributor] = &[
    Contributor::plain("#% to All Resistances"),
    Contributor::plain("#% to all Elemental Resistances"),
    Contributor::plain("#% to Cold Resistance"),
    Contributor::plain("#% to Fire and Cold Resistances"),
    Contributor::plain("#% to Cold and Lightning Resistances"),
    Contributor::plain("#% to Cold and Chaos Resistances"),
];

const LIGHTNING_SOURCES: &[Contributor] = &[
    Contributor::plain("#% to All Resistances"),
    Contributor::plain("#% to all Elemental Resistances"),
    Contributor::plain("#% to Lightning Resistance"),
    Contributor::plain("#% to Fire and Lightning Resistances"),
    Contributor::plain("#% to Cold and Lightning Resistances"),
    Contributor::plain("#% to Lightning and Chaos Resistances"),
];

const CHAOS_SOURCES: &[Contributor] = &[
    Contributor::plain("#% to All Resistances"),
    Contributor::plain("#% to Chaos Resistance"),
    Contributor::plain("#% to Fire and Chaos Resistances"),
    Contributor::plain("#% to Cold and Chaos Resistances"),
    Contributor::plain("#% to Lightning and Chaos Resistances"),
];

const ELEMENTAL_TOTAL_SOURCES: &[Contributor] = &[
    Contributor::scaled("#% to All Resistances", 3.0),
    Contributor::scaled("#% to all Elemental Resistances", 3.0),
    Contributor::plain("#% to Fire Resistance"),
    Contributor::plain("#% to Cold Resistance"),
    Contributor::plain("#% to Lightning Resistance"),
    Contributor::scaled("#% to Fire and Lightning Resistances", 2.0),
    Contributor::scaled("#% to Fire and Cold Resistances", 2.0),
    Contributor::scaled("#% to Cold and Lightning Resistances", 2.0),
    Contributor::plain("#% to Fire and Chaos Resistances"),
    Contributor::plain("#% to Cold and Chaos Resistances"),
    Contributor::plain("#% to Lightning and Chaos Resistances"),
];

const STRENGTH_SOURCES: &[Contributor] = &[
    Contributor::plain("# to all Attributes"),
    Contributor::plain("# to Strength"),
    Contributor::plain("# to Strength and Intelligence"),
    Contributor::plain("# to Strength and Dexterity"),
];

const DEXTERITY_SOURCES: &[Contributor] = &[
    Contributor::plain("# to all Attributes"),
    Contributor::plain("# to Dexterity"),
    Contributor::plain("# to Strength and Dexterity"),
    Contributor::plain("# to Dexterity and Intelligence"),
];

const INTELLIGENCE_SOURCES: &[Contributor] = &[
    Contributor::plain("# to all Attributes"),
    Contributor::plain("# to Intelligence"),
    Contributor::plain("# to Strength and Intelligence"),
    Contributor::plain("# to Dexterity and Intelligence"),
];

const LIFE_SOURCES: &[Contributor] = &[
    Contributor::required("# to maximum Life"),
    Contributor::scaled("# to all Attributes", 2.0),
    Contributor::scaled("# to Strength", 2.0),
    Contributor::scaled("# to Strength and Intelligence", 2.0),
    Contributor::scaled("# to Strength and Dexterity", 2.0),
];

const MANA_SOURCES: &[Contributor] = &[
    Contributor::required("# to maximum Mana"),
    Contributor::scaled("# to all Attributes", 2.0),
    Contributor::scaled("# to Intelligence", 2.0),
    Contributor::scaled("# to Strength and Intelligence", 2.0),
    Contributor::scaled("# to Dexterity and Intelligence", 2.0),
];

pub const PSEUDO_RULES: &[PseudoRule] = &[
    PseudoRule {
        pseudo: "+#% total Elemental Resistance",
        disabled: false,
        stats: ELEMENTAL_TOTAL_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total maximum Life",
        disabled: false,
        stats: LIFE_SOURCES,
    },
    PseudoRule {
        pseudo: "+#% total to Fire Resistance",
        disabled: true,
        stats: FIRE_SOURCES,
    },
    PseudoRule {
        pseudo: "+#% total to Cold Resistance",
        disabled: true,
        stats: COLD_SOURCES,
    },
    PseudoRule {
        pseudo: "+#% total to Lightning Resistance",
        disabled: true,
        stats: LIGHTNING_SOURCES,
    },
    PseudoRule {
        pseudo: "+#% total to Chaos Resistance",
        disabled: true,
        stats: CHAOS_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total maximum Mana",
        disabled: true,
        stats: MANA_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total to Strength",
        disabled: true,
        stats: STRENGTH_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total to Dexterity",
        disabled: true,
        stats: DEXTERITY_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total to Intelligence",
        disabled: true,
        stats: INTELLIGENCE_SOURCES,
    },
    PseudoRule {
        pseudo: "+# total maximum Energy Shield",
        disabled: true,
        stats: &[Contributor::plain("# to maximum Energy Shield")],
    },
    PseudoRule {
        pseudo: "#% total increased maximum Energy Shield",
        disabled: true,
        stats: &[Contributor::plain("#% increased maximum Energy Shield")],
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct PseudoTotal {
    pub reference: &'static str,
    pub roll: StatRoll,
    pub disabled: bool,
    pub sources: usize,
}

pub fn pseudo_totals(totals: &[StatTotal]) -> Vec<PseudoTotal> {
    let mut out = Vec::new();

    for rule in PSEUDO_RULES {
        let mut value = 0.0;
        let mut min = 0.0;
        let mut max = 0.0;
        let mut sources = 0;
        let mut missing_required = false;

        for contributor in rule.stats {
            let found = totals
                .iter()
                .find(|t| signs_match(&t.reference, contributor.reference));

            let Some(total) = found else {
                if contributor.required {
                    missing_required = true;
                }

                continue;
            };

            let Some(roll) = total.roll else {
                continue;
            };

            value += roll.value * contributor.multiplier;
            min += roll.min * contributor.multiplier;
            max += roll.max * contributor.multiplier;
            sources += total.sources;
        }

        if missing_required || sources == 0 {
            continue;
        }

        out.push(PseudoTotal {
            reference: rule.pseudo,
            roll: StatRoll {
                value,
                min,
                max,
                ..StatRoll::default()
            },
            disabled: rule.disabled,
            sources,
        });
    }

    out
}

pub fn affects_pseudo(reference: &str) -> bool {
    if PSEUDO_RULES
        .iter()
        .flat_map(|rule| rule.stats)
        .any(|c| signs_match(c.reference, reference))
    {
        return true;
    }

    if ARMOUR_STATS.contains(&reference) || WEAPON_STATS.contains(&reference) {
        return true;
    }

    reference == "Adds # to # Chaos Damage"
}

fn signs_match(rule: &str, reference: &str) -> bool {
    rule == reference || strip_sign(rule) == strip_sign(reference)
}

fn strip_sign(reference: &str) -> String {
    reference.replace("+#", "#").replace("-#", "#")
}

pub const ARMOUR_STATS: &[&str] = &[
    "#% increased Armour",
    "#% increased Evasion Rating",
    "#% increased Energy Shield",
    "#% increased Armour and Evasion",
    "#% increased Armour and Energy Shield",
    "#% increased Evasion and Energy Shield",
    "#% increased Armour, Evasion and Energy Shield",
    "+# to Armour",
    "+# to Evasion Rating",
    "+# to maximum Energy Shield",
    "#% increased Block chance",
];

pub const WEAPON_STATS: &[&str] = &[
    "#% increased Physical Damage",
    "Adds # to # Physical Damage",
    "#% increased Attack Speed",
    "Adds # to # Fire Damage",
    "Adds # to # Cold Damage",
    "Adds # to # Lightning Damage",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::modifier::ModifierType;

    fn total(reference: &str, value: f64) -> StatTotal {
        StatTotal {
            reference: reference.to_string(),
            kind: ModifierType::Explicit,
            roll: Some(StatRoll {
                value,
                min: value,
                max: value,
                ..StatRoll::default()
            }),
            sources: 1,
        }
    }

    fn find<'a>(totals: &'a [PseudoTotal], reference: &str) -> Option<&'a PseudoTotal> {
        totals.iter().find(|t| t.reference == reference)
    }

    #[test]
    fn the_poe1_spelling_of_a_contributor_still_feeds_its_total() {
        let got = pseudo_totals(&[total("+#% to Fire Resistance", 30.0)]);

        assert_eq!(
            find(&got, "+#% total to Fire Resistance")
                .unwrap()
                .roll
                .value,
            30.0
        );
    }

    #[test]
    fn the_poe1_spelling_satisfies_a_required_contributor() {
        let got = pseudo_totals(&[
            total("+# to maximum Life", 79.0),
            total("+# to Strength", 20.0),
        ]);

        assert_eq!(
            find(&got, "+# total maximum Life").unwrap().roll.value,
            79.0 + 40.0
        );
    }

    #[test]
    fn a_missing_life_modifier_is_still_missing_in_the_poe1_spelling() {
        let got = pseudo_totals(&[total("+# to Strength", 20.0)]);

        assert!(find(&got, "+# total maximum Life").is_none());
    }

    #[test]
    fn a_single_resistance_modifier_becomes_a_pseudo_total() {
        let got = pseudo_totals(&[total("#% to Fire Resistance", 30.0)]);

        assert_eq!(
            find(&got, "+#% total to Fire Resistance")
                .unwrap()
                .roll
                .value,
            30.0
        );
    }

    #[test]
    fn a_dual_resistance_modifier_feeds_both_of_its_elements() {
        let got = pseudo_totals(&[total("#% to Fire and Cold Resistances", 20.0)]);

        assert_eq!(
            find(&got, "+#% total to Fire Resistance")
                .unwrap()
                .roll
                .value,
            20.0
        );
        assert_eq!(
            find(&got, "+#% total to Cold Resistance")
                .unwrap()
                .roll
                .value,
            20.0
        );
        assert!(find(&got, "+#% total to Lightning Resistance").is_none());
    }

    #[test]
    fn a_dual_and_a_single_modifier_add_together() {
        let got = pseudo_totals(&[
            total("#% to Fire and Cold Resistances", 20.0),
            total("#% to Fire Resistance", 15.0),
        ]);

        assert_eq!(
            find(&got, "+#% total to Fire Resistance")
                .unwrap()
                .roll
                .value,
            35.0
        );
    }

    #[test]
    fn an_all_resistance_modifier_feeds_every_element_and_chaos() {
        let got = pseudo_totals(&[total("#% to All Resistances", 10.0)]);

        for reference in [
            "+#% total to Fire Resistance",
            "+#% total to Cold Resistance",
            "+#% total to Lightning Resistance",
            "+#% total to Chaos Resistance",
        ] {
            assert_eq!(
                find(&got, reference).unwrap().roll.value,
                10.0,
                "{reference}"
            );
        }
    }

    #[test]
    fn an_all_elemental_modifier_does_not_feed_chaos() {
        let got = pseudo_totals(&[total("#% to all Elemental Resistances", 10.0)]);

        assert!(find(&got, "+#% total to Fire Resistance").is_some());
        assert!(find(&got, "+#% total to Chaos Resistance").is_none());
    }

    #[test]
    fn total_elemental_resistance_counts_a_dual_twice() {
        let got = pseudo_totals(&[total("#% to Fire and Cold Resistances", 20.0)]);

        assert_eq!(
            find(&got, "+#% total Elemental Resistance")
                .unwrap()
                .roll
                .value,
            40.0
        );
    }

    #[test]
    fn total_elemental_resistance_counts_an_all_elemental_thrice() {
        let got = pseudo_totals(&[total("#% to all Elemental Resistances", 10.0)]);

        assert_eq!(
            find(&got, "+#% total Elemental Resistance")
                .unwrap()
                .roll
                .value,
            30.0
        );
    }

    #[test]
    fn a_fire_and_chaos_modifier_counts_once_toward_the_elemental_total() {
        let got = pseudo_totals(&[total("#% to Fire and Chaos Resistances", 20.0)]);

        assert_eq!(
            find(&got, "+#% total Elemental Resistance")
                .unwrap()
                .roll
                .value,
            20.0
        );
        assert_eq!(
            find(&got, "+#% total to Chaos Resistance")
                .unwrap()
                .roll
                .value,
            20.0
        );
    }

    #[test]
    fn total_life_counts_strength_at_double() {
        let got = pseudo_totals(&[
            total("# to maximum Life", 80.0),
            total("# to Strength", 30.0),
        ]);

        assert_eq!(
            find(&got, "+# total maximum Life").unwrap().roll.value,
            80.0 + 60.0
        );
    }

    #[test]
    fn total_life_is_not_offered_without_a_real_life_modifier() {
        let got = pseudo_totals(&[total("# to Strength", 30.0)]);

        assert!(find(&got, "+# total maximum Life").is_none());
        assert!(find(&got, "+# total to Strength").is_some());
    }

    #[test]
    fn total_mana_counts_intelligence_at_double() {
        let got = pseudo_totals(&[
            total("# to maximum Mana", 60.0),
            total("# to Intelligence", 20.0),
        ]);

        assert_eq!(
            find(&got, "+# total maximum Mana").unwrap().roll.value,
            60.0 + 40.0
        );
    }

    #[test]
    fn an_all_attributes_modifier_feeds_every_attribute() {
        let got = pseudo_totals(&[total("# to all Attributes", 10.0)]);

        for reference in [
            "+# total to Strength",
            "+# total to Dexterity",
            "+# total to Intelligence",
        ] {
            assert_eq!(
                find(&got, reference).unwrap().roll.value,
                10.0,
                "{reference}"
            );
        }
    }

    #[test]
    fn a_hybrid_attribute_modifier_feeds_both_of_its_attributes() {
        let got = pseudo_totals(&[total("# to Strength and Dexterity", 15.0)]);

        assert_eq!(find(&got, "+# total to Strength").unwrap().roll.value, 15.0);
        assert_eq!(
            find(&got, "+# total to Dexterity").unwrap().roll.value,
            15.0
        );
        assert!(find(&got, "+# total to Intelligence").is_none());
    }

    #[test]
    fn an_item_with_no_matching_modifiers_yields_no_totals() {
        assert!(pseudo_totals(&[total("Cannot be Frozen", 0.0)]).is_empty());
        assert!(pseudo_totals(&[]).is_empty());
    }

    #[test]
    fn total_elemental_resistance_and_total_life_start_enabled() {
        let got = pseudo_totals(&[
            total("#% to Fire Resistance", 30.0),
            total("# to maximum Life", 80.0),
        ]);

        assert!(
            !find(&got, "+#% total Elemental Resistance")
                .unwrap()
                .disabled
        );
        assert!(!find(&got, "+# total maximum Life").unwrap().disabled);
        assert!(find(&got, "+#% total to Fire Resistance").unwrap().disabled);
    }

    #[test]
    fn the_bounds_are_totalled_alongside_the_value() {
        let totals = [StatTotal {
            reference: "#% to Fire Resistance".to_string(),
            kind: ModifierType::Explicit,
            roll: Some(StatRoll {
                value: 30.0,
                min: 20.0,
                max: 40.0,
                ..StatRoll::default()
            }),
            sources: 1,
        }];

        let got = pseudo_totals(&totals);
        let fire = find(&got, "+#% total to Fire Resistance").unwrap();

        assert_eq!(fire.roll.min, 20.0);
        assert_eq!(fire.roll.max, 40.0);
    }

    #[test]
    fn a_multiplied_contributor_scales_its_bounds_too() {
        let totals = [StatTotal {
            reference: "# to Strength".to_string(),
            kind: ModifierType::Explicit,
            roll: Some(StatRoll {
                value: 30.0,
                min: 20.0,
                max: 40.0,
                ..StatRoll::default()
            }),
            sources: 1,
        }];

        let got = pseudo_totals(&totals);
        let strength = find(&got, "+# total to Strength").unwrap();

        assert_eq!(strength.roll.min, 20.0);
    }

    #[test]
    fn the_source_count_reflects_how_many_modifiers_fed_it() {
        let got = pseudo_totals(&[
            total("#% to Fire and Cold Resistances", 20.0),
            total("#% to Fire Resistance", 15.0),
        ]);

        assert_eq!(
            find(&got, "+#% total to Fire Resistance").unwrap().sources,
            2
        );
    }

    #[test]
    fn a_stat_with_no_roll_contributes_nothing() {
        let totals = [StatTotal {
            reference: "#% to Fire Resistance".to_string(),
            kind: ModifierType::Explicit,
            roll: None,
            sources: 1,
        }];

        assert!(pseudo_totals(&totals).is_empty());
    }

    #[test]
    fn every_rule_names_at_least_one_contributor() {
        for rule in PSEUDO_RULES {
            assert!(!rule.stats.is_empty(), "{}", rule.pseudo);
        }
    }

    #[test]
    fn every_pseudo_reference_is_unique() {
        let mut seen: Vec<&str> = PSEUDO_RULES.iter().map(|r| r.pseudo).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), before);
    }

    #[test]
    fn every_required_contributor_is_the_first_in_its_rule() {
        for rule in PSEUDO_RULES {
            if let Some(index) = rule.stats.iter().position(|c| c.required) {
                assert_eq!(index, 0, "{}", rule.pseudo);
            }
        }
    }

    #[test]
    fn a_stat_that_feeds_a_pseudo_total_is_rebuilt() {
        assert!(affects_pseudo("+# to maximum Life"));
        assert!(affects_pseudo("+#% to Fire Resistance"));
    }

    #[test]
    fn the_sign_marker_does_not_hide_a_match() {
        assert!(affects_pseudo("# to maximum Life"));
        assert!(affects_pseudo("-# to maximum Life"));
    }

    #[test]
    fn a_defence_stat_is_rebuilt() {
        assert!(affects_pseudo("#% increased Armour"));
        assert!(affects_pseudo("+# to maximum Energy Shield"));
    }

    #[test]
    fn a_weapon_damage_stat_is_rebuilt() {
        assert!(affects_pseudo("#% increased Physical Damage"));
        assert!(affects_pseudo("Adds # to # Fire Damage"));
    }

    #[test]
    fn chaos_damage_is_rebuilt_despite_having_no_pseudo_rule() {
        assert!(affects_pseudo("Adds # to # Chaos Damage"));
    }

    #[test]
    fn a_stat_that_feeds_nothing_is_left_alone() {
        assert!(!affects_pseudo("Cannot be Frozen"));
        assert!(!affects_pseudo("#% increased Rarity of Items found"));
    }

    #[test]
    fn an_empty_reference_feeds_nothing() {
        assert!(!affects_pseudo(""));
    }

    #[test]
    fn every_property_stat_reference_is_distinct() {
        for table in [ARMOUR_STATS, WEAPON_STATS] {
            let mut seen = table.to_vec();
            seen.sort_unstable();
            let count = seen.len();
            seen.dedup();

            assert_eq!(seen.len(), count, "{table:?}");
        }
    }

    #[test]
    fn every_property_stat_carries_a_placeholder() {
        for reference in ARMOUR_STATS.iter().chain(WEAPON_STATS) {
            assert!(reference.contains('#'), "{reference}");
        }
    }
}
