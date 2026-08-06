//! The parsed item.
//!
//! Ported from `renderer/src/parser/ParsedItem.ts`.
//!
//! The reference holds one flat interface with about 90 optional fields. Here
//! the armour, weapon and map blocks are grouped, because a weapon never has a
//! map field and the grouping makes that obvious. Field names inside each
//! group keep the reference spelling so a parser port stays greppable.

use crate::types::category::ItemCategory;
use crate::types::modifier::ModifierType;

/// How rare an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemRarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

impl ItemRarity {
    /// The word the game prints on the `Rarity:` line.
    pub fn as_str(self) -> &'static str {
        match self {
            ItemRarity::Normal => "Normal",
            ItemRarity::Magic => "Magic",
            ItemRarity::Rare => "Rare",
            ItemRarity::Unique => "Unique",
        }
    }

    /// Parse the word the game prints.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Normal" => Some(ItemRarity::Normal),
            "Magic" => Some(ItemRarity::Magic),
            "Rare" => Some(ItemRarity::Rare),
            "Unique" => Some(ItemRarity::Unique),
            _ => None,
        }
    }
}

/// The six PoE1 influences.
///
/// PoE2 has none. The field stays because one parser serves both games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Influence {
    Crusader,
    Elder,
    Hunter,
    Redeemer,
    Shaper,
    Warlord,
}

impl Influence {
    /// The trade site spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Influence::Crusader => "Crusader",
            Influence::Elder => "Elder",
            Influence::Hunter => "Hunter",
            Influence::Redeemer => "Redeemer",
            Influence::Shaper => "Shaper",
            Influence::Warlord => "Warlord",
        }
    }

    /// Parse the trade site spelling.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Crusader" => Some(Influence::Crusader),
            "Elder" => Some(Influence::Elder),
            "Hunter" => Some(Influence::Hunter),
            "Redeemer" => Some(Influence::Redeemer),
            "Shaper" => Some(Influence::Shaper),
            "Warlord" => Some(Influence::Warlord),
            _ => None,
        }
    }
}

/// How a map was corrupted by Blight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapBlighted {
    Blighted,
    BlightRavaged,
}

impl MapBlighted {
    /// The trade site spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            MapBlighted::Blighted => "Blighted",
            MapBlighted::BlightRavaged => "Blight-ravaged",
        }
    }
}

/// What a heist contract targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeistTarget {
    Enchants,
    Trinkets,
    Gems,
    Replicas,
}

/// The hint an ultimatum trial prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UltimatumHint {
    Victorious,
    Cowardly,
    Deadly,
}

/// The range a base's defences can roll in.
///
/// Each is a min and a max. Absent means our data does not know, which is the
/// normal case until the bundle tables are vendored.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmourBounds {
    pub ar: Option<(f64, f64)>,
    pub ev: Option<(f64, f64)>,
    pub es: Option<(f64, f64)>,
}

/// Defensive numbers on an armour piece.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmourStats {
    /// Armour rating.
    pub ar: Option<f64>,
    /// Evasion rating.
    pub ev: Option<f64>,
    /// Energy shield.
    pub es: Option<f64>,
    /// Ward. PoE1 only.
    pub rw: Option<f64>,
    /// Block chance.
    pub block: Option<f64>,
}

impl ArmourStats {
    /// Whether the item printed any defensive number at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Offensive numbers on a weapon.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponStats {
    /// Critical hit chance.
    pub crit: Option<f64>,
    /// Attacks per second.
    pub attack_speed: Option<f64>,
    /// Physical damage, averaged across the printed range.
    pub physical: Option<f64>,
    /// Total elemental damage.
    pub elemental: Option<f64>,
    pub fire: Option<f64>,
    pub cold: Option<f64>,
    pub lightning: Option<f64>,
    pub chaos: Option<f64>,
    /// Reload time. PoE2 crossbows.
    pub reload: Option<f64>,
    /// Spirit. PoE2 sceptres.
    pub spirit: Option<f64>,
}

impl WeaponStats {
    /// Whether the item printed any offensive number at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Numbers on a map or a waystone.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapStats {
    pub blighted: Option<MapBlighted>,
    pub tier: Option<u32>,
    pub pack_size: Option<f64>,
    pub item_rarity: Option<f64>,
    pub revives: Option<u32>,
    pub drop_chance: Option<f64>,
    /// PoE1 only. Gone in PoE2 0.5.2.
    pub magic_monsters: Option<f64>,
    /// PoE1 only. Gone in PoE2 0.5.2.
    pub rare_monsters: Option<f64>,
    /// PoE1 only. Gone in PoE2 0.5.2.
    pub gold: Option<f64>,
    pub monster_rarity: Option<f64>,
    pub effectiveness: Option<f64>,
}

/// Gem sockets and links.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GemSockets {
    pub number: u32,
    /// Only ever 5 or 6. Anything smaller is not worth searching on.
    pub linked: Option<u32>,
    pub white: u32,
}

/// PoE2 rune and soul core sockets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AugmentSockets {
    pub empty: u32,
    pub current: u32,
    pub normal: u32,
}

