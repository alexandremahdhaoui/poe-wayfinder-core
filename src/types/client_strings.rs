//! The exact strings the English game client prints.
//!
//! Ported from `dataParser/output/en/client_strings.js`.
//!
//! The reference loads these from a per language JSON file because it supports
//! nine languages. We support English, so they are constants. That removes a
//! whole data file, a loader and a failure mode.
//!
//! Every trailing space is deliberate. The game prints `Item Level: 84` and the
//! parser strips the prefix including its space.

// ---------------------------------------------------------------------------
// Name plate
// ---------------------------------------------------------------------------

pub const ITEM_CLASS: &str = "Item Class: ";
pub const RARITY: &str = "Rarity: ";

pub const RARITY_NORMAL: &str = "Normal";
pub const RARITY_MAGIC: &str = "Magic";
pub const RARITY_RARE: &str = "Rare";
pub const RARITY_UNIQUE: &str = "Unique";
pub const RARITY_GEM: &str = "Gem";
pub const RARITY_CURRENCY: &str = "Currency";
pub const RARITY_DIVCARD: &str = "Divination Card";
pub const RARITY_QUEST: &str = "Quest";

/// Printed when the character cannot equip the item.
///
/// It sits inside the name plate section and shifts every following line, so
/// the parser lifts it out before anything else runs.
pub const CANNOT_USE_ITEM: &str = "You cannot use this item. Its stats will be ignored";

// ---------------------------------------------------------------------------
// Levels and tiers
// ---------------------------------------------------------------------------

pub const ITEM_LEVEL: &str = "Item Level: ";
pub const CORPSE_LEVEL: &str = "Corpse Level: ";
pub const AREA_LEVEL: &str = "Area Level: ";
pub const MAP_TIER: &str = "Map Tier: ";
pub const WAYSTONE_TIER: &str = "Waystone Tier: ";
pub const TALISMAN_TIER: &str = "Talisman Tier: ";
pub const GEM_LEVEL: &str = "Level: ";

// ---------------------------------------------------------------------------
// Numbers on the item
// ---------------------------------------------------------------------------

pub const STACK_SIZE: &str = "Stack Size: ";
pub const SOCKETS: &str = "Sockets: ";
pub const QUALITY: &str = "Quality: ";
pub const PHYSICAL_DAMAGE: &str = "Physical Damage: ";
pub const ELEMENTAL_DAMAGE: &str = "Elemental Damage: ";
pub const LIGHTNING_DAMAGE: &str = "Lightning Damage: ";
pub const COLD_DAMAGE: &str = "Cold Damage: ";
pub const FIRE_DAMAGE: &str = "Fire Damage: ";
pub const CRIT_CHANCE: &str = "Critical Hit Chance: ";
pub const ATTACK_SPEED: &str = "Attacks per Second: ";
pub const RELOAD_SPEED: &str = "Reload Time: ";
pub const ARMOUR: &str = "Armour: ";
pub const EVASION: &str = "Evasion Rating: ";
pub const ENERGY_SHIELD: &str = "Energy Shield: ";
pub const RUNIC_WARD: &str = "Runic Ward: ";
pub const BLOCK_CHANCE: &str = "Block chance: ";
pub const BASE_SPIRIT: &str = "Spirit: ";
pub const CHARM_SLOTS: &str = "Charm Slots: ";
pub const SENTINEL_CHARGE: &str = "Charge: ";
pub const TIMELESS_RADIUS: &str = "Radius: ";
pub const REQUIREMENTS: &str = "Requirements";
pub const REQUIRES: &str = "Requires: ";
pub const GRANTS_SKILL: &str = "Grants Skill: ";
pub const PRICE_NOTE: &str = "Note: ";
pub const HYPHEN: &str = "-";

// ---------------------------------------------------------------------------
// Waystone and map numbers
// ---------------------------------------------------------------------------

pub const WAYSTONE_REVIVES: &str = "Revives Available: ";
pub const WAYSTONE_PACK_SIZE: &str = "Monster Pack Size: ";
pub const WAYSTONE_MAGIC_MONSTERS: &str = "Magic Monsters: ";
pub const WAYSTONE_RARE_MONSTERS: &str = "Rare Monsters: ";
pub const WAYSTONE_DROP_CHANCE: &str = "Waystone Drop Chance: ";
pub const WAYSTONE_RARITY: &str = "Item Rarity: ";
pub const WAYSTONE_GOLD: &str = "Gold Found: ";

