//! The trade site query.
//!
//! Ported from the `TradeRequest` interface in
//! `web/price-check/trade/pathofexile-trade.ts`.
//!
//! # Why this is a type and not a JSON blob
//!
//! The trade API rejects an unknown key outright and silently ignores a filter
//! block whose inner shape is wrong. A wrong shape therefore returns a full
//! page of results that ignored half the query, which looks like a working
//! search and is not.
//!
//! The nesting is the API's and not ours. Every filter sits under
//! `filters.<group>.filters.<name>`, which reads oddly and is what the server
//! expects.

use std::collections::BTreeMap;

/// A numeric range filter.
///
/// Both ends are optional. A range with neither end set is meaningless and is
/// dropped rather than sent.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Range {
    /// A range with only a floor.
    pub fn at_least(min: f64) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// A range with only a ceiling.
    pub fn at_most(max: f64) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    /// A range pinned to one value.
    pub fn exactly(value: f64) -> Self {
        Self {
            min: Some(value),
            max: Some(value),
        }
    }

    /// Whether the range constrains anything.
    pub fn is_empty(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
}

/// Which listings to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// Sellers who are online now.
    #[default]
    Online,
    /// Every listing, including offline sellers.
    Any,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Online => "online",
            Status::Any => "any",
        }
    }
}

/// How a group of stat filters combines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatGroupKind {
    /// Every filter in the group must match.
    #[default]
    And,
    /// At least `value` of them must match.
    Count,
    /// None of them may match.
    Not,
    /// Match if present, ignore if not.
    If,
}

impl StatGroupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StatGroupKind::And => "and",
            StatGroupKind::Count => "count",
            StatGroupKind::Not => "not",
            StatGroupKind::If => "if",
        }
    }
}

/// One stat filter.
#[derive(Debug, Clone, PartialEq)]
pub struct StatFilter {
    /// The trade stat id, such as `explicit.stat_3299347043`.
    pub id: String,
    pub range: Range,
    /// A fixed option rather than a range.
    pub option: Option<f64>,
    /// Sent but not applied.
    ///
    /// A disabled filter still travels so the trade site's own UI shows it
    /// greyed out when the user opens the link.
    pub disabled: bool,
    /// Never shown and never sent.
    ///
    /// A filter can be built and still be wrong to offer. A corrupted item's
    /// implicit cannot be changed, so filtering on it narrows the search to
    /// items identical to the one in hand. It stays in the list so the code
    /// that groups filters still sees it, and it stays out of the UI.
    pub hidden: bool,
}

impl StatFilter {
    /// A filter on a range.
    pub fn range(id: &str, range: Range) -> Self {
        Self {
            id: id.to_string(),
            range,
            option: None,
            disabled: false,
            hidden: false,
        }
    }
}

/// A group of stat filters.
#[derive(Debug, Clone, PartialEq)]
pub struct StatGroup {
    pub kind: StatGroupKind,
    /// How many must match, for a count group.
    pub value: Range,
    pub filters: Vec<StatFilter>,
    pub disabled: bool,
}

impl StatGroup {
    /// A group where every filter must match.
    pub fn all(filters: Vec<StatFilter>) -> Self {
        Self {
            kind: StatGroupKind::And,
            value: Range::default(),
            filters,
            disabled: false,
        }
    }
}

/// Filters on what kind of item this is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypeFilters {
    /// `nonunique` or `uniquefoil`.
    pub rarity: Option<String>,
    /// The trade category id, such as `weapon.bow`.
    pub category: Option<String>,
    pub ilvl: Range,
    pub quality: Range,
}

/// Filters on a weapon or armour piece's numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EquipmentFilters {
    /// Attacks per second.
    pub aps: Range,
    /// Armour rating.
    pub ar: Range,
    pub block: Range,
    pub crit: Range,
    /// Damage per second.
    pub dps: Range,
    /// Elemental damage per second.
    pub edps: Range,
    /// Energy shield.
    pub es: Range,
    /// Evasion rating.
    pub ev: Range,
    /// Physical damage per second.
    pub pdps: Range,
    /// Rune and soul core sockets. Still called rune on the trade site.
    pub rune_sockets: Range,
    pub spirit: Range,
    pub reload_time: Range,
}

