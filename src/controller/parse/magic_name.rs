//! Finding the base type inside a magic item's rolled name.
//!
//! Ported from `renderer/src/parser/magic-name.ts`, plus `replaceHashWithValues`
//! and `modsEqual` from `Parser.ts` and `modifiers.ts`.
//!
//! # The problem
//!
//! A magic item prints one line with its prefix, its base and its suffix all
//! run together: `Serrated Spine Bow of the Lynx`. There is no separator and
//! no marker. The base is `Spine Bow` and nothing in the text says so.
//!
//! # The approach
//!
//! Try every contiguous run of words against the item table and keep the ones
//! that are real bases. `Serrated Spine Bow of the Lynx` yields `Spine`,
//! `Spine Bow`, `Bow` and a few dozen others, and only two of those are bases.
//!
//! The longest match wins. `Spine Bow` beats `Bow`, and taking `Bow` would
//! search every bow in the game.

use crate::adapter::data_adapter::{ItemLookup, Namespace};
use crate::types::GameVersion;

/// Every contiguous run of words in a name, longest first.
///
/// Longest first so the caller can take the first hit rather than scoring
/// them. A shorter run is always a substring of a longer one, so a shorter
/// match is always the worse answer.
pub fn word_runs(name: &str) -> Vec<String> {
    let words: Vec<&str> = name.split(' ').filter(|w| !w.is_empty()).collect();

    let mut out = Vec::new();

    for start in 0..words.len() {
        for end in start..words.len() {
            out.push(words[start..=end].join(" "));
        }
    }

    // Longest first. Ties keep their original order so the result is stable
    // across runs, which a search that changed between identical items would
    // not be.
    out.sort_by_key(|run| std::cmp::Reverse(run.len()));
    out.dedup();

    out
}

/// Find the base type inside a magic item's rolled name.
///
/// Ported from `magicBasetype`. Returns nothing when no run of words is a
/// craftable base, which happens on an item whose base our data does not have.
/// The caller then searches by the whole name, which finds nothing but is
/// honest.
///
/// Only a craftable base counts. `of the Lynx` might collide with a currency
/// name, and a currency is not what a magic bow is.
pub fn magic_base_type(name: &str, data: &dyn ItemLookup, game: GameVersion) -> Option<String> {
    for run in word_runs(name) {
        let found = data.items_by_name(&run, Namespace::Item, game);

        if found.iter().any(|base| base.craftable) {
            return Some(run);
        }
    }

    None
}

/// Fill a stat template's placeholders with real numbers.
///
/// Ported from `replaceHashWithValues`. Used to show a user what a stat says
/// with its rolls in place, rather than the `#` the data file stores.
///
/// A template with more placeholders than values keeps the extra ones, which
/// reads oddly and is better than silently dropping them.
pub fn fill_placeholders(template: &str, values: &[f64]) -> String {
    let mut out = template.to_string();

    for value in values {
        let Some(position) = out.find('#') else {
            break;
        };

        // Render a whole number without a trailing decimal. "+25 to Life"
        // reads better than "+25.0 to Life" and matches what the game prints.
        let rendered = if value.fract() == 0.0 && value.abs() < 1.0e15 {
            format!("{}", *value as i64)
        } else {
            format!("{value}")
        };

        out.replace_range(position..position + 1, &rendered);
    }

    out
}