// ---------------------------------------------------------------------------
// Whole line flags
// ---------------------------------------------------------------------------

pub const CORRUPTED: &str = "Corrupted";
pub const DOUBLE_CORRUPTED: &str = "Twice Corrupted";
pub const UNIDENTIFIED: &str = "Unidentified";
pub const UNMODIFIABLE: &str = "Unmodifiable";
pub const MIRRORED: &str = "Mirrored";
pub const SANCTIFIED: &str = "Sanctified";
pub const FOIL_UNIQUE: &str = "Foil Unique";
pub const SECTION_SYNTHESISED: &str = "Synthesised Item";
pub const FRACTURED_ITEM: &str = "Fractured Item";

// ---------------------------------------------------------------------------
// Influence lines
// ---------------------------------------------------------------------------

pub const INFLUENCE_SHAPER: &str = "Shaper Item";
pub const INFLUENCE_ELDER: &str = "Elder Item";
pub const INFLUENCE_CRUSADER: &str = "Crusader Item";
pub const INFLUENCE_HUNTER: &str = "Hunter Item";
pub const INFLUENCE_REDEEMER: &str = "Redeemer Item";
pub const INFLUENCE_WARLORD: &str = "Warlord Item";

// ---------------------------------------------------------------------------
// Help text, used to identify a category with no rarity of its own
// ---------------------------------------------------------------------------

pub const METAMORPH_HELP: &str =
    "Combine this with four other different samples in Tane's Laboratory.";
pub const BEAST_HELP: &str = "Right-click to add this to your bestiary.";
pub const VOIDSTONE_HELP: &str = "Socket this into your Atlas to increase the Tier of all Maps.";

// ---------------------------------------------------------------------------
// Modifier metadata lines, printed with Advanced Item Description on
// ---------------------------------------------------------------------------

pub const PREFIX_MODIFIER: &str = "Prefix Modifier";
pub const SUFFIX_MODIFIER: &str = "Suffix Modifier";
pub const IMPLICIT_MODIFIER: &str = "Implicit Modifier";
pub const CRAFTED_MODIFIER: &str = "Master Crafted";
pub const FRACTURED_MODIFIER: &str = "Fractured";
pub const DESECRATED_MODIFIER: &str = "Desecrated";
pub const CORRUPTED_IMPLICIT: &str = "Corruption Implicit Modifier";
pub const UNIQUE_MODIFIER: &str = "Unique Modifier";
pub const VAAL_UNIQUE_MODIFIER: &str = "Vaal Unique Modifier";
pub const UNSCALABLE_VALUE: &str = " — Unscalable Value";
pub const VEILED_PREFIX: &str = "Desecrated Prefix";
pub const VEILED_SUFFIX: &str = "Desecrated Suffix";

// ---------------------------------------------------------------------------
// Eldritch implicit ranks
// ---------------------------------------------------------------------------

pub const ELDRITCH_MOD_R1: &str = "Lesser";
pub const ELDRITCH_MOD_R2: &str = "Greater";
pub const ELDRITCH_MOD_R3: &str = "Grand";
pub const ELDRITCH_MOD_R4: &str = "Exceptional";
pub const ELDRITCH_MOD_R5: &str = "Exquisite";
pub const ELDRITCH_MOD_R6: &str = "Perfect";

/// Eldritch rank words, index 0 is rank 1.
pub const ELDRITCH_MOD_RANKS: [&str; 6] = [
    ELDRITCH_MOD_R1,
    ELDRITCH_MOD_R2,
    ELDRITCH_MOD_R3,
    ELDRITCH_MOD_R4,
    ELDRITCH_MOD_R5,
    ELDRITCH_MOD_R6,
];

// ---------------------------------------------------------------------------
// Heist
// ---------------------------------------------------------------------------

pub const HEIST_WINGS_REVEALED: &str = "Wings Revealed: ";
pub const HEIST_TARGET: &str = "Heist Target: ";
pub const HEIST_BLUEPRINT_ENCHANTS: &str = "Enchanted Armaments";
pub const HEIST_BLUEPRINT_TRINKETS: &str = "Thieves' Trinkets or Currency";
pub const HEIST_BLUEPRINT_GEMS: &str = "Unusual Gems";
pub const HEIST_BLUEPRINT_REPLICAS: &str = "Replicas or Experimented Items";

