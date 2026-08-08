pub mod magic_name;
pub mod nameplate;
pub mod pipeline;
pub mod poe1;
pub mod sections;
pub mod shared;

use crate::adapter::data_adapter::{GameData, NO_DATA};
use crate::types::item::ParsedItem;

pub use nameplate::{parse_name_plate, NamePlate};
pub use pipeline::pipeline;
pub use sections::text_to_sections;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseOutcome {
    SectionParsed,
    SectionSkipped,
    ParserSkipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    NotAnItem,
    BadNamePlate,
    WrongLanguage,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NotAnItem => write!(f, "text is not a copied item"),
            ParseError::BadNamePlate => write!(f, "reading the item name plate"),
            ParseError::WrongLanguage => {
                write!(f, "item was copied from a client that is not English")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub type SectionStage = fn(&[String], &mut ParserState<'_>) -> ParseOutcome;

pub type VirtualStage = fn(&mut ParserState<'_>) -> Result<(), ParseError>;

pub enum Stage {
    Section(SectionStage),
    Virtual(VirtualStage),
}

#[derive(Clone)]
pub struct ParserState<'a> {
    pub item: ParsedItem,
    pub name: String,
    pub base_type: Option<String>,
    pub game: crate::types::game::GameVersion,
    pub variants: Vec<(
        crate::types::item::BaseInfo,
        crate::controller::parse::shared::variant::Discriminator,
    )>,
    pub data: &'a dyn GameData,
}

impl std::fmt::Debug for ParserState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserState")
            .field("item", &self.item)
            .field("name", &self.name)
            .field("base_type", &self.base_type)
            .field("game", &self.game)
            .finish_non_exhaustive()
    }
}

impl Default for ParserState<'_> {
    fn default() -> Self {
        Self {
            item: ParsedItem::default(),
            name: String::new(),
            base_type: None,
            game: crate::types::game::GameVersion::Poe2,
            variants: Vec::new(),
            data: &NO_DATA,
        }
    }
}

pub fn parse_clipboard(
    clipboard: &str,
    game: crate::types::game::GameVersion,
    data: &dyn GameData,
) -> Result<ParsedItem, ParseError> {
    run(clipboard, &pipeline(game), data, game)
}

pub fn run(
    clipboard: &str,
    pipeline: &[Stage],
    data: &dyn GameData,
    game: crate::types::game::GameVersion,
) -> Result<ParsedItem, ParseError> {
    let mut sections = text_to_sections(clipboard);

    if sections.is_empty() {
        return Err(ParseError::NotAnItem);
    }

    hoist_cannot_use_item(&mut sections);

    let plate = parse_name_plate(&sections[0])?;
    sections.remove(0);

    let mut state = ParserState {
        item: plate.item,
        name: plate.name,
        base_type: plate.base_type,
        variants: Vec::new(),
        game,
        data,
    };
    state.item.raw_text = clipboard.to_string();

    for stage in pipeline {
        match stage {
            Stage::Virtual(f) => f(&mut state)?,
            Stage::Section(f) => {
                let mut consumed = None;

                for (i, section) in sections.iter().enumerate() {
                    match f(section, &mut state) {
                        ParseOutcome::SectionParsed => {
                            consumed = Some(i);
                            break;
                        }
                        ParseOutcome::ParserSkipped => break,
                        ParseOutcome::SectionSkipped => {}
                    }
                }

                if let Some(i) = consumed {
                    sections.remove(i);
                }
            }
        }
    }

    Ok(state.item)
}

