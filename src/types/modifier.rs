use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierType {
    Pseudo,
    Explicit,
    Implicit,
    Crafted,
    Enchant,
    Scourge,
    Necropolis,
    Veiled,
    Fractured,
    Augment,
    AddedAugment,
    Sanctum,
    Desecrated,
    Skill,
}

impl ModifierType {
    pub fn as_str(self) -> &'static str {
        match self {
            ModifierType::Pseudo => "pseudo",
            ModifierType::Explicit => "explicit",
            ModifierType::Implicit => "implicit",
            ModifierType::Crafted => "crafted",
            ModifierType::Enchant => "enchant",
            ModifierType::Scourge => "scourge",
            ModifierType::Necropolis => "necropolis",
            ModifierType::Veiled => "veiled",
            ModifierType::Fractured => "fractured",
            ModifierType::Augment => "rune",
            ModifierType::AddedAugment => "added-rune",
            ModifierType::Sanctum => "sanctum",
            ModifierType::Desecrated => "desecrated",
            ModifierType::Skill => "skill",
        }
    }

    pub fn line_suffix(self) -> Option<&'static str> {
        match self {
            ModifierType::Scourge => Some(" (scourge)"),
            ModifierType::Enchant => Some(" (enchant)"),
            ModifierType::Implicit => Some(" (implicit)"),
            ModifierType::Fractured => Some(" (fractured)"),
            ModifierType::Crafted => Some(" (crafted)"),
            ModifierType::Augment => Some(" (rune)"),
            ModifierType::AddedAugment => Some(" (added rune)"),
            ModifierType::Desecrated => Some(" (desecrated)"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    Prefix,
    Suffix,
    Corrupted,
    Eldritch,
    Mutated,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifierInfo {
    pub kind: Option<ModifierType>,
    pub generation: Option<Generation>,
    pub name: Option<String>,
    pub tier: Option<u32>,
    pub rank: Option<u32>,
    pub tags: Vec<String>,
    pub roll_incr: Option<f64>,
    pub hybrid_with_ref: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY: &[ModifierType] = &[
        ModifierType::Pseudo,
        ModifierType::Explicit,
        ModifierType::Implicit,
        ModifierType::Crafted,
        ModifierType::Enchant,
        ModifierType::Scourge,
        ModifierType::Necropolis,
        ModifierType::Veiled,
        ModifierType::Fractured,
        ModifierType::Augment,
        ModifierType::AddedAugment,
        ModifierType::Sanctum,
        ModifierType::Desecrated,
        ModifierType::Skill,
    ];

    #[test]
    fn every_kind_has_a_spelling_the_trade_site_knows() {
        for kind in EVERY {
            assert!(!kind.as_str().is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn no_two_kinds_share_a_spelling() {
        let mut seen: Vec<&str> = EVERY.iter().map(|k| k.as_str()).collect();
        seen.sort_unstable();

        let before = seen.len();
        seen.dedup();

        assert_eq!(before, seen.len(), "two kinds would collide on the wire");
    }

    #[test]
    fn a_rune_is_spelled_the_way_the_trade_site_spells_it() {
        assert_eq!(ModifierType::Augment.as_str(), "rune");
        assert_eq!(ModifierType::AddedAugment.as_str(), "added-rune");
    }

    #[test]
    fn a_suffix_is_what_the_game_prints_after_the_line() {
        assert_eq!(ModifierType::Implicit.line_suffix(), Some(" (implicit)"));
        assert_eq!(ModifierType::Crafted.line_suffix(), Some(" (crafted)"));
    }

    #[test]
    fn an_explicit_modifier_prints_no_suffix_because_it_is_the_default() {
        assert_eq!(ModifierType::Explicit.line_suffix(), None);
    }

    #[test]
    fn every_suffix_is_parenthesised_and_starts_with_a_space() {
        for kind in EVERY {
            let Some(suffix) = kind.line_suffix() else {
                continue;
            };

            assert!(suffix.starts_with(" ("), "{kind:?} -> {suffix:?}");
            assert!(suffix.ends_with(')'), "{kind:?} -> {suffix:?}");
        }
    }

    #[test]
    fn no_two_kinds_share_a_suffix() {
        let mut seen: Vec<&str> = EVERY.iter().filter_map(|k| k.line_suffix()).collect();
        seen.sort_unstable();

        let before = seen.len();
        seen.dedup();

        assert_eq!(before, seen.len(), "the parser could not tell them apart");
    }
}
