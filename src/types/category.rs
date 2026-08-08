#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemCategory {
    Unknown,
    Map,
    CapturedBeast,
    MetamorphSample,
    Helmet,
    BodyArmour,
    Gloves,
    Boots,
    Shield,
    Amulet,
    Belt,
    Ring,
    Flask,
    AbyssJewel,
    Jewel,
    Quiver,
    Claw,
    Bow,
    Sceptre,
    Wand,
    FishingRod,
    Staff,
    Warstaff,
    Dagger,
    RuneDagger,
    OneHandedAxe,
    TwoHandedAxe,
    OneHandedMace,
    TwoHandedMace,
    OneHandedSword,
    TwoHandedSword,
    ClusterJewel,
    HeistBlueprint,
    HeistContract,
    HeistTool,
    HeistBrooch,
    HeistGear,
    HeistCloak,
    Trinket,
    Invitation,
    Gem,
    Currency,
    DivinationCard,
    Voidstone,
    Sentinel,
    MemoryLine,
    SanctumRelic,
    Tincture,
    Charm,
    Crossbow,
    SkillGem,
    SupportGem,
    MetaGem,
    UncutGem,
    Focus,
    Waystone,
    Relic,
    Tablet,
    Spear,
    Flail,
    Buckler,
    MapFragment,
    Talisman,
    Augment,
    Wombgift,
}

pub fn from_item_class(class: &str) -> Option<ItemCategory> {
    use ItemCategory::*;

    Some(match class.trim() {
        "Claws" => Claw,
        "Daggers" => Dagger,
        "Rune Daggers" => RuneDagger,
        "Wands" => Wand,
        "One Hand Swords" | "Thrusting One Hand Swords" => OneHandedSword,
        "One Hand Axes" => OneHandedAxe,
        "One Hand Maces" => OneHandedMace,
        "Sceptres" => Sceptre,
        "Spears" => Spear,
        "Flails" => Flail,

        "Bows" => Bow,
        "Staves" => Staff,
        "Two Hand Swords" => TwoHandedSword,
        "Two Hand Axes" => TwoHandedAxe,
        "Two Hand Maces" => TwoHandedMace,
        "Warstaves" | "Quarterstaves" => Warstaff,
        "Crossbows" => Crossbow,
        "Fishing Rods" => FishingRod,

        "Quivers" => Quiver,
        "Shields" => Shield,
        "Foci" => Focus,
        "Bucklers" => Buckler,

        "Amulets" => Amulet,
        "Rings" => Ring,
        "Belts" => Belt,
        "Trinkets" => Trinket,
        "Talismans" => Talisman,

        "Body Armours" => BodyArmour,
        "Helmets" => Helmet,
        "Gloves" => Gloves,
        "Boots" => Boots,

        "Life Flasks" | "Mana Flasks" | "Hybrid Flasks" | "Utility Flasks" => Flask,
        "Charms" => Charm,
        "Jewels" => Jewel,
        "Abyss Jewels" => AbyssJewel,
        "Maps" | "Waystones" => Map,
        "Stackable Currency" | "Currency" => Currency,
        "Divination Cards" => DivinationCard,
        "Relics" => SanctumRelic,
        "Tablet" | "Tablets" => Tablet,

        "Skill Gems" | "Support Gems" | "Uncut Skill Gems" | "Uncut Support Gems"
        | "Uncut Spirit Gems" => Gem,

        _ => return None,
    })
}

impl ItemCategory {
    pub fn as_str(self) -> &'static str {
        use ItemCategory::*;

