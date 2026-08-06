//! Pseudo filters.
//!
//! Ported from `web/price-check/filters/pseudo/index.ts`.
//!
//! # What a pseudo filter is
//!
//! A filter on something no single modifier grants. Total fire resistance is
//! the sum of every modifier that gives fire resistance, and there are seven
//! different ones. A ring with `+20% to Fire and Cold Resistances` and
//! `+15% to Fire Resistance` has 35% fire resistance, and no filter on any
//! real modifier finds that.
//!
//! The trade site indexes these itself under the `pseudo` namespace, so the
//! client only has to work out the total and send it.
//!
//! # Why this is most of what people actually search
//!
//! Nobody buys a ring for a specific prefix. They buy it for total resistance
//! and total life, because that is what a build needs. Without pseudo filters
//! the tool can only search for the exact modifier split an item happens to
//! have, which is close to useless for jewellery.

use crate::controller::aggregate::StatTotal;
use crate::types::stat::StatRoll;

/// One real stat that feeds a pseudo total.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contributor {
    /// The canonical reference of the real stat.
    pub reference: &'static str,
    /// How much of this stat counts toward the total.
    ///
    /// Two for an attribute feeding life, because ten strength gives five
    /// life and the trade site counts strength at double.
    pub multiplier: f64,
    /// The pseudo total is only offered when this stat is present.
    ///
    /// Total life without any real life modifier is a number no buyer wants,
    /// because it would match a ring with only strength on it.
    pub required: bool,
}

impl Contributor {
    /// A contributor that counts once.
    const fn plain(reference: &'static str) -> Self {
        Self {
            reference,
            multiplier: 1.0,
            required: false,
        }
    }

    /// A contributor that counts more than once.
    const fn scaled(reference: &'static str, multiplier: f64) -> Self {
        Self {
            reference,
            multiplier,
            required: false,
        }
    }

    /// A contributor without which the total is not offered.
    const fn required(reference: &'static str) -> Self {
        Self {
            reference,
            multiplier: 1.0,
            required: true,
        }
    }
}

/// One pseudo total.
#[derive(Debug, Clone, PartialEq)]
pub struct PseudoRule {
    /// The pseudo stat's canonical reference.
    pub pseudo: &'static str,
    /// Whether the filter starts switched off.
    ///
    /// Most do. A query that requires every pseudo total at once returns the
    /// one listing that is this exact item.
    pub disabled: bool,
    /// The real stats that feed it.
    pub stats: &'static [Contributor],
}

// ---------------------------------------------------------------------------
// Resistances
//
// Seven modifiers grant fire resistance and the trade site indexes the total.
// A ring with a dual resistance modifier and a single one has both counted.
// ---------------------------------------------------------------------------

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

/// Total elemental resistance counts a dual modifier twice and an all
/// resistance modifier three times, because it grants three elements.
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

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

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

/// Life counts strength at double, because ten strength gives five life and
/// the trade site's own pseudo total uses that ratio.
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

