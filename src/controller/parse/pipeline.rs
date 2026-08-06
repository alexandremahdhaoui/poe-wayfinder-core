//! The ordered stage lists.
//!
//! Ported from the `parsers` array in `renderer/src/parser/Parser.ts`.
//!
//! Order is the whole design. A stage that runs early claims its section
//! before a hungrier stage can, and the modifier stages run last so they only
//! ever see what nothing else wanted.
//!
//! 35 of the 50 reference stages are shared. Those live in `shared`. The two
//! builders here differ only in which game specific stages they splice in.

use crate::types::game::GameVersion;

use super::shared::{
    combat, content, database, flags, levels, misc, modifiers, sockets, special, variant,
};
use super::Stage;

/// Build the stage list for a game.
pub fn pipeline(game: GameVersion) -> Vec<Stage> {
    match game {
        GameVersion::Poe1 => poe1(),
        GameVersion::Poe2 => poe2(),
    }
}

/// Stages every item goes through before the base is known.
///
/// These run first because the later stages branch on the category and the
/// base name, and neither is known until the name plate has been cleaned up.
fn preamble() -> Vec<Stage> {
    vec![
        Stage::Section(flags::parse_unidentified),
        Stage::Virtual(content::parse_superior),
        Stage::Virtual(content::parse_exceptional),
        Stage::Virtual(content::parse_runeforged),
        Stage::Section(flags::parse_synthesised),
        Stage::Section(content::parse_category_by_help_text),
        // The name has to be normalised before the lookup, and the lookup has
        // to run before any stage that branches on the category or the base
        // name. Everything in `body` does.
        Stage::Virtual(database::normalize_name),
        Stage::Virtual(database::find_in_database),
    ]
}

/// Stages both games share, after the base is known.
fn body() -> Vec<Stage> {
    vec![
        Stage::Section(levels::parse_item_level),
        Stage::Section(levels::parse_requirements),
        Stage::Section(levels::parse_talisman_tier),
        Stage::Section(levels::parse_gem),
        Stage::Section(combat::parse_armour),
        Stage::Section(combat::parse_weapon),
        Stage::Section(misc::parse_flask),
        Stage::Section(misc::parse_price_note),
        Stage::Section(misc::parse_stack_size),
        Stage::Section(flags::parse_corrupted),
        Stage::Section(flags::parse_foil),
        Stage::Section(flags::parse_influence),
        Stage::Section(content::parse_map),
        Stage::Section(misc::parse_sockets),
        Stage::Section(content::parse_heist_blueprint),
        Stage::Section(content::parse_area_level),
        Stage::Section(flags::parse_mirrored),
        Stage::Section(flags::parse_sanctified),
        Stage::Section(flags::parse_fractured_flag),
        Stage::Section(misc::parse_sentinel_charge),
    ]
}

/// Five occurrences of the modifier stage.
///
/// One per modifier block the game can print: enchant, rune, implicit,
/// granted skill and explicit. Each occurrence claims a different section,
/// because a section is consumed at most once.
/// Stages for named bases whose modifier block breaks the normal rules.
///
/// These run before the general modifier stages so a named base's odd block is
/// claimed before the general reader can misread it. Each is gated on the base
/// name, so an ordinary item passes straight through.
///
/// The logbook stage appears three times because a logbook carries three
/// areas, each its own section.
fn special_base_stages() -> Vec<Stage> {
    vec![
        Stage::Section(special::parse_mirrored_tablet),
        Stage::Section(special::parse_filled_coffin),
        Stage::Section(special::parse_atzoatl_rooms),
        Stage::Section(special::parse_logbook_area),
        Stage::Section(special::parse_logbook_area),
        Stage::Section(special::parse_logbook_area),
    ]
}

/// Stages that read what the modifier stages produced.
fn derived_stages() -> Vec<Stage> {
    vec![
        Stage::Virtual(special::parse_fractured),
        Stage::Virtual(special::parse_blighted_map),
        // The variant is chosen after the modifiers are read, because the
        // implicit is what tells two bases of one name apart.
        Stage::Virtual(variant::pick_correct_variant),
        Stage::Virtual(special::calc_base_percentile),
    ]
}

fn modifier_stages() -> Vec<Stage> {
    (0..5)
        .map(|_| Stage::Section(modifiers::parse_modifiers))
        .collect()
}

/// The PoE2 modifier stages.
///
/// PoE2 prints every explicit in one unmarked section, so each line there is
/// its own modifier. The shared reader would call the whole section one
/// modifier and attribute six explicits to the first of them.
fn modifier_stages_poe2() -> Vec<Stage> {
    (0..5)
        .map(|_| Stage::Section(modifiers::parse_modifiers_poe2))
        .collect()
}

/// The PoE1 pipeline.
fn poe1() -> Vec<Stage> {
    let mut out = preamble();
    out.extend(body());
    out.extend([Stage::Section(misc::parse_timelost_radius)]);
    out.extend(special_base_stages());
    out.extend(modifier_stages());
    out.extend(derived_stages());

    out
}