        match self {
            Unknown => "Unknown",
            Map => "Map",
            CapturedBeast => "Captured Beast",
            MetamorphSample => "Metamorph Sample",
            Helmet => "Helmet",
            BodyArmour => "Body Armour",
            Gloves => "Gloves",
            Boots => "Boots",
            Shield => "Shield",
            Amulet => "Amulet",
            Belt => "Belt",
            Ring => "Ring",
            Flask => "Flask",
            AbyssJewel => "Abyss Jewel",
            Jewel => "Jewel",
            Quiver => "Quiver",
            Claw => "Claw",
            Bow => "Bow",
            Sceptre => "Sceptre",
            Wand => "Wand",
            FishingRod => "Fishing Rod",
            Staff => "Staff",
            Warstaff => "Warstaff",
            Dagger => "Dagger",
            RuneDagger => "Rune Dagger",
            OneHandedAxe => "One Hand Axe",
            TwoHandedAxe => "Two Hand Axe",
            OneHandedMace => "One Hand Mace",
            TwoHandedMace => "Two Hand Mace",
            OneHandedSword => "One Hand Sword",
            TwoHandedSword => "Two Hand Sword",
            ClusterJewel => "Cluster Jewel",
            HeistBlueprint => "Heist Blueprint",
            HeistContract => "Heist Contract",
            HeistTool => "Heist Tool",
            HeistBrooch => "Heist Brooch",
            HeistGear => "Heist Gear",
            HeistCloak => "Heist Cloak",
            Trinket => "Trinket",
            Invitation => "Invitation",
            Gem => "Gem",
            Currency => "Currency",
            DivinationCard => "Divination Card",
            Voidstone => "Voidstone",
            Sentinel => "Sentinel",
            MemoryLine => "Memory Line",
            SanctumRelic => "Sanctum Relic",
            Tincture => "Tincture",
            Charm => "Charm",
            Crossbow => "Crossbow",
            SkillGem => "Skill Gem",
            SupportGem => "Support Gem",
            MetaGem => "Meta Gem",
            UncutGem => "UncutSkillGem",
            Focus => "Focus",
            Waystone => "Waystone",
            Relic => "Relic",
            Tablet => "TowerAugment",
            Spear => "Spear",
            Flail => "Flail",
            Buckler => "Buckler",
            MapFragment => "MapFragment",
            Talisman => "Talisman",
            Augment => "Augment",
            Wombgift => "BrequelFruit",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        ALL.iter().copied().find(|c| c.as_str() == s)
    }

    pub fn is_one_handed_melee(self) -> bool {
        use ItemCategory::*;

        matches!(
            self,
            OneHandedAxe
                | OneHandedMace
                | OneHandedSword
                | Claw
                | Dagger
                | RuneDagger
                | Spear
                | Flail
        )
    }

    pub fn is_one_handed(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed_melee() || matches!(self, Sceptre | Wand)
    }

    pub fn is_two_handed_melee(self) -> bool {
        use ItemCategory::*;

        matches!(
            self,
            TwoHandedAxe | TwoHandedMace | TwoHandedSword | Warstaff | Talisman
        )
    }

    pub fn is_martial_weapon(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed_melee() || self.is_two_handed_melee() || matches!(self, Bow | Crossbow)
    }

    pub fn is_weapon(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed() || self.is_martial_weapon() || matches!(self, Staff | FishingRod)
    }

    pub fn is_armour(self) -> bool {
        use ItemCategory::*;

        matches!(
            self,
            BodyArmour | Boots | Gloves | Helmet | Shield | Focus | Buckler
        )
    }

    pub fn is_ring_or_amulet(self) -> bool {
        matches!(self, ItemCategory::Ring | ItemCategory::Amulet)
    }

    pub fn is_accessory(self) -> bool {
        use ItemCategory::*;

        matches!(self, Amulet | Belt | Ring | Trinket)
    }

    pub fn grants_real_skill(self) -> bool {
        use ItemCategory::*;

        matches!(self, Staff | Wand | Sceptre)
    }

    pub fn grants_skill(self) -> bool {
        use ItemCategory::*;

        self.grants_real_skill() || matches!(self, Spear | Shield | Buckler)
    }

    pub fn is_gem(self) -> bool {
        use ItemCategory::*;

        matches!(self, Gem | MetaGem | SupportGem)
    }
}