/// Whether two modifiers are the same modifier.
///
/// Ported from `modsEqual`. Used to spot a modifier the user has already seen,
/// so the panel does not list one twice.
///
/// Every field has to match, including the rolls. Two life prefixes with
/// different rolls are two different modifiers on the same item.
pub fn mods_equal(
    a: &crate::controller::parse::shared::modifiers::ParsedModifier,
    b: &crate::controller::parse::shared::modifiers::ParsedModifier,
) -> bool {
    if a.stats.len() != b.stats.len() {
        return false;
    }

    if a.info.name != b.info.name
        || a.info.tier != b.info.tier
        || a.info.generation != b.info.generation
        || a.info.kind != b.info.kind
    {
        return false;
    }

    a.stats.iter().zip(&b.stats).all(|(x, y)| {
        x.reference == y.reference
            && x.roll.map(|r| (r.value, r.min, r.max)) == y.roll.map(|r| (r.value, r.min, r.max))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::data_adapter::StatLookup;
    use crate::controller::parse::shared::modifiers::ParsedModifier;
    use crate::types::item::BaseInfo;
    use crate::types::modifier::{Generation, ModifierInfo, ModifierType};
    use crate::types::stat::{ParsedStat, StatHit, StatRoll};

    struct FakeItems {
        craftable: Vec<&'static str>,
        other: Vec<&'static str>,
    }

    impl StatLookup for FakeItems {
        fn stat_by_matcher(&self, _: &str) -> Option<StatHit<'_>> {
            None
        }
    }

    impl ItemLookup for FakeItems {
        fn items_by_name(&self, name: &str, _: Namespace, _: GameVersion) -> Vec<BaseInfo> {
            let craftable = self.craftable.contains(&name);
            let known = craftable || self.other.contains(&name);

            if !known {
                return Vec::new();
            }

            vec![BaseInfo {
                name: name.to_string(),
                reference_name: name.to_string(),
                craftable,
                ..BaseInfo::default()
            }]
        }
    }

    fn data(craftable: Vec<&'static str>) -> FakeItems {
        FakeItems {
            craftable,
            other: Vec::new(),
        }
    }

    #[test]
    fn every_contiguous_run_of_words_is_produced() {
        let got = word_runs("a b c");

        assert!(got.contains(&"a".to_string()));
        assert!(got.contains(&"b c".to_string()));
        assert!(got.contains(&"a b c".to_string()));
        // Not contiguous.
        assert!(!got.contains(&"a c".to_string()));
    }

    #[test]
    fn the_runs_come_longest_first() {
        // A shorter run is always a substring of a longer one, so a shorter
        // match is always the worse answer.
        let got = word_runs("a bb ccc");

        assert!(got[0].len() >= got[got.len() - 1].len());
        assert_eq!(got[0], "a bb ccc");
    }

    #[test]
    fn a_one_word_name_yields_itself() {
        assert_eq!(word_runs("Bow"), vec!["Bow"]);
    }

    #[test]
    fn an_empty_name_yields_nothing() {
        assert!(word_runs("").is_empty());
    }

    #[test]
    fn repeated_spaces_do_not_produce_empty_words() {
        let got = word_runs("a  b");

        assert!(got.contains(&"a b".to_string()));
        assert!(!got.iter().any(std::string::String::is_empty));
    }

    #[test]
    fn the_base_is_found_inside_a_rolled_magic_name() {
        // The whole point. Nothing in the text says where the base starts.
        let got = magic_base_type(
            "Serrated Spine Bow of the Lynx",
            &data(vec!["Spine Bow"]),
            GameVersion::Poe1,
        );

        assert_eq!(got.as_deref(), Some("Spine Bow"));
    }

    #[test]
    fn the_longest_matching_base_wins() {
        // Taking "Bow" would search every bow in the game.
        let got = magic_base_type(
            "Serrated Spine Bow of the Lynx",
            &data(vec!["Spine Bow", "Bow"]),
            GameVersion::Poe1,
        );

        assert_eq!(got.as_deref(), Some("Spine Bow"));
    }

    #[test]
    fn a_known_but_uncraftable_name_is_not_a_base() {
        // "of the Lynx" might collide with a currency name, and a currency is
        // not what a magic bow is.
        let items = FakeItems {
            craftable: vec!["Spine Bow"],
            other: vec!["Serrated Spine Bow of the Lynx"],
        };

        let got = magic_base_type("Serrated Spine Bow of the Lynx", &items, GameVersion::Poe1);

        assert_eq!(got.as_deref(), Some("Spine Bow"));
    }

    #[test]
    fn a_name_with_no_known_base_yields_nothing() {
        // The caller then searches by the whole name, which finds nothing but
        // is honest.
        let got = magic_base_type("Mystery Thing of Mystery", &data(vec![]), GameVersion::Poe1);

        assert_eq!(got, None);
    }

    #[test]
    fn a_bare_base_name_is_found() {
        let got = magic_base_type("Spine Bow", &data(vec!["Spine Bow"]), GameVersion::Poe1);

        assert_eq!(got.as_deref(), Some("Spine Bow"));
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(
            fill_placeholders("Adds # to # Fire Damage", &[5.0, 12.0]),
            "Adds 5 to 12 Fire Damage"
        );
    }

    #[test]
    fn a_whole_number_renders_without_a_decimal() {
        // "+25 to Life" reads better than "+25.0 to Life" and matches what the
        // game prints.
        assert_eq!(fill_placeholders("# to Life", &[25.0]), "25 to Life");
    }

    #[test]
    fn a_fractional_number_keeps_its_decimals() {
        assert_eq!(fill_placeholders("#% Leeched", &[1.5]), "1.5% Leeched");
    }

    #[test]
    fn a_negative_number_keeps_its_sign() {
        assert_eq!(fill_placeholders("# to Life", &[-25.0]), "-25 to Life");
    }

    #[test]
    fn extra_placeholders_are_left_in_place() {
        // Reads oddly and is better than silently dropping them.
        assert_eq!(
            fill_placeholders("Adds # to # Fire Damage", &[5.0]),
            "Adds 5 to # Fire Damage"
        );
    }

    #[test]
    fn extra_values_are_ignored() {
        assert_eq!(fill_placeholders("# to Life", &[25.0, 99.0]), "25 to Life");
    }

    #[test]
    fn a_template_with_no_placeholders_is_unchanged() {
        assert_eq!(
            fill_placeholders("Cannot be Frozen", &[5.0]),
            "Cannot be Frozen"
        );
    }

    fn modifier(name: Option<&str>, tier: Option<u32>, roll: Option<f64>) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                generation: Some(Generation::Prefix),
                name: name.map(str::to_string),
                tier,
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: "# to maximum Life".into(),
                matched: "# to maximum Life".into(),
                roll: roll.map(|value| StatRoll {
                    value,
                    min: value,
                    max: value,
                    ..StatRoll::default()
                }),
            }],
        }
    }

    #[test]
    fn two_identical_modifiers_are_equal() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let b = modifier(Some("Rotund"), Some(3), Some(50.0));

        assert!(mods_equal(&a, &b));
    }

    #[test]
    fn two_life_prefixes_with_different_rolls_are_different_modifiers() {
        // They are two modifiers on the same item, not one listed twice.
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let b = modifier(Some("Rotund"), Some(3), Some(40.0));

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn a_different_tier_makes_them_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let b = modifier(Some("Rotund"), Some(2), Some(50.0));

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn a_different_name_makes_them_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let b = modifier(Some("Athletic"), Some(3), Some(50.0));

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn a_different_generation_makes_them_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let mut b = modifier(Some("Rotund"), Some(3), Some(50.0));
        b.info.generation = Some(Generation::Suffix);

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn a_different_type_makes_them_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let mut b = modifier(Some("Rotund"), Some(3), Some(50.0));
        b.info.kind = Some(ModifierType::Crafted);

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn a_different_stat_count_makes_them_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let mut b = modifier(Some("Rotund"), Some(3), Some(50.0));
        b.stats.clear();

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn one_with_a_roll_and_one_without_are_different() {
        let a = modifier(Some("Rotund"), Some(3), Some(50.0));
        let b = modifier(Some("Rotund"), Some(3), None);

        assert!(!mods_equal(&a, &b));
    }

    #[test]
    fn two_rollless_modifiers_are_equal() {
        let a = modifier(None, None, None);
        let b = modifier(None, None, None);

        assert!(mods_equal(&a, &b));
    }
}
