use crate::controller::parse::shared::special::AtzoatlRoom;
use crate::types::item::ParsedItem;
use crate::types::query::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniqueSearch {
    ByName,
    ByNameAndLevel,
    ByNameAndLinks,
}

const LEVEL_GATED: &[&str] = &[
    "Tabula Rasa",
    "The Anvil",
    "Shavronne's Wrappings",
    "Kaom's Heart",
    "Voll's Devotion",
];

pub fn unique_search(item: &ParsedItem) -> UniqueSearch {
    let name = item.info.name.as_str();

    if LEVEL_GATED.contains(&name) {
        return UniqueSearch::ByNameAndLevel;
    }

    if item
        .gem_sockets
        .is_some_and(|s| s.linked.is_some_and(|l| l >= 5))
    {
        return UniqueSearch::ByNameAndLinks;
    }

    UniqueSearch::ByName
}

pub fn link_filter(item: &ParsedItem, search: UniqueSearch) -> Option<Range> {
    if search != UniqueSearch::ByNameAndLinks {
        return None;
    }

    let linked = item.gem_sockets?.linked?;

    Some(Range::at_least(f64::from(linked)))
}

pub fn unique_level_filter(item: &ParsedItem, search: UniqueSearch) -> Option<Range> {
    if search != UniqueSearch::ByNameAndLevel {
        return None;
    }

    Some(Range::at_least(f64::from(item.item_level?)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anointment {
    pub passive: String,
}

pub fn anointment(item: &ParsedItem) -> Option<Anointment> {
    use crate::types::modifier::ModifierType;

    for modifier in &item.modifiers {
        if modifier.info.kind != Some(ModifierType::Enchant) {
            continue;
        }

        for stat in &modifier.stats {
            let Some(passive) = stat.matched.strip_prefix("Allocates ") else {
                continue;
            };

            let passive = passive
                .split(" (")
                .next()
                .unwrap_or(passive)
                .trim()
                .to_string();

            if passive.is_empty() {
                continue;
            }

            return Some(Anointment { passive });
        }
    }

    None
}

const VALUABLE_ROOMS: &[&str] = &[
    "Apex of Ascension",
    "Hall of Offerings",
    "Locus of Corruption",
    "Doryani's Institute",
    "Temple Nexus",
    "Chamber of Iron",
    "Hall of Metallurgy",
    "Sanctum of Unity",
    "Hybridisation Chamber",
    "Court of Sealed Death",
];

pub fn valuable_rooms(rooms: &[AtzoatlRoom]) -> Vec<&str> {
    rooms
        .iter()
        .filter(|r| r.open && VALUABLE_ROOMS.contains(&r.name.as_str()))
        .map(|r| r.name.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::parse::shared::modifiers::ParsedModifier;
    use crate::types::item::{BaseInfo, GemSockets, ItemRarity};
    use crate::types::modifier::{ModifierInfo, ModifierType};
    use crate::types::stat::ParsedStat;

    fn unique(name: &str) -> ParsedItem {
        ParsedItem {
            rarity: Some(ItemRarity::Unique),
            info: BaseInfo {
                name: name.into(),
                ..BaseInfo::default()
            },
            ..ParsedItem::default()
        }
    }

    fn room(name: &str, open: bool) -> AtzoatlRoom {
        AtzoatlRoom {
            name: name.into(),
            open,
        }
    }

    #[test]
    fn most_uniques_are_searched_by_name_alone() {
        assert_eq!(unique_search(&unique("Goldrim")), UniqueSearch::ByName);
    }

    #[test]
    fn a_level_gated_unique_carries_its_level() {
        for name in LEVEL_GATED {
            assert_eq!(
                unique_search(&unique(name)),
                UniqueSearch::ByNameAndLevel,
                "{name}"
            );
        }
    }

    #[test]
    fn a_linked_unique_carries_its_links() {
        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 6,
                linked: Some(6),
                white: 0,
            }),
            ..unique("Tabula Rasa")
        };

        assert_eq!(unique_search(&item), UniqueSearch::ByNameAndLevel);

        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 6,
                linked: Some(6),
                white: 0,
            }),
            ..unique("Belly of the Beast")
        };

        assert_eq!(unique_search(&item), UniqueSearch::ByNameAndLinks);
    }

    #[test]
    fn an_unlinked_unique_is_searched_by_name() {
        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 4,
                linked: None,
                white: 0,
            }),
            ..unique("Goldrim")
        };

        assert_eq!(unique_search(&item), UniqueSearch::ByName);
    }

    #[test]
    fn the_link_filter_is_a_floor() {
        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 6,
                linked: Some(5),
                white: 0,
            }),
            ..unique("Belly of the Beast")
        };

        let got = link_filter(&item, UniqueSearch::ByNameAndLinks).unwrap();

        assert_eq!(got.min, Some(5.0));
        assert_eq!(got.max, None);
    }

    #[test]
    fn a_name_only_search_sends_no_link_filter() {
        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 6,
                linked: Some(6),
                white: 0,
            }),
            ..unique("Goldrim")
        };

        assert_eq!(link_filter(&item, UniqueSearch::ByName), None);
    }

    #[test]
    fn a_level_gated_unique_sends_its_level_as_a_floor() {
        let item = ParsedItem {
            item_level: Some(84),
            ..unique("Kaom's Heart")
        };

        let got = unique_level_filter(&item, UniqueSearch::ByNameAndLevel).unwrap();

        assert_eq!(got.min, Some(84.0));
    }

    #[test]
    fn a_unique_with_no_item_level_sends_none() {
        assert_eq!(
            unique_level_filter(&unique("Kaom's Heart"), UniqueSearch::ByNameAndLevel),
            None
        );
    }

    #[test]
    fn an_anointment_is_read_from_the_enchant() {
        let item = ParsedItem {
            modifiers: vec![ParsedModifier {
                info: ModifierInfo {
                    kind: Some(ModifierType::Enchant),
                    ..ModifierInfo::default()
                },
                stats: vec![ParsedStat {
                    reference: "Allocates #".into(),
                    matched: "Allocates Charisma".into(),
                    roll: None,
                }],
            }],
            ..unique("Blue Pearl Amulet")
        };

        assert_eq!(
            anointment(&item),
            Some(Anointment {
                passive: "Charisma".into()
            })
        );
    }

    #[test]
    fn the_reminder_after_the_passive_name_is_dropped() {
        let item = ParsedItem {
            modifiers: vec![ParsedModifier {
                info: ModifierInfo {
                    kind: Some(ModifierType::Enchant),
                    ..ModifierInfo::default()
                },
                stats: vec![ParsedStat {
                    reference: "Allocates #".into(),
                    matched: "Allocates Charisma (must be adjacent)".into(),
                    roll: None,
                }],
            }],
            ..unique("Blue Pearl Amulet")
        };

        assert_eq!(anointment(&item).unwrap().passive, "Charisma");
    }

    #[test]
    fn an_explicit_allocation_is_not_an_anointment() {
        let item = ParsedItem {
            modifiers: vec![ParsedModifier {
                info: ModifierInfo {
                    kind: Some(ModifierType::Explicit),
                    ..ModifierInfo::default()
                },
                stats: vec![ParsedStat {
                    reference: "Allocates #".into(),
                    matched: "Allocates Charisma".into(),
                    roll: None,
                }],
            }],
            ..unique("Blue Pearl Amulet")
        };

        assert_eq!(anointment(&item), None);
    }

    #[test]
    fn an_item_with_no_anointment_reports_none() {
        assert_eq!(anointment(&unique("Goldrim")), None);
    }

    #[test]
    fn only_the_valuable_rooms_are_filtered_on() {
        let rooms = [
            room("Hall of Metallurgy", true),
            room("Sparring Room", true),
            room("Storage Room", true),
        ];

        assert_eq!(valuable_rooms(&rooms), vec!["Hall of Metallurgy"]);
    }

    #[test]
    fn an_obstructed_room_is_not_filtered_on() {
        let rooms = [room("Apex of Ascension", false)];

        assert!(valuable_rooms(&rooms).is_empty());
    }

    #[test]
    fn several_valuable_rooms_are_all_reported() {
        let rooms = [
            room("Apex of Ascension", true),
            room("Locus of Corruption", true),
        ];

        assert_eq!(valuable_rooms(&rooms).len(), 2);
    }

    #[test]
    fn no_rooms_yields_nothing() {
        assert!(valuable_rooms(&[]).is_empty());
    }

    #[test]
    fn every_valuable_room_name_is_unique() {
        let mut names = VALUABLE_ROOMS.to_vec();
        let before = names.len();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), before);
    }
}
