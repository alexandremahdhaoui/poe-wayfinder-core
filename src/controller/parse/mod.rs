//! The item text parser.
//!
//! Ported from `renderer/src/parser/Parser.ts`.
//!
//! The algorithm is a section consuming ordered pipeline.
//!
//! 1. Split the clipboard on lines equal to `--------`.
//! 2. Lift the name plate out of section zero.
//! 3. Run an ordered list of stages over the sections that remain.
//!
//! Two rules drive everything. A stage consumes at most one section. A section
//! is consumed at most once. That is why the modifier stage appears five times
//! in the real pipeline. Each occurrence eats a different modifier section.

pub mod nameplate;
pub mod pipeline;
pub mod sections;
pub mod shared;

use crate::adapter::data_adapter::{GameData, NO_DATA};
use crate::types::item::ParsedItem;

pub use nameplate::{parse_name_plate, NamePlate};
pub use pipeline::pipeline;
pub use sections::text_to_sections;

/// What a stage did with a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseOutcome {
    /// The section is consumed and removed from the remaining list.
    SectionParsed,
    /// Not this section. Try the stage against the next one.
    SectionSkipped,
    /// This stage does not apply to this item at all. Move to the next stage.
    ParserSkipped,
}

/// Why parsing failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The text does not look like a copied item at all.
    NotAnItem,
    /// The name plate was missing a line the parser needs.
    BadNamePlate,
    /// The item was copied from a client in another language.
    ///
    /// The reference names the detected language. We support English only, so
    /// naming it would only point at a setting that does not exist here.
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

/// One section consuming stage.
pub type SectionStage = fn(&[String], &mut ParserState<'_>) -> ParseOutcome;

/// One stage that reads no section.
///
/// The reference calls these virtual parsers. They run once, in pipeline
/// order, and derive state from what earlier stages already set.
pub type VirtualStage = fn(&mut ParserState<'_>) -> Result<(), ParseError>;

/// A pipeline entry.
pub enum Stage {
    /// Consumes at most one section.
    Section(SectionStage),
    /// Reads no section.
    Virtual(VirtualStage),
}

/// The item under construction.
///
/// Carries two fields the finished item does not. `name` and `base_type` are
/// the raw name plate lines, and stages rewrite them before the database
/// lookup runs.
#[derive(Clone)]
pub struct ParserState<'a> {
    pub item: ParsedItem,
    /// Line 3 of the name plate. Always present.
    pub name: String,
    /// Line 4 of the name plate. Absent on normal items and currency.
    pub base_type: Option<String>,
    /// The stat and item tables.
    ///
    /// Carried on the state rather than passed to every stage, because only
    /// two of the fifty stages need it and threading it through the other
    /// forty eight would be noise.
    pub data: &'a dyn GameData,
}

impl std::fmt::Debug for ParserState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The data tables hold thousands of records and are never what a
        // reader wants to see in a failure message.
        f.debug_struct("ParserState")
            .field("item", &self.item)
            .field("name", &self.name)
            .field("base_type", &self.base_type)
            .finish_non_exhaustive()
    }
}

impl Default for ParserState<'_> {
    fn default() -> Self {
        Self {
            item: ParsedItem::default(),
            name: String::new(),
            base_type: None,
            data: &NO_DATA,
        }
    }
}

/// Parse clipboard text for a game.
///
/// The one call the rest of the project needs. It picks the pipeline and runs
/// it, so no caller has to know a pipeline exists.
pub fn parse_clipboard(
    clipboard: &str,
    game: crate::types::game::GameVersion,
    data: &dyn GameData,
) -> Result<ParsedItem, ParseError> {
    run(clipboard, &pipeline(game), data)
}

