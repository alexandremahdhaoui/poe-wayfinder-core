//! Filters for uniques, anointments and incursion rooms.
//!
//! Ported from `create-unique-filters.ts`, `pseudo/anointments.ts` and
//! `pseudo/atzoatl-rules.ts`.
//!
//! Three small rule sets that share one shape: an item whose price comes from
//! something other than its modifier rolls.

use crate::controller::parse::shared::special::AtzoatlRoom;
use crate::types::item::ParsedItem;
use crate::types::query::Range;

/// How a unique should be searched for.
///
/// Ported from `createUniquePresets`. A unique's modifiers are fixed, so the
/// only things that vary are its name, whether it is corrupted, and the
/// handful of rolls that do move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniqueSearch {
    /// Search by name alone. Most uniques.
    ByName,
    /// Search by name and its item level, because the level gates its rolls.
    ByNameAndLevel,
    /// Search by name and the link count, which is most of the price.
    ByNameAndLinks,
}

/// Uniques whose item level changes what they roll.
///
/// Ported from the level gated list. Searching one of these by name alone
/// returns every copy including the ones that cannot roll what the buyer
/// wants.
const LEVEL_GATED: &[&str] = &[
    "Tabula Rasa",
    "The Anvil",
    "Shavronne's Wrappings",
    "Kaom's Heart",
    "Voll's Devotion",
];

/// Which search a unique calls for.
pub fn unique_search(item: &ParsedItem) -> UniqueSearch {
    let name = item.info.name.as_str();

    if LEVEL_GATED.contains(&name) {
        return UniqueSearch::ByNameAndLevel;
    }

    // A six link is most of what a body armour or two handed weapon is worth.
    if item
        .gem_sockets
        .is_some_and(|s| s.linked.is_some_and(|l| l >= 5))
    {
        return UniqueSearch::ByNameAndLinks;
    }

    UniqueSearch::ByName
}

/// The link filter a unique deserves.
///
/// A five link and a six link are different markets. A floor rather than an
/// exact match, because a six link satisfies someone shopping for a five.
pub fn link_filter(item: &ParsedItem, search: UniqueSearch) -> Option<Range> {
    if search != UniqueSearch::ByNameAndLinks {
        return None;
    }

    let linked = item.gem_sockets?.linked?;

    Some(Range::at_least(f64::from(linked)))
}

/// The item level filter a level gated unique deserves.
pub fn unique_level_filter(item: &ParsedItem, search: UniqueSearch) -> Option<Range> {
    if search != UniqueSearch::ByNameAndLevel {
        return None;
    }

    Some(Range::at_least(f64::from(item.item_level?)))
}

/// An anointment read off an amulet.
///
/// Ported from `anointments.ts`. The anointed passive is worth more than the
/// amulet on a cheap base, so it is filtered on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anointment {
    /// The passive the amulet grants.
    pub passive: String,
}

/// Read the anointment off an item.
///
/// The game prints it as an enchant naming the passive. Nothing else on an
/// amulet is an enchant, so the namespace alone identifies it.
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

            // The reference strips a trailing reminder about the passive
            // needing to be adjacent to the tree.
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

/// How much a Chronicle of Atzoatl is worth searching on.
///
/// Ported from `atzoatl-rules.ts`. Only a handful of rooms carry any price and
/// filtering on the rest returns nothing, because no two chronicles share a
/// full room list.
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

/// Which of an item's rooms are worth filtering on.
///
/// Only open ones. An obstructed room needs opening first and is not what the
/// buyer is paying for.
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
        // Searching one by name alone returns every copy including those that
        // cannot roll what the buyer wants.
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
        // A six link is most of what a body armour is worth.
        let item = ParsedItem {
            gem_sockets: Some(GemSockets {
                number: 6,
                linked: Some(6),
                white: 0,
            }),
            ..unique("Tabula Rasa")
        };

        // Level gating wins, because it is checked first.
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
        // A six link satisfies someone shopping for a five.
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
        // The anointed passive is worth more than the amulet on a cheap base.
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
        // Only an enchant is one. A timeless jewel allocates passives too.
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
        // No two chronicles share a full room list, so filtering on all of
        // them returns nothing.
        let rooms = [
            room("Hall of Metallurgy", true),
            room("Sparring Room", true),
            room("Storage Room", true),
        ];

        assert_eq!(valuable_rooms(&rooms), vec!["Hall of Metallurgy"]);
    }

    #[test]
    fn an_obstructed_room_is_not_filtered_on() {
        // It needs opening first and is not what the buyer is paying for.
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
        // A duplicate would send the same filter twice.
        let mut names = VALUABLE_ROOMS.to_vec();
        let before = names.len();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), before);
    }
}
