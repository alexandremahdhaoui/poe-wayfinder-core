use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Range {
    pub fn at_least(min: f64) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    pub fn at_most(max: f64) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    pub fn exactly(value: f64) -> Self {
        Self {
            min: Some(value),
            max: Some(value),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Online,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatGroupKind {
    #[default]
    And,
    Count,
    Not,
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

#[derive(Debug, Clone, PartialEq)]
pub struct StatFilter {
    pub id: String,
    pub range: Range,
    pub option: Option<f64>,
    pub disabled: bool,
    pub hidden: bool,
}

impl StatFilter {
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

#[derive(Debug, Clone, PartialEq)]
pub struct StatGroup {
    pub kind: StatGroupKind,
    pub value: Range,
    pub filters: Vec<StatFilter>,
    pub disabled: bool,
}

impl StatGroup {
    pub fn all(filters: Vec<StatFilter>) -> Self {
        Self {
            kind: StatGroupKind::And,
            value: Range::default(),
            filters,
            disabled: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypeFilters {
    pub rarity: Option<String>,
    pub category: Option<String>,
    pub ilvl: Range,
    pub quality: Range,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EquipmentFilters {
    pub aps: Range,
    pub ar: Range,
    pub block: Range,
    pub crit: Range,
    pub dps: Range,
    pub edps: Range,
    pub es: Range,
    pub ev: Range,
    pub pdps: Range,
    pub rune_sockets: Range,
    pub spirit: Range,
    pub reload_time: Range,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReqFilters {
    pub lvl: Range,
    pub str: Range,
    pub dex: Range,
    pub int: Range,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapFilters {
    pub map_tier: Range,
    pub map_revives: Range,
    pub map_packsize: Range,
    pub map_magic_monsters: Range,
    pub map_rare_monsters: Range,
    pub map_bonus: Range,
    pub map_iir: Range,
    pub ultimatum_hint: Option<String>,
}

pub type Flag = Option<bool>;

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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeFilters {
    pub collapse: Flag,
    pub indexed: Option<String>,
    pub price: Range,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filters {
    pub type_filters: TypeFilters,
    pub equipment_filters: EquipmentFilters,
    pub req_filters: ReqFilters,
    pub map_filters: MapFilters,
    pub misc_filters: MiscFilters,
    pub trade_filters: TradeFilters,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discriminated {
    pub option: String,
    pub discriminator: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameField {
    Plain(String),
    Discriminated(Discriminated),
}

impl NameField {
    pub fn new(name: &str, discriminator: Option<&str>) -> Self {
        match discriminator {
            Some(d) => NameField::Discriminated(Discriminated {
                option: name.to_string(),
                discriminator: d.to_string(),
            }),
            None => NameField::Plain(name.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            NameField::Plain(n) => n,
            NameField::Discriminated(d) => &d.option,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeQuery {
    pub status: Status,
    pub name: Option<NameField>,
    pub type_name: Option<NameField>,
    pub stats: Vec<StatGroup>,
    pub filters: Filters,
}

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
        let ids = category_trade_ids();

        assert_eq!(ids.get(ItemCategory::Currency.as_str()), None);
        assert_eq!(ids.get(ItemCategory::DivinationCard.as_str()), None);
        assert_eq!(ids.get(ItemCategory::Unknown.as_str()), None);
    }
}