/// Filters on attribute requirements.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReqFilters {
    pub lvl: Range,
    pub str: Range,
    pub dex: Range,
    pub int: Range,
}

/// Filters on a map or waystone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapFilters {
    pub map_tier: Range,
    pub map_revives: Range,
    pub map_packsize: Range,
    pub map_magic_monsters: Range,
    pub map_rare_monsters: Range,
    /// Waystone drop chance.
    pub map_bonus: Range,
    /// Increased item rarity.
    pub map_iir: Range,
    pub ultimatum_hint: Option<String>,
}

/// A tri state boolean filter.
///
/// Absent means do not filter. `Some(true)` means require it. `Some(false)`
/// means exclude it. Sending false where absent was meant excludes every item
/// that has the property, which is a very different search.
pub type Flag = Option<bool>;

/// Filters that fit nowhere else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MiscFilters {
    pub alternate_art: Flag,
    pub area_level: Range,
    pub corrupted: Flag,
    pub gem_level: Range,
    pub gem_sockets: Range,
    pub identified: Flag,
    pub mirrored: Flag,
    pub sanctified: Flag,
    pub sanctum_gold: Range,
    pub unidentified_tier: Range,
    pub veiled: Flag,
    pub fractured_item: Flag,
}

/// Filters on the listing rather than the item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeFilters {
    /// Fold several listings by one seller into one row.
    pub collapse: Flag,
    /// How recently the listing was indexed.
    pub indexed: Option<String>,
    pub price: Range,
}

/// Every filter group.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filters {
    pub type_filters: TypeFilters,
    pub equipment_filters: EquipmentFilters,
    pub req_filters: ReqFilters,
    pub map_filters: MapFilters,
    pub misc_filters: MiscFilters,
    pub trade_filters: TradeFilters,
}

/// A name or type constrained by a discriminator.
///
/// A Two-Stone Ring is fire and cold or fire and lightning. Sending the bare
/// name searches for both, which prices the wrong item half the time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discriminated {
    pub option: String,
    pub discriminator: String,
}

/// A name or type field, with or without a discriminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameField {
    Plain(String),
    Discriminated(Discriminated),
}

impl NameField {
    /// Build from a name and an optional discriminator.
    pub fn new(name: &str, discriminator: Option<&str>) -> Self {
        match discriminator {
            Some(d) => NameField::Discriminated(Discriminated {
                option: name.to_string(),
                discriminator: d.to_string(),
            }),
            None => NameField::Plain(name.to_string()),
        }
    }

    /// The name, whichever form this is.
    pub fn name(&self) -> &str {
        match self {
            NameField::Plain(n) => n,
            NameField::Discriminated(d) => &d.option,
        }
    }
}

/// One search.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeQuery {
    pub status: Status,
    /// The unique's name. Absent on a rare.
    pub name: Option<NameField>,
    /// The base type.
    pub type_name: Option<NameField>,
    pub stats: Vec<StatGroup>,
    pub filters: Filters,
}