/// A stack of currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSize {
    pub value: u32,
    pub max: u32,
}

/// What a heist blueprint has revealed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Heist {
    pub wings_revealed: Option<u32>,
    pub target: Option<HeistTarget>,
}

/// Attribute and level requirements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requirements {
    pub level: u32,
    pub str: u32,
    pub dex: u32,
    pub int: u32,
}

/// Ultimatum trial data. PoE2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Trials {
    pub number_of_trials: Option<u32>,
    pub ultimatum_hint: Option<UltimatumHint>,
}

/// A modifier line the parser could not match to a known stat.
///
/// Kept rather than dropped. An unknown modifier means our data is older than
/// the game, and the UI has to say so instead of quietly pricing a worse item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModifier {
    pub text: String,
    pub kind: ModifierType,
}

/// What the parser knows about the item base.
///
/// Filled from `items.ndjson` by the `GameData` lookup. Its own type rather
/// than a borrow so a parsed item can outlive the lookup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BaseInfo {
    pub name: String,
    pub reference_name: String,
    pub namespace: String,
    pub trade_discriminator: Option<String>,
    /// Whether the base can take crafted modifiers at all.
    pub craftable: bool,
    /// The map tier this base always has.
    ///
    /// A PoE2 waystone prints no tier of its own, so the parser reads it from
    /// here. None for everything that is not a map.
    pub map_tier: Option<u32>,
    /// The roll ranges the base can have.
    ///
    /// From the game bundles and not from the trade API, so absent for most
    /// bases today. The percentile reports nothing rather than guessing when
    /// it is missing.
    pub armour_bounds: ArmourBounds,
    /// The category this base belongs to.
    ///
    /// The item text names an item class, not a trade category, and the two do
    /// not line up. The data file holds the mapping, so the parser reads the
    /// category from the base rather than guessing from the class line.
    pub category: Option<ItemCategory>,
}

/// One parsed item.
///
/// Every field is optional because the game prints only what applies. A
/// currency stack has a stack size and nothing else. A rare bow has fifteen
/// fields set and no stack size.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedItem {
    pub rarity: Option<ItemRarity>,
    pub category: Option<ItemCategory>,
    pub item_level: Option<u32>,
    pub area_level: Option<u32>,
    pub gem_level: Option<u32>,
    pub talisman_tier: Option<u32>,
    pub quality: Option<i32>,
    /// Where the base rolls sit inside their possible range, 0 to 100.
    pub base_percentile: Option<f64>,

    pub armour: ArmourStats,
    pub weapon: WeaponStats,
    pub map: MapStats,

    pub gem_sockets: Option<GemSockets>,
    pub augment_sockets: Option<AugmentSockets>,
    pub stack_size: Option<StackSize>,
    pub requires: Option<Requirements>,
    pub heist: Option<Heist>,
    pub trials: Option<Trials>,

    pub is_unidentified: bool,
    pub is_corrupted: bool,
    pub is_unmodifiable: bool,
    pub is_mirrored: bool,
    pub is_sanctified: bool,
    pub is_synthesised: bool,
    pub is_fractured: bool,
    pub is_veiled: bool,
    pub is_foil: bool,

    pub influences: Vec<Influence>,
    pub sentinel_charge: Option<u32>,
    pub unidentified_tier: Option<u32>,

    /// Every modifier the parser matched, in the order the game printed them.
    ///
    /// Typed as the parse controller's own struct rather than a types struct,
    /// because a modifier is only meaningful next to the stats it granted and
    /// splitting the two would lose that pairing.
    pub modifiers: Vec<crate::controller::parse::shared::modifiers::ParsedModifier>,

    pub unknown_modifiers: Vec<UnknownModifier>,

    /// The rooms a Chronicle of Atzoatl carries.
    ///
    /// Empty for every other item. The rooms are what that item is worth.
    pub atzoatl_rooms: Vec<crate::controller::parse::shared::special::AtzoatlRoom>,

    /// The price note the seller wrote, verbatim.
    pub note: Option<String>,

    pub info: BaseInfo,

    /// The clipboard text this item came from.
    ///
    /// Kept so a bug report can carry the exact input. Every parser bug so far
    /// has been reproduced from one of these.
    pub raw_text: String,
}

impl ParsedItem {
    /// An item that was never on the clipboard.
    ///
    /// Ported from `createVirtualItem`. Used by the rune preview, which builds
    /// the item as it would be after the rune and runs it through the same
    /// filter code the real one goes through.
    ///
    /// The raw text says so outright. A virtual item that looked real would be
    /// indistinguishable in a bug report, and its text does not round trip
    /// through the parser.
    pub fn virtual_item(info: BaseInfo) -> Self {
        Self {
            info,
            raw_text: "VIRTUAL_ITEM".to_string(),
            ..Self::default()
        }
    }

    /// Whether this item never came from the clipboard.
    pub fn is_virtual(&self) -> bool {
        self.raw_text == "VIRTUAL_ITEM"
    }
}

