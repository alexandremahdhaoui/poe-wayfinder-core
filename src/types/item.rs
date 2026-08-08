use crate::types::category::ItemCategory;
use crate::types::modifier::ModifierType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemRarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

impl ItemRarity {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemRarity::Normal => "Normal",
            ItemRarity::Magic => "Magic",
            ItemRarity::Rare => "Rare",
            ItemRarity::Unique => "Unique",
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapBlighted {
    Blighted,
    BlightRavaged,
}

impl MapBlighted {
    pub fn as_str(self) -> &'static str {
        match self {
            MapBlighted::Blighted => "Blighted",
            MapBlighted::BlightRavaged => "Blight-ravaged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeistTarget {
    Enchants,
    Trinkets,
    Gems,
    Replicas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UltimatumHint {
    Victorious,
    Cowardly,
    Deadly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmourBounds {
    pub ar: Option<(f64, f64)>,
    pub ev: Option<(f64, f64)>,
    pub es: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmourStats {
    pub ar: Option<f64>,
    pub ev: Option<f64>,
    pub es: Option<f64>,
    pub rw: Option<f64>,
    pub block: Option<f64>,
}

impl ArmourStats {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponStats {
    pub crit: Option<f64>,
    pub attack_speed: Option<f64>,
    pub physical: Option<f64>,
    pub elemental: Option<f64>,
    pub fire: Option<f64>,
    pub cold: Option<f64>,
    pub lightning: Option<f64>,
    pub chaos: Option<f64>,
    pub reload: Option<f64>,
    pub spirit: Option<f64>,
}

impl WeaponStats {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapStats {
    pub blighted: Option<MapBlighted>,
    pub tier: Option<u32>,
    pub pack_size: Option<f64>,
    pub item_rarity: Option<f64>,
    pub revives: Option<u32>,
    pub drop_chance: Option<f64>,
    pub magic_monsters: Option<f64>,
    pub rare_monsters: Option<f64>,
    pub gold: Option<f64>,
    pub monster_rarity: Option<f64>,
    pub effectiveness: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GemSockets {
    pub number: u32,
    pub linked: Option<u32>,
    pub white: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AugmentSockets {
    pub empty: u32,
    pub current: u32,
    pub normal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSize {
    pub value: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Heist {
    pub wings_revealed: Option<u32>,
    pub target: Option<HeistTarget>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeistContract {
    pub required_job: Option<String>,
    pub job_level: Option<u32>,
    pub priceless: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requirements {
    pub level: u32,
    pub str: u32,
    pub dex: u32,
    pub int: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Trials {
    pub number_of_trials: Option<u32>,
    pub ultimatum_hint: Option<UltimatumHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModifier {
    pub text: String,
    pub kind: ModifierType,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BaseInfo {
    pub name: String,
    pub reference_name: String,
    pub namespace: String,
    pub trade_discriminator: Option<String>,
    pub craftable: bool,
    pub map_tier: Option<u32>,
    pub armour_bounds: ArmourBounds,
    pub trade_tag: Option<String>,
    pub category: Option<ItemCategory>,
    pub unique_base: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedItem {
    pub rarity: Option<ItemRarity>,
    pub category: Option<ItemCategory>,
    pub item_level: Option<u32>,
    pub area_level: Option<u32>,
    pub gem_level: Option<u32>,
    pub talisman_tier: Option<u32>,
    pub quality: Option<i32>,
    pub base_percentile: Option<f64>,

    pub armour: ArmourStats,
    pub weapon: WeaponStats,
    pub map: MapStats,

    pub gem_sockets: Option<GemSockets>,
    pub augment_sockets: Option<AugmentSockets>,
    pub stack_size: Option<StackSize>,
    pub requires: Option<Requirements>,
    pub heist: Option<Heist>,
    pub heist_contract: Option<HeistContract>,
    pub trials: Option<Trials>,
    pub map_completion_reward: Option<String>,
    pub memory_strands: Option<u32>,

    pub is_unidentified: bool,
    pub is_corrupted: bool,
    pub is_unmodifiable: bool,
    pub is_mirrored: bool,
    pub is_sanctified: bool,
    pub is_synthesised: bool,
    pub is_fractured: bool,
    pub is_veiled: bool,
    pub is_foil: bool,
    pub is_split: bool,
    pub is_vestigial: bool,
    pub is_foulborn: bool,
    pub is_imbued_gem: bool,

    pub influences: Vec<Influence>,
    pub sentinel_charge: Option<u32>,
    pub unidentified_tier: Option<u32>,

    pub modifiers: Vec<crate::controller::parse::shared::modifiers::ParsedModifier>,

    pub unknown_modifiers: Vec<UnknownModifier>,

    pub atzoatl_rooms: Vec<crate::controller::parse::shared::special::AtzoatlRoom>,

    pub note: Option<String>,

    pub info: BaseInfo,

    pub raw_text: String,
}

impl ParsedItem {
    pub fn virtual_item(info: BaseInfo) -> Self {
        Self {
            info,
            raw_text: "VIRTUAL_ITEM".to_string(),
            ..Self::default()
        }
    }

    pub fn is_virtual(&self) -> bool {
        self.raw_text == "VIRTUAL_ITEM"
    }
}

impl ParsedItem {
    pub fn is_modifiable(&self) -> bool {
        self.info.craftable && !self.is_corrupted && !self.is_mirrored && !self.is_sanctified
    }

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
        let got = ParsedItem::virtual_item(BaseInfo::default());

        assert!(got.modifiers.is_empty());
        assert!(got.unknown_modifiers.is_empty());
        assert!(got.influences.is_empty());
        assert!(!got.is_corrupted);
        assert!(!got.is_unidentified);
    }
}