/// Every pseudo total the client offers.
///
/// The references are the trade site's own spellings, taken from the live
/// `/data/stats` response. They are exact and several are not what you would
/// guess: every resistance pseudo carries a leading plus and the elemental
/// total is singular. A wrong spelling here does not error, it silently
/// produces no filter, which is the failure mode this comment exists to
/// prevent.
///
/// Order is the order the panel shows them in. Resistance and life come first
/// because they are what almost every search is about.
pub const PSEUDO_RULES: &[PseudoRule] = &[
    PseudoRule {
        pseudo: "+#% total Elemental Resistance",
        // The one pseudo total that starts on. It is the single most searched
        // number in the game.
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

/// One pseudo total worked out from an item.
#[derive(Debug, Clone, PartialEq)]
pub struct PseudoTotal {
    pub reference: &'static str,
    pub roll: StatRoll,
    pub disabled: bool,
    /// How many real modifiers fed it.
    pub sources: usize,
}

/// Work out every pseudo total an item supports.
///
/// A rule yields nothing when no contributor is present, or when a required
/// contributor is missing. Total life without any real life modifier would
/// match a ring carrying only strength, which is not what anyone means.
pub fn pseudo_totals(totals: &[StatTotal]) -> Vec<PseudoTotal> {
    let mut out = Vec::new();

    for rule in PSEUDO_RULES {
        let mut value = 0.0;
        let mut min = 0.0;
        let mut max = 0.0;
        let mut sources = 0;
        let mut missing_required = false;

        for contributor in rule.stats {
            // Sign insensitive, because the two games spell the same stat
            // differently. PoE1 calls it `+# to maximum Life` and PoE2 calls it
            // `# to maximum Life`. Comparing outright gave PoE1 no pseudo
            // totals at all, which is most of what a PoE1 search is.
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

/// Whether a stat feeds a pseudo total or a property filter.
///
/// Ported from `refEffectsPseudos`.
///
/// # Why this question is asked
///
/// A rune preview replaces the item's filters with the ones it would have.
/// Rebuilding every filter is wrong: only the ones the rune can move need to
/// change, and rebuilding the rest resets whatever the user had switched on.
///
/// A stat that feeds nothing is left exactly as it was.
pub fn affects_pseudo(reference: &str) -> bool {
    if PSEUDO_RULES
        .iter()
        .flat_map(|rule| rule.stats)
        .any(|c| signs_match(c.reference, reference))
    {
        return true;
    }

    // A defence or damage stat feeds a property filter rather than a pseudo
    // total, and a rune moves those just as much.
    if ARMOUR_STATS.contains(&reference) || WEAPON_STATS.contains(&reference) {
        return true;
    }

    // Chaos damage has no pseudo rule of its own but a rune still adds it, so
    // the preview has to rebuild its filter.
    reference == "Adds # to # Chaos Damage"
}

/// Whether two references are the same stat ignoring the sign marker.
///
/// The rules spell a stat `+# to maximum Life` and an item can print it as
/// `# to maximum Life` or `-# to maximum Life`. Comparing the strings outright
/// misses two forms of the same stat, and the preview then leaves a filter the
/// rune does move.
fn signs_match(rule: &str, reference: &str) -> bool {
    rule == reference || strip_sign(rule) == strip_sign(reference)
}

fn strip_sign(reference: &str) -> String {
    reference.replace("+#", "#").replace("-#", "#")
}

/// The armour stats a property filter already covers.
///
/// Sending both a defence filter and the modifier that produced it filters the
/// same thing twice.
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

/// The weapon stats a property filter already covers.
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
        // PoE1 names the stat `+#% to Fire Resistance` and PoE2 names it
        // `#% to Fire Resistance`. Matching the string outright gave a PoE1
        // item no pseudo totals at all.
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
        // Life is only offered when a real life modifier is present. Reading
        // the PoE1 spelling as absent suppressed the single most searched
        // filter in the game.
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
        // The sign tolerance must not turn into matching anything. Strength
        // alone is not life in either game.
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
        // This is the whole reason pseudo filters exist. A filter on any real
        // modifier misses an item whose resistance comes from a dual.
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
        // A ring with +20% fire and cold and +15% fire has 35% fire.
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
        // It grants two elements, so it contributes twice to the total across
        // all three. This is how the trade site's own pseudo works.
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
        // Only its fire half is elemental.
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
        // Ten strength gives five life and the trade site's pseudo uses that
        // ratio.
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
        // It would match a ring carrying only strength, which is not what
        // anyone searching for life means.
        let got = pseudo_totals(&[total("# to Strength", 30.0)]);

        assert!(find(&got, "+# total maximum Life").is_none());
        // Strength itself is still offered.
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
        // An empty pseudo filter matches everything and only slows the search.
        assert!(pseudo_totals(&[total("Cannot be Frozen", 0.0)]).is_empty());
        assert!(pseudo_totals(&[]).is_empty());
    }

    #[test]
    fn total_elemental_resistance_and_total_life_start_enabled() {
        // They are what almost every search is about. Everything else starts
        // off, because a query requiring every pseudo total returns the one
        // listing that is this exact item.
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
        // A pseudo total's range is what tells the UI whether the roll is good.
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

        // Strength itself is not multiplied.
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
        // A rule with no contributors can never fire and is dead weight.
        for rule in PSEUDO_RULES {
            assert!(!rule.stats.is_empty(), "{}", rule.pseudo);
        }
    }

    #[test]
    fn every_pseudo_reference_is_unique() {
        // Two rules producing the same reference would send two filters on one
        // trade id, and the second would overwrite the first.
        let mut seen: Vec<&str> = PSEUDO_RULES.iter().map(|r| r.pseudo).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), before);
    }

    #[test]
    fn every_required_contributor_is_the_first_in_its_rule() {
        // Not a correctness requirement, but a rule whose required stat is
        // buried reads as if it were optional.
        for rule in PSEUDO_RULES {
            if let Some(index) = rule.stats.iter().position(|c| c.required) {
                assert_eq!(index, 0, "{}", rule.pseudo);
            }
        }
    }

    // -----------------------------------------------------------------
    // Which stats a rune preview has to rebuild
    // -----------------------------------------------------------------

    #[test]
    fn a_stat_that_feeds_a_pseudo_total_is_rebuilt() {
        // Only the filters a rune can move need to change. Rebuilding the rest
        // resets whatever the user had switched on.
        assert!(affects_pseudo("+# to maximum Life"));
        assert!(affects_pseudo("+#% to Fire Resistance"));
    }

    #[test]
    fn the_sign_marker_does_not_hide_a_match() {
        // The rules spell it "+# to maximum Life" and an item can print it
        // without the plus. Comparing outright leaves a filter the rune moves.
        assert!(affects_pseudo("# to maximum Life"));
        assert!(affects_pseudo("-# to maximum Life"));
    }

    #[test]
    fn a_defence_stat_is_rebuilt() {
        // It feeds a property filter rather than a pseudo total, and a rune
        // moves those just as much.
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
        // A rune still adds it, so the preview has to rebuild its filter.
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
        // A duplicate would be dead weight and hide a typo in its twin.
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
        // A reference with no hash matches no roll and silently filters
        // nothing, which is the failure this crate keeps hitting.
        for reference in ARMOUR_STATS.iter().chain(WEAPON_STATS) {
            assert!(reference.contains('#'), "{reference}");
        }
    }
}