// ---------------------------------------------------------------------------
// Incursion and trials
// ---------------------------------------------------------------------------

pub const INCURSION_OPEN: &str = "Open Rooms:";
pub const INCURSION_OBSTRUCTED: &str = "Obstructed Rooms:";
pub const TRIAL_COUNT: &str = "Number of Trials: ";
pub const ULTIMATUM_VICTORIOUS: &str = "Victorious";
pub const ULTIMATUM_COWARDLY: &str = "Cowardly";
pub const ULTIMATUM_DEADLY: &str = "Deadly";

// ---------------------------------------------------------------------------
// Influence naming conventions on unidentified items
// ---------------------------------------------------------------------------

pub const SHAPER_MODS: [&str; 2] = ["of Shaping", "The Shaper's"];
pub const ELDER_MODS: [&str; 2] = ["of the Elder", "The Elder's"];
pub const CRUSADER_MODS: [&str; 2] = ["Crusader's", "of the Crusade"];
pub const HUNTER_MODS: [&str; 2] = ["Hunter's", "of the Hunt"];
pub const REDEEMER_MODS: [&str; 2] = ["Redeemer's", "of Redemption"];
pub const WARLORD_MODS: [&str; 2] = ["Warlord's", "of the Conquest"];
pub const DELVE_MODS: [&str; 2] = ["Subterranean", "of the Underground"];
pub const VEILED_MODS: [&str; 2] = ["Chosen", "of the Order"];

pub const INCURSION_MODS: [&str; 11] = [
    "Guatelitzi's",
    "Xopec's",
    "Topotante's",
    "Tacati's",
    "Matatl's",
    "of Matatl",
    "Citaqualotl's",
    "of Citaqualotl",
    "of Tacati",
    "of Guatelitzi",
    "of Puhuarte",
];

// ---------------------------------------------------------------------------
// Name prefixes
//
// The reference writes these as anchored regexes with one capture group. In
// English each is a literal prefix, so a prefix strip does the same job with
// no regex engine and no backtracking.
// ---------------------------------------------------------------------------

pub const ITEM_SUPERIOR: &str = "Superior ";
pub const ITEM_EXCEPTIONAL: &str = "Exceptional ";
pub const ITEM_RUNEFORGED: &str = "Runeforged ";
pub const ITEM_SYNTHESISED: &str = "Synthesised ";
pub const MAP_BLIGHTED: &str = "Blighted ";
pub const MAP_BLIGHT_RAVAGED: &str = "Blight-ravaged ";
pub const QUALITY_ANOMALOUS: &str = "Anomalous ";
pub const QUALITY_DIVERGENT: &str = "Divergent ";
pub const QUALITY_PHANTASMAL: &str = "Phantasmal ";
pub const ITEM_FOULBORN: &str = "Foulborn ";
pub const ITEM_VESTIGIAL: &str = "Vestigial ";

// ---------------------------------------------------------------------------
// PoE1 only lines
//
// Exiled Exchange 2 is a PoE2 fork and dropped every one of these. They come
// from Awakened PoE Trade's client strings.
// ---------------------------------------------------------------------------

pub const SPLIT: &str = "Split";
pub const MEMORY_STRANDS: &str = "Memory Strands: ";
pub const HEIST_CONTRACT_REQUIRES: &str = "Requires ";
pub const HEIST_CONTRACT_TARGET: &str = "Heist Target: ";
pub const HEIST_TARGET_PRICELESS: &str = "Priceless";

/// The nine heist jobs, in the reference's order.
///
/// Each pairs the printed name with the trade id the site files it under. The
/// site calls them `item.heist_job_*` with the spaces and the hyphen removed,
/// which is not derivable from the name and is why both halves are written
/// out.
pub const HEIST_JOBS: [(&str, &str); 9] = [
    ("Lockpicking", "item.heist_job_lockpicking"),
    ("Brute Force", "item.heist_job_bruteforce"),
    ("Perception", "item.heist_job_perception"),
    ("Demolition", "item.heist_job_demolition"),
    ("Counter-Thaumaturgy", "item.heist_job_counterthaumaturgy"),
    ("Trap Disarmament", "item.heist_job_trapdisarmament"),
    ("Agility", "item.heist_job_agility"),
    ("Deception", "item.heist_job_deception"),
    ("Engineering", "item.heist_job_engineering"),
];