pub const ALL: [ItemCategory; 65] = {
    use ItemCategory::*;

    [
        Unknown,
        Map,
        CapturedBeast,
        MetamorphSample,
        Helmet,
        BodyArmour,
        Gloves,
        Boots,
        Shield,
        Amulet,
        Belt,
        Ring,
        Flask,
        AbyssJewel,
        Jewel,
        Quiver,
        Claw,
        Bow,
        Sceptre,
        Wand,
        FishingRod,
        Staff,
        Warstaff,
        Dagger,
        RuneDagger,
        OneHandedAxe,
        TwoHandedAxe,
        OneHandedMace,
        TwoHandedMace,
        OneHandedSword,
        TwoHandedSword,
        ClusterJewel,
        HeistBlueprint,
        HeistContract,
        HeistTool,
        HeistBrooch,
        HeistGear,
        HeistCloak,
        Trinket,
        Invitation,
        Gem,
        Currency,
        DivinationCard,
        Voidstone,
        Sentinel,
        MemoryLine,
        SanctumRelic,
        Tincture,
        Charm,
        Crossbow,
        SkillGem,
        SupportGem,
        MetaGem,
        UncutGem,
        Focus,
        Waystone,
        Relic,
        Tablet,
        Spear,
        Flail,
        Buckler,
        MapFragment,
        Talisman,
        Augment,
        Wombgift,
    ]
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_round_trips() {
        for c in ALL {
            assert_eq!(ItemCategory::parse(c.as_str()), Some(c), "{c:?}");
        }
    }

    #[test]
    fn every_spelling_is_unique() {
        let mut seen: Vec<&str> = ALL.iter().map(|c| c.as_str()).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), before, "two categories share a trade spelling");
    }

    #[test]
    fn parse_rejects_an_unknown_spelling() {
        assert_eq!(ItemCategory::parse("Trousers"), None);
    }

    #[test]
    fn the_odd_trade_spellings_are_preserved() {
        assert_eq!(ItemCategory::Tablet.as_str(), "TowerAugment");
        assert_eq!(ItemCategory::UncutGem.as_str(), "UncutSkillGem");
        assert_eq!(ItemCategory::Wombgift.as_str(), "BrequelFruit");
        assert_eq!(ItemCategory::MapFragment.as_str(), "MapFragment");
    }

    #[test]
    fn a_one_handed_melee_weapon_is_also_one_handed_and_martial_and_a_weapon() {
        let c = ItemCategory::Claw;

        assert!(c.is_one_handed_melee());
        assert!(c.is_one_handed());
        assert!(c.is_martial_weapon());
        assert!(c.is_weapon());
    }

    #[test]
    fn a_wand_is_one_handed_but_not_martial() {
        let c = ItemCategory::Wand;

        assert!(c.is_one_handed());
        assert!(!c.is_one_handed_melee());
        assert!(!c.is_martial_weapon());
        assert!(c.is_weapon());
    }

    #[test]
    fn a_bow_is_martial_but_neither_one_nor_two_handed_melee() {
        let c = ItemCategory::Bow;

        assert!(c.is_martial_weapon());
        assert!(!c.is_one_handed_melee());
        assert!(!c.is_two_handed_melee());
        assert!(c.is_weapon());
    }

    #[test]
    fn a_talisman_counts_as_a_two_handed_melee_weapon() {
        assert!(ItemCategory::Talisman.is_two_handed_melee());
        assert!(ItemCategory::Talisman.is_weapon());
    }

    #[test]
    fn a_fishing_rod_is_a_weapon_and_nothing_narrower() {
        let c = ItemCategory::FishingRod;

        assert!(c.is_weapon());
        assert!(!c.is_one_handed());
        assert!(!c.is_martial_weapon());
    }

    #[test]
    fn a_shield_is_armour_and_grants_a_skill() {
        let c = ItemCategory::Shield;

        assert!(c.is_armour());
        assert!(c.grants_skill());
        assert!(!c.grants_real_skill());
    }

    #[test]
    fn a_quiver_is_not_an_accessory() {
        assert!(!ItemCategory::Quiver.is_accessory());

        for c in [
            ItemCategory::Amulet,
            ItemCategory::Belt,
            ItemCategory::Ring,
            ItemCategory::Trinket,
        ] {
            assert!(c.is_accessory(), "{c:?}");
        }
    }

    #[test]
    fn an_uncut_gem_is_not_a_gem() {
        assert!(!ItemCategory::UncutGem.is_gem());

        for c in [
            ItemCategory::Gem,
            ItemCategory::MetaGem,
            ItemCategory::SupportGem,
        ] {
            assert!(c.is_gem(), "{c:?}");
        }
    }

    #[test]
    fn a_weapon_is_never_armour() {
        for c in ALL {
            assert!(
                !(c.is_weapon() && c.is_armour()),
                "{c:?} is both a weapon and armour"
            );
        }
    }

    #[test]
    fn unknown_belongs_to_no_group() {
        let c = ItemCategory::Unknown;

        assert!(!c.is_weapon());
        assert!(!c.is_armour());
        assert!(!c.is_accessory());
        assert!(!c.is_gem());
        assert!(!c.grants_skill());
    }
}