impl ParsedItem {
    /// Whether the item can still take crafted modifiers.
    ///
    /// Ported from `itemIsModifiable`.
    pub fn is_modifiable(&self) -> bool {
        self.info.craftable && !self.is_corrupted && !self.is_mirrored && !self.is_sanctified
    }

    /// Whether the item has an influence.
    pub fn has_influence(&self, influence: Influence) -> bool {
        self.influences.contains(&influence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_round_trips() {
        for r in [
            ItemRarity::Normal,
            ItemRarity::Magic,
            ItemRarity::Rare,
            ItemRarity::Unique,
        ] {
            assert_eq!(ItemRarity::parse(r.as_str()), Some(r));
        }
    }

    #[test]
    fn rarity_rejects_anything_else() {
        // "Gem" and "Currency" appear on the Rarity line in some locales of
        // older clients. They are not rarities and must not parse as one.
        for s in ["Gem", "Currency", "rare", ""] {
            assert_eq!(ItemRarity::parse(s), None, "{s} parsed");
        }
    }

    #[test]
    fn influence_round_trips() {
        for i in [
            Influence::Crusader,
            Influence::Elder,
            Influence::Hunter,
            Influence::Redeemer,
            Influence::Shaper,
            Influence::Warlord,
        ] {
            assert_eq!(Influence::parse(i.as_str()), Some(i));
        }
    }

    #[test]
    fn influence_rejects_anything_else() {
        assert_eq!(Influence::parse("Searing Exarch"), None);
    }

    #[test]
    fn blight_ravaged_keeps_its_hyphen() {
        // The trade site matches this string exactly.
        assert_eq!(MapBlighted::BlightRavaged.as_str(), "Blight-ravaged");
    }

    #[test]
    fn a_default_item_is_empty_and_not_modifiable() {
        let item = ParsedItem::default();

        assert!(item.armour.is_empty());
        assert!(item.weapon.is_empty());
        assert!(!item.is_modifiable());
        assert!(item.influences.is_empty());
    }

    #[test]
    fn a_craftable_item_is_modifiable() {
        let item = ParsedItem {
            info: BaseInfo {
                craftable: true,
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        assert!(item.is_modifiable());
    }

    #[test]
    fn each_of_three_flags_blocks_crafting_on_its_own() {
        let base = ParsedItem {
            info: BaseInfo {
                craftable: true,
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        };

        for (label, item) in [
            (
                "corrupted",
                ParsedItem {
                    is_corrupted: true,
                    ..base.clone()
                },
            ),
            (
                "mirrored",
                ParsedItem {
                    is_mirrored: true,
                    ..base.clone()
                },
            ),
            (
                "sanctified",
                ParsedItem {
                    is_sanctified: true,
                    ..base.clone()
                },
            ),
        ] {
            assert!(!item.is_modifiable(), "{label} item was modifiable");
        }
    }

    #[test]
    fn an_uncraftable_base_is_never_modifiable() {
        let item = ParsedItem::default();

        assert!(!item.is_modifiable());
    }

    #[test]
    fn armour_stats_stop_being_empty_once_one_number_lands() {
        let mut a = ArmourStats::default();
        assert!(a.is_empty());

        a.es = Some(1.0);
        assert!(!a.is_empty());
    }

    #[test]
    fn weapon_stats_stop_being_empty_once_one_number_lands() {
        let mut w = WeaponStats::default();
        assert!(w.is_empty());

        w.spirit = Some(100.0);
        assert!(!w.is_empty());
    }

    #[test]
    fn influence_lookup_finds_only_what_is_present() {
        let item = ParsedItem {
            influences: vec![Influence::Shaper],
            ..ParsedItem::default()
        };

        assert!(item.has_influence(Influence::Shaper));
        assert!(!item.has_influence(Influence::Elder));
    }

    // -----------------------------------------------------------------
    // Virtual items
    // -----------------------------------------------------------------

    #[test]
    fn a_virtual_item_carries_the_base_it_was_built_from() {
        let base = BaseInfo {
            name: "Spine Bow".into(),
            reference_name: "Spine Bow".into(),
            ..BaseInfo::default()
        };

        let got = ParsedItem::virtual_item(base.clone());

        assert_eq!(got.info, base);
    }

    #[test]
    fn a_virtual_item_says_it_is_virtual() {
        // One that looked real would be indistinguishable in a bug report, and
        // its text does not round trip through the parser.
        assert!(ParsedItem::virtual_item(BaseInfo::default()).is_virtual());
    }

    #[test]
    fn a_parsed_item_is_not_virtual() {
        let item = ParsedItem {
            raw_text: "Item Class: Bows".into(),
            ..ParsedItem::default()
        };

        assert!(!item.is_virtual());
    }

    #[test]
    fn a_virtual_item_starts_with_nothing_on_it() {
        // The preview adds the rune's modifier itself, and inherited state
        // would show a modifier the rune did not add.
        let got = ParsedItem::virtual_item(BaseInfo::default());

        assert!(got.modifiers.is_empty());
        assert!(got.unknown_modifiers.is_empty());
        assert!(got.influences.is_empty());
        assert!(!got.is_corrupted);
        assert!(!got.is_unidentified);
    }
}