/// The trade id for a contract whose target is priceless.
pub const HEIST_TARGET_PRICELESS_ID: &str = "item.heist_target_priceless";

// ---------------------------------------------------------------------------
// Metamorph organ suffixes
// ---------------------------------------------------------------------------

pub const METAMORPH_ORGANS: [&str; 5] = ["Brain", "Eye", "Lung", "Heart", "Liver"];

/// Strip a literal prefix and return the rest.
///
/// Returns None when the text does not start with the prefix, so a caller can
/// chain attempts without checking twice.
pub fn strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixed_lines_keep_their_trailing_space() {
        // A missing trailing space silently turns "Item Level: 84" into
        // " 84" and every number parse then depends on a trim.
        for s in [
            ITEM_CLASS,
            RARITY,
            ITEM_LEVEL,
            AREA_LEVEL,
            MAP_TIER,
            WAYSTONE_TIER,
            QUALITY,
            SOCKETS,
            STACK_SIZE,
            ARMOUR,
            EVASION,
            ENERGY_SHIELD,
            PRICE_NOTE,
        ] {
            assert!(s.ends_with(' '), "{s:?} lost its trailing space");
        }
    }

    #[test]
    fn whole_line_flags_carry_no_trailing_space() {
        for s in [CORRUPTED, MIRRORED, SANCTIFIED, UNMODIFIABLE, FOIL_UNIQUE] {
            assert!(!s.ends_with(' '), "{s:?} gained a trailing space");
        }
    }

    #[test]
    fn name_prefixes_end_with_a_space() {
        // These strip a word off the front of a name. Without the space
        // "Superiority" would strip to "ity".
        for s in [
            ITEM_SUPERIOR,
            ITEM_EXCEPTIONAL,
            ITEM_RUNEFORGED,
            ITEM_SYNTHESISED,
            MAP_BLIGHTED,
            MAP_BLIGHT_RAVAGED,
            QUALITY_ANOMALOUS,
            QUALITY_DIVERGENT,
            QUALITY_PHANTASMAL,
        ] {
            assert!(s.ends_with(' '), "{s:?} lost its trailing space");
        }
    }

    #[test]
    fn stripping_a_prefix_leaves_the_base_name() {
        assert_eq!(
            strip_prefix("Superior Iron Hat", ITEM_SUPERIOR),
            Some("Iron Hat")
        );
        assert_eq!(
            strip_prefix("Blight-ravaged Toxic Map", MAP_BLIGHT_RAVAGED),
            Some("Toxic Map")
        );
    }

    #[test]
    fn stripping_a_prefix_that_is_not_there_returns_nothing() {
        assert_eq!(strip_prefix("Iron Hat", ITEM_SUPERIOR), None);

        // The trailing space is what stops this matching.
        assert_eq!(strip_prefix("Superiority", ITEM_SUPERIOR), None);
    }

    #[test]
    fn blighted_does_not_swallow_blight_ravaged() {
        // "Blight-ravaged Toxic Map" must not strip as "Blighted ". It does
        // not, because the hyphen differs, and this test pins that.
        assert_eq!(strip_prefix("Blight-ravaged Toxic Map", MAP_BLIGHTED), None);
    }

    #[test]
    fn the_unscalable_value_marker_uses_an_em_dash() {
        // A hyphen here would silently stop matching. The game prints U+2014.
        assert!(UNSCALABLE_VALUE.contains('\u{2014}'));
    }

    #[test]
    fn eldritch_ranks_are_in_ascending_order() {
        assert_eq!(ELDRITCH_MOD_RANKS[0], "Lesser");
        assert_eq!(ELDRITCH_MOD_RANKS[5], "Perfect");
        assert_eq!(ELDRITCH_MOD_RANKS.len(), 6);
    }

    #[test]
    fn apostrophes_are_typewriter_and_not_curly() {
        // The game prints U+0027. A curly quote here would never match.
        for s in [
            METAMORPH_HELP,
            HEIST_BLUEPRINT_TRINKETS,
            SHAPER_MODS[1],
            CRUSADER_MODS[0],
        ] {
            assert!(!s.contains('\u{2019}'), "{s:?} has a curly apostrophe");
        }
    }
}
