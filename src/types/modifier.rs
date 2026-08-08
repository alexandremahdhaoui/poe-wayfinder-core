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