fn hoist_cannot_use_item(sections: &mut Vec<Vec<String>>) {
    use crate::types::client_strings::CANNOT_USE_ITEM;

    let is_present = sections
        .first()
        .and_then(|s| s.get(2))
        .is_some_and(|line| line == CANNOT_USE_ITEM);

    if !is_present || sections.len() < 2 {
        return;
    }

    let mut head = sections.remove(0);
    head.pop();

    for line in head.into_iter().rev() {
        sections[0].insert(0, line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::item::ItemRarity;

    const CANNOT_USE: &str = "You cannot use this item. Its stats will be ignored";

    fn plain_item() -> String {
        [
            "Item Class: Bows",
            "Rarity: Rare",
            "Doom Fletch",
            "Spine Bow",
            "--------",
            "Quality: +20% (augmented)",
            "--------",
            "Corrupted",
        ]
        .join("\n")
    }

    fn count_sections(_section: &[String], state: &mut ParserState) -> ParseOutcome {
        state.item.item_level = Some(state.item.item_level.unwrap_or(0) + 1);

        ParseOutcome::SectionSkipped
    }

    fn eat_first(_section: &[String], _state: &mut ParserState) -> ParseOutcome {
        ParseOutcome::SectionParsed
    }

    fn bail_out(_section: &[String], state: &mut ParserState) -> ParseOutcome {
        state.item.is_mirrored = true;

        ParseOutcome::ParserSkipped
    }

    #[test]
    fn the_name_plate_is_read_and_removed() {
        let item = run(
            &plain_item(),
            &[],
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.rarity, Some(ItemRarity::Rare));
        assert_eq!(item.raw_text, plain_item());
    }

    #[test]
    fn a_stage_sees_every_remaining_section() {
        let item = run(
            &plain_item(),
            &[Stage::Section(count_sections)],
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.item_level, Some(2));
    }

    #[test]
    fn a_consumed_section_is_not_offered_again() {
        let pipeline = [Stage::Section(eat_first), Stage::Section(count_sections)];

        let item = run(
            &plain_item(),
            &pipeline,
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.item_level, Some(1));
    }

    #[test]
    fn parser_skipped_stops_that_stage_without_consuming() {
        let pipeline = [Stage::Section(bail_out), Stage::Section(count_sections)];

        let item = run(
            &plain_item(),
            &pipeline,
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.item_level, Some(2));
        assert!(item.is_mirrored);
    }

    #[test]
    fn the_same_stage_twice_eats_two_sections() {
        let pipeline = [
            Stage::Section(eat_first),
            Stage::Section(eat_first),
            Stage::Section(count_sections),
        ];

        let item = run(
            &plain_item(),
            &pipeline,
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert_eq!(item.item_level, None);
    }

    #[test]
    fn a_virtual_stage_runs_once_and_reads_no_section() {
        fn mark(state: &mut ParserState) -> Result<(), ParseError> {
            state.item.is_corrupted = true;

            Ok(())
        }

        let item = run(
            &plain_item(),
            &[Stage::Virtual(mark)],
            &NO_DATA,
            crate::types::game::GameVersion::Poe2,
        )
        .unwrap();

        assert!(item.is_corrupted);
    }

    #[test]
    fn a_virtual_stage_error_aborts_the_parse() {
        fn boom(_state: &mut ParserState) -> Result<(), ParseError> {
            Err(ParseError::NotAnItem)
        }

        assert_eq!(
            run(
                &plain_item(),
                &[Stage::Virtual(boom)],
                &NO_DATA,
                crate::types::game::GameVersion::Poe2
            ),
            Err(ParseError::NotAnItem)
        );
    }

    #[test]
    fn empty_text_is_not_an_item() {
        assert_eq!(
            run("", &[], &NO_DATA, crate::types::game::GameVersion::Poe2),
            Err(ParseError::NotAnItem)
        );
    }

    #[test]
    fn a_bad_name_plate_aborts_before_any_stage_runs() {
        assert_eq!(
            run(
                "hello\nworld",
                &[],
                &NO_DATA,
                crate::types::game::GameVersion::Poe2
            ),
            Err(ParseError::BadNamePlate)
        );
    }

    #[test]
    fn every_error_prints_its_own_message() {
        let seen: Vec<String> = [
            ParseError::NotAnItem,
            ParseError::BadNamePlate,
            ParseError::WrongLanguage,
        ]
        .iter()
        .map(|e| e.to_string())
        .collect();

        assert_eq!(seen.len(), 3);
        assert!(seen.iter().all(|m| !m.is_empty()));
    }

    #[test]
    fn the_cannot_use_line_is_lifted_out_of_the_name_plate() {
        let text = [
            "Item Class: Body Armours",
            "Rarity: Rare",
            CANNOT_USE,
            "--------",
            "Doom Shell",
            "Astral Plate",
            "--------",
            "Corrupted",
        ]
        .join("\n");

        let item = run(&text, &[], &NO_DATA, crate::types::game::GameVersion::Poe2).unwrap();

        assert_eq!(item.rarity, Some(ItemRarity::Rare));
    }

    #[test]
    fn the_hoist_merges_the_name_plate_forward() {
        let text = [
            "Item Class: Body Armours",
            "Rarity: Rare",
            CANNOT_USE,
            "--------",
            "Doom Shell",
            "Astral Plate",
        ]
        .join("\n");

        let mut sections = text_to_sections(&text);
        hoist_cannot_use_item(&mut sections);

        assert_eq!(sections.len(), 1);
        assert_eq!(
            sections[0],
            vec![
                "Item Class: Body Armours",
                "Rarity: Rare",
                "Doom Shell",
                "Astral Plate"
            ]
        );
    }

    #[test]
    fn the_hoist_does_nothing_when_the_line_is_absent() {
        let mut sections = text_to_sections(&plain_item());
        let before = sections.clone();

        hoist_cannot_use_item(&mut sections);

        assert_eq!(sections, before);
    }

    #[test]
    fn the_hoist_does_nothing_when_there_is_no_second_section() {
        let text = ["Item Class: Rings", "Rarity: Rare", CANNOT_USE].join("\n");

        let mut sections = text_to_sections(&text);
        hoist_cannot_use_item(&mut sections);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].len(), 3);
    }
}
