//! The modifier model.
//!
//! Ported from `renderer/src/parser/modifiers.ts` and the metadata half of
//! `advanced-mod-desc.ts`.

use std::collections::HashSet;

/// The namespace a modifier belongs to.
///
/// A stat can exist in several namespaces with a different trade id in each,
/// which is why `TradeIds` is keyed by this.
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
    /// Called `rune` on the trade site.
    Augment,
    AddedAugment,
    Sanctum,
    Desecrated,
    Skill,
}

impl ModifierType {
    /// The key used in the trade id table.
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

    /// The suffix the game prints after a stat line for this type.
    ///
    /// Explicit modifiers carry no suffix, which is why this returns an
    /// option and why suffix detection is an ordered chain.
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

/// How a modifier was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    Prefix,
    Suffix,
    Corrupted,
    Eldritch,
    Mutated,
}

/// Metadata the game prints when Advanced Item Description is enabled.
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