/// Run a pipeline over clipboard text.
///
/// The section list shrinks as stages consume from it, so a later stage never
/// sees a section an earlier stage already claimed.
pub fn run(
    clipboard: &str,
    pipeline: &[Stage],
    data: &dyn GameData,
) -> Result<ParsedItem, ParseError> {
    let mut sections = text_to_sections(clipboard);

    if sections.is_empty() {
        return Err(ParseError::NotAnItem);
    }

    // "You cannot use this item" sits inside the name plate and shifts every
    // line after it. Lift it out before the name plate is read.
    hoist_cannot_use_item(&mut sections);

    let plate = parse_name_plate(&sections[0])?;
    sections.remove(0);

    let mut state = ParserState {
        item: plate.item,
        name: plate.name,
        base_type: plate.base_type,
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

/// Move the "cannot use this item" line out of the name plate.
///
/// The game prints it as the third line of section zero. Everything the name
/// plate parser expects then sits one line late. The reference drops the line
/// and merges the rest of section zero into section one.
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
        let item = run(&plain_item(), &[], &NO_DATA).unwrap();

        assert_eq!(item.rarity, Some(ItemRarity::Rare));
        assert_eq!(item.raw_text, plain_item());
    }

    #[test]
    fn a_stage_sees_every_remaining_section() {
        let item = run(&plain_item(), &[Stage::Section(count_sections)], &NO_DATA).unwrap();

        assert_eq!(item.item_level, Some(2));
    }

    #[test]
    fn a_consumed_section_is_not_offered_again() {
        let pipeline = [Stage::Section(eat_first), Stage::Section(count_sections)];

        let item = run(&plain_item(), &pipeline, &NO_DATA).unwrap();

        assert_eq!(item.item_level, Some(1));
    }

    #[test]
    fn parser_skipped_stops_that_stage_without_consuming() {
        let pipeline = [Stage::Section(bail_out), Stage::Section(count_sections)];

        let item = run(&plain_item(), &pipeline, &NO_DATA).unwrap();

        // bail_out gave up on the first section and consumed nothing, so both
        // sections are still on offer.
        assert_eq!(item.item_level, Some(2));
        assert!(item.is_mirrored);
    }

    #[test]
    fn the_same_stage_twice_eats_two_sections() {
        // This is why the modifier stage appears five times in the real
        // pipeline. Repetition is how it reaches every modifier section.
        let pipeline = [
            Stage::Section(eat_first),
            Stage::Section(eat_first),
            Stage::Section(count_sections),
        ];

        let item = run(&plain_item(), &pipeline, &NO_DATA).unwrap();

        assert_eq!(item.item_level, None);
    }

    #[test]
    fn a_virtual_stage_runs_once_and_reads_no_section() {
        fn mark(state: &mut ParserState) -> Result<(), ParseError> {
            state.item.is_corrupted = true;

            Ok(())
        }

        let item = run(&plain_item(), &[Stage::Virtual(mark)], &NO_DATA).unwrap();

        assert!(item.is_corrupted);
    }

    #[test]
    fn a_virtual_stage_error_aborts_the_parse() {
        fn boom(_state: &mut ParserState) -> Result<(), ParseError> {
            Err(ParseError::NotAnItem)
        }

        assert_eq!(
            run(&plain_item(), &[Stage::Virtual(boom)], &NO_DATA),
            Err(ParseError::NotAnItem)
        );
    }

    #[test]
    fn empty_text_is_not_an_item() {
        assert_eq!(run("", &[], &NO_DATA), Err(ParseError::NotAnItem));
    }

    #[test]
    fn a_bad_name_plate_aborts_before_any_stage_runs() {
        assert_eq!(
            run("Rarity: Rare\nDoom", &[], &NO_DATA),
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

        let item = run(&text, &[], &NO_DATA).unwrap();

        // Without the hoist the name plate would read CANNOT_USE as the item
        // name and every later lookup would miss.
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
        // Merging forward with nothing to merge into would drop every line.
        let text = ["Item Class: Rings", "Rarity: Rare", CANNOT_USE].join("\n");

        let mut sections = text_to_sections(&text);
        hoist_cannot_use_item(&mut sections);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].len(), 3);
    }
}