/// The category ids the trade site uses.
///
/// Ported from `CATEGORY_TO_TRADE_ID`. These are the server's spellings and
/// several do not follow from the category name. A guess here returns an
/// empty result rather than an error.
pub fn category_trade_ids() -> BTreeMap<&'static str, &'static str> {
    use crate::types::category::ItemCategory as C;

    [
        (C::Map, "map"),
        (C::AbyssJewel, "jewel.abyss"),
        (C::Amulet, "accessory.amulet"),
        (C::Belt, "accessory.belt"),
        (C::BodyArmour, "armour.chest"),
        (C::Boots, "armour.boots"),
        (C::Bow, "weapon.bow"),
        (C::Claw, "weapon.claw"),
        (C::Dagger, "weapon.dagger"),
        (C::FishingRod, "weapon.rod"),
        (C::Flask, "flask"),
        (C::Gloves, "armour.gloves"),
        (C::Helmet, "armour.helmet"),
        (C::Jewel, "jewel"),
        (C::OneHandedAxe, "weapon.oneaxe"),
        (C::OneHandedMace, "weapon.onemace"),
        (C::OneHandedSword, "weapon.onesword"),
        (C::Quiver, "armour.quiver"),
        (C::Ring, "accessory.ring"),
        (C::RuneDagger, "weapon.runedagger"),
        (C::Sceptre, "weapon.sceptre"),
        (C::Shield, "armour.shield"),
        (C::Staff, "weapon.staff"),
        (C::TwoHandedAxe, "weapon.twoaxe"),
        (C::TwoHandedMace, "weapon.twomace"),
        (C::TwoHandedSword, "weapon.twosword"),
        (C::Wand, "weapon.wand"),
        (C::Warstaff, "weapon.warstaff"),
        (C::ClusterJewel, "jewel.cluster"),
        (C::HeistBlueprint, "heistmission.blueprint"),
        (C::HeistContract, "heistmission.contract"),
        (C::HeistTool, "heistequipment.heisttool"),
        (C::HeistBrooch, "heistequipment.heistreward"),
        (C::HeistGear, "heistequipment.heistweapon"),
        (C::HeistCloak, "heistequipment.heistutility"),
        (C::Trinket, "accessory.trinket"),
        (C::SanctumRelic, "sanctum.relic"),
        (C::Tincture, "tincture"),
        (C::Charm, "flask.charm"),
        (C::Crossbow, "weapon.crossbow"),
        (C::SkillGem, "gem.activegem"),
        (C::SupportGem, "gem.supportgem"),
        (C::MetaGem, "gem.metagem"),
        (C::Focus, "armour.focus"),
        (C::Spear, "weapon.spear"),
        (C::Flail, "weapon.flail"),
        (C::Buckler, "armour.buckler"),
        (C::Tablet, "map.tablet"),
        (C::MapFragment, "map.fragment"),
        (C::Talisman, "weapon.talisman"),
        (C::Waystone, "map.waystone"),
    ]
    .into_iter()
    .map(|(c, id)| (c.as_str(), id))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::category::ItemCategory;

    #[test]
    fn an_empty_range_constrains_nothing() {
        assert!(Range::default().is_empty());
    }

    #[test]
    fn a_one_ended_range_constrains_something() {
        assert!(!Range::at_least(10.0).is_empty());
        assert!(!Range::at_most(10.0).is_empty());
    }

    #[test]
    fn at_least_sets_only_the_floor() {
        let r = Range::at_least(10.0);

        assert_eq!(r.min, Some(10.0));
        assert_eq!(r.max, None);
    }

    #[test]
    fn at_most_sets_only_the_ceiling() {
        let r = Range::at_most(10.0);

        assert_eq!(r.min, None);
        assert_eq!(r.max, Some(10.0));
    }

    #[test]
    fn exactly_pins_both_ends() {
        let r = Range::exactly(16.0);

        assert_eq!(r.min, Some(16.0));
        assert_eq!(r.max, Some(16.0));
    }

    #[test]
    fn the_default_status_is_online_sellers_only() {
        // Defaulting to any returns listings from sellers who will never
        // reply, which makes every price look lower than it is.
        assert_eq!(Status::default(), Status::Online);
        assert_eq!(Status::Online.as_str(), "online");
        assert_eq!(Status::Any.as_str(), "any");
    }

    #[test]
    fn the_default_stat_group_requires_every_filter() {
        assert_eq!(StatGroupKind::default(), StatGroupKind::And);
    }

    #[test]
    fn every_stat_group_kind_has_the_servers_spelling() {
        assert_eq!(StatGroupKind::And.as_str(), "and");
        assert_eq!(StatGroupKind::Count.as_str(), "count");
        assert_eq!(StatGroupKind::Not.as_str(), "not");
        assert_eq!(StatGroupKind::If.as_str(), "if");
    }

    #[test]
    fn a_stat_filter_starts_enabled() {
        let f = StatFilter::range("explicit.stat_1", Range::at_least(25.0));

        assert!(!f.disabled);
        assert_eq!(f.range.min, Some(25.0));
        assert_eq!(f.option, None);
    }

    #[test]
    fn a_plain_name_carries_no_discriminator() {
        let n = NameField::new("Sapphire Ring", None);

        assert_eq!(n, NameField::Plain("Sapphire Ring".into()));
        assert_eq!(n.name(), "Sapphire Ring");
    }

    #[test]
    fn a_discriminated_name_keeps_both_halves() {
        // A Two-Stone Ring is fire and cold or fire and lightning. Sending the
        // bare name searches for both and prices the wrong one half the time.
        let n = NameField::new("Two-Stone Ring", Some("fire_cold"));

        assert_eq!(n.name(), "Two-Stone Ring");
        assert_eq!(
            n,
            NameField::Discriminated(Discriminated {
                option: "Two-Stone Ring".into(),
                discriminator: "fire_cold".into()
            })
        );
    }

    #[test]
    fn a_flag_distinguishes_absent_from_false() {
        // Sending false where absent was meant excludes every item that has
        // the property, which is a very different search.
        let absent: Flag = None;
        let excluded: Flag = Some(false);

        assert_ne!(absent, excluded);
    }

    #[test]
    fn a_default_query_constrains_nothing() {
        let q = TradeQuery::default();

        assert_eq!(q.status, Status::Online);
        assert!(q.name.is_none());
        assert!(q.stats.is_empty());
        assert!(q.filters.type_filters.ilvl.is_empty());
        assert_eq!(q.filters.misc_filters.corrupted, None);
    }

    #[test]
    fn every_mapped_category_has_a_trade_id() {
        let ids = category_trade_ids();

        assert_eq!(ids.get(ItemCategory::Bow.as_str()), Some(&"weapon.bow"));
        assert_eq!(
            ids.get(ItemCategory::BodyArmour.as_str()),
            Some(&"armour.chest")
        );
        assert_eq!(
            ids.get(ItemCategory::Ring.as_str()),
            Some(&"accessory.ring")
        );
    }

    #[test]
    fn the_trade_ids_that_do_not_follow_from_the_name_are_preserved() {
        // Each of these would be guessed wrong. A wrong id returns an empty
        // result rather than an error.
        let ids = category_trade_ids();

        assert_eq!(
            ids.get(ItemCategory::HeistBrooch.as_str()),
            Some(&"heistequipment.heistreward")
        );
        assert_eq!(
            ids.get(ItemCategory::HeistGear.as_str()),
            Some(&"heistequipment.heistweapon")
        );
        assert_eq!(
            ids.get(ItemCategory::HeistCloak.as_str()),
            Some(&"heistequipment.heistutility")
        );
        assert_eq!(ids.get(ItemCategory::Tablet.as_str()), Some(&"map.tablet"));
        assert_eq!(
            ids.get(ItemCategory::Talisman.as_str()),
            Some(&"weapon.talisman")
        );
    }

    #[test]
    fn every_trade_id_is_unique() {
        let ids = category_trade_ids();

        let mut seen: Vec<&str> = ids.values().copied().collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), before, "two categories share a trade id");
    }

    #[test]
    fn a_category_with_no_trade_id_is_absent_rather_than_guessed() {
        // Currency and divination cards trade by name and have no category
        // filter. A guessed id would silently return nothing.
        let ids = category_trade_ids();

        assert_eq!(ids.get(ItemCategory::Currency.as_str()), None);
        assert_eq!(ids.get(ItemCategory::DivinationCard.as_str()), None);
        assert_eq!(ids.get(ItemCategory::Unknown.as_str()), None);
    }
}