/// The PoE2 pipeline.
///
/// PoE2 adds the caster, jewellery, charm slot, spirit, waystone and trial
/// stages. Each of those reads a line PoE1 never prints.
fn poe2() -> Vec<Stage> {
    let mut out = preamble();

    out.extend([
        Stage::Section(combat::parse_caster),
        Stage::Section(misc::parse_jewelery),
        Stage::Section(misc::parse_charm_slots),
        Stage::Section(misc::parse_spirit),
    ]);

    out.extend(body());

    out.extend([
        Stage::Section(content::parse_waystone),
        Stage::Section(content::parse_trials),
        Stage::Section(misc::parse_timelost_radius),
        // Runes are PoE2 only. The socket line has to be claimed before the
        // modifier stages, or it is read as an unknown modifier.
        Stage::Section(sockets::parse_augment_sockets),
    ]);

    out.extend(special_base_stages());
    out.extend(modifier_stages_poe2());

    // These read what the modifier stages produced, so they run last.
    out.extend(derived_stages());
    out.extend([
        Stage::Virtual(sockets::apply_augment_sockets),
        Stage::Virtual(sockets::apply_elemental_added),
    ]);

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::data_adapter::NO_DATA;
    use crate::controller::parse::run;
    use crate::types::item::{ItemRarity, StackSize};

    fn text(lines: &[&str]) -> String {
        lines.join("\n")
    }

    #[test]
    fn both_pipelines_are_non_empty() {
        assert!(!pipeline(GameVersion::Poe1).is_empty());
        assert!(!pipeline(GameVersion::Poe2).is_empty());
    }

    #[test]
    fn poe2_has_more_stages_than_poe1() {
        // PoE2 prints lines PoE1 never does. If this ever inverts, a stage was
        // spliced into the wrong builder.
        assert!(pipeline(GameVersion::Poe2).len() > pipeline(GameVersion::Poe1).len());
    }

    #[test]
    fn a_rare_bow_parses_end_to_end() {
        let item = run(
            &text(&[
                "Item Class: Bows",
                "Rarity: Rare",
                "Doom Fletch",
                "Spine Bow",
                "--------",
                "Physical Damage: 45-90",
                "Critical Hit Chance: 6.50%",
                "Attacks per Second: 1.35",
                "--------",
                "Requires: Level 68, 212 Dex",
                "--------",
                "Item Level: 84",
                "--------",
                "Corrupted",
            ]),
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert_eq!(item.rarity, Some(ItemRarity::Rare));
        assert_eq!(item.item_level, Some(84));
        assert_eq!(item.weapon.physical, Some(67.5));
        assert_eq!(item.weapon.crit, Some(6.5));
        assert_eq!(item.weapon.attack_speed, Some(1.35));
        assert_eq!(item.requires.unwrap().dex, 212);
        assert!(item.is_corrupted);
    }

    #[test]
    fn a_currency_stack_parses_end_to_end() {
        let item = run(
            &text(&[
                "Item Class: Stackable Currency",
                "Rarity: Currency",
                "Chaos Orb",
                "--------",
                "Stack Size: 1,240/20",
                "--------",
                "Reforges a rare item with new random modifiers.",
            ]),
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert_eq!(
            item.stack_size,
            Some(StackSize {
                value: 1240,
                max: 20
            })
        );
    }

    #[test]
    fn an_unidentified_superior_base_loses_its_prefix() {
        // parse_unidentified has to run before parse_superior, or the prefix
        // is kept and the database lookup misses.
        let item_text = text(&[
            "Item Class: Helmets",
            "Rarity: Rare",
            "Superior Iron Hat",
            "--------",
            "Unidentified",
            "--------",
            "Item Level: 70",
        ]);

        let item = run(
            &item_text,
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert!(item.is_unidentified);
        assert_eq!(item.item_level, Some(70));
    }

    #[test]
    fn a_synthesised_item_keeps_its_flag_and_loses_its_prefix() {
        let item = run(
            &text(&[
                "Item Class: Body Armours",
                "Rarity: Rare",
                "Doom Shell",
                "Synthesised Vaal Regalia",
                "--------",
                "Energy Shield: 220",
                "--------",
                "Synthesised Item",
                "--------",
                "Item Level: 84",
            ]),
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert!(item.is_synthesised);
        assert_eq!(item.armour.es, Some(220.0));
        assert_eq!(item.item_level, Some(84));
    }

    #[test]
    fn a_poe2_sceptre_consumes_its_spirit_section() {
        // Without parse_spirit that section would survive as an unparsed
        // section. This proves the PoE2 builder wires it in.
        let item = run(
            &text(&[
                "Item Class: Sceptres",
                "Rarity: Rare",
                "Doom Call",
                "Rattling Sceptre",
                "--------",
                "Spirit: 100",
                "--------",
                "Item Level: 78",
            ]),
            &pipeline(GameVersion::Poe2),
            &NO_DATA,
            GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.item_level, Some(78));
    }

    #[test]
    fn a_price_note_survives_the_whole_pipeline() {
        let item = run(
            &text(&[
                "Item Class: Rings",
                "Rarity: Rare",
                "Doom Loop",
                "Iron Ring",
                "--------",
                "Item Level: 60",
                "--------",
                "Note: ~price 5 divine",
            ]),
            &pipeline(GameVersion::Poe2),
            &NO_DATA,
            GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.note.as_deref(), Some("~price 5 divine"));
    }

    #[test]
    fn an_influence_section_is_read_on_poe1() {
        let item = run(
            &text(&[
                "Item Class: Gloves",
                "Rarity: Rare",
                "Doom Grip",
                "Sorcerer Gloves",
                "--------",
                "Energy Shield: 60",
                "--------",
                "Item Level: 84",
                "--------",
                "Shaper Item",
            ]),
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert_eq!(item.influences.len(), 1);
    }

    #[test]
    fn a_unique_reports_no_base_numbers() {
        let item = run(
            &text(&[
                "Item Class: Body Armours",
                "Rarity: Unique",
                "Kaom's Heart",
                "Glorious Plate",
                "--------",
                "Armour: 500",
                "--------",
                "Item Level: 84",
                "--------",
                "Corrupted",
            ]),
            &pipeline(GameVersion::Poe1),
            &NO_DATA,
            GameVersion::Poe1,
        )
        .unwrap();

        assert_eq!(item.rarity, Some(ItemRarity::Unique));
        assert!(item.armour.is_empty());
        assert!(item.is_corrupted);
    }
}
