//! What kind of thing an item is.
//!
//! Ported from `renderer/src/parser/meta.ts`.
//!
//! The string form is the trade site's spelling and not ours. Several are odd.
//! `Tablet` is `TowerAugment`. `UncutGem` is `UncutSkillGem`. `Wombgift` is
//! `BrequelFruit`. Those are the values the trade API accepts, so they stay.

/// Every item category the parser recognises.
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

impl ItemCategory {
    /// The trade site spelling.
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

    /// Parse the trade site spelling.
    ///
    /// Data files carry these strings, so an unknown one means the data is
    /// newer than this build. The caller decides whether that is fatal.
    pub fn parse(s: &str) -> Option<Self> {
        ALL.iter().copied().find(|c| c.as_str() == s)
    }

    /// One handed melee weapon.
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

    /// One handed weapon, melee or caster.
    pub fn is_one_handed(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed_melee() || matches!(self, Sceptre | Wand)
    }

    /// Two handed melee weapon.
    pub fn is_two_handed_melee(self) -> bool {
        use ItemCategory::*;

        matches!(
            self,
            TwoHandedAxe | TwoHandedMace | TwoHandedSword | Warstaff | Talisman
        )
    }

    /// Martial weapon. Bows and crossbows plus every melee weapon.
    pub fn is_martial_weapon(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed_melee() || self.is_two_handed_melee() || matches!(self, Bow | Crossbow)
    }

    /// Any weapon.
    pub fn is_weapon(self) -> bool {
        use ItemCategory::*;

        self.is_one_handed() || self.is_martial_weapon() || matches!(self, Staff | FishingRod)
    }

    /// Any armour piece.
    pub fn is_armour(self) -> bool {
        use ItemCategory::*;

        matches!(
            self,
            BodyArmour | Boots | Gloves | Helmet | Shield | Focus | Buckler
        )
    }

    /// Bases a catalyst can scale.
    pub fn is_ring_or_amulet(self) -> bool {
        matches!(self, ItemCategory::Ring | ItemCategory::Amulet)
    }

    /// Rings, amulets, belts and trinkets.
    ///
    /// Quivers are excluded. The reference has that line commented out and the
    /// trade site agrees.
    pub fn is_accessory(self) -> bool {
        use ItemCategory::*;

        matches!(self, Amulet | Belt | Ring | Trinket)
    }

    /// Bases that can carry a real granted skill.
    pub fn grants_real_skill(self) -> bool {
        use ItemCategory::*;

        matches!(self, Staff | Wand | Sceptre)
    }

    /// Bases that can carry any granted skill.
    pub fn grants_skill(self) -> bool {
        use ItemCategory::*;

        self.grants_real_skill() || matches!(self, Spear | Shield | Buckler)
    }

    /// Gem categories.
    ///
    /// `UncutGem` is deliberately excluded, matching the reference. An uncut
    /// gem is currency until it is cut.
    pub fn is_gem(self) -> bool {
        use ItemCategory::*;

        matches!(self, Gem | MetaGem | SupportGem)
    }
}

/// Every category, in declaration order.
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
        // Each of these looks like a typo and is not. The trade API only
        // accepts these exact strings.
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
        // Surprising and correct. The reference puts Talisman in
        // WEAPON_TWO_HANDED_MELEE.
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
        // The reference comments Quiver out of ACCESSORY. The trade site
        // agrees, so this is intentional and not an omission.
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
        // An uncut gem trades as currency until it is cut.
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
