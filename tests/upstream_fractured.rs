//! Exiled Exchange 2's fractured item expectations, run against ours.
//!
//! Ported case for case from `renderer/specs/Parser/fractureParsers.test.ts`.
//!
//! # Why a whole file for one flag
//!
//! A fractured modifier cannot be changed by any craft, which is most of what
//! a fractured item is worth. The flag decides whether the query asks for one,
//! and both ways of getting it wrong are expensive: claiming an ordinary item
//! is fractured searches a market it is not in, and missing a real fracture
//! prices a fractured item as a plain one.
//!
//! The flag is derived rather than printed. The game marks the individual
//! modifier, and the item's own flag is inferred from the modifiers after they
//! are all read. That inference is what these cases pin.

use poe_trader_core::controller::parse::shared::modifiers::ParsedModifier;
use poe_trader_core::controller::parse::shared::special::parse_fractured;
use poe_trader_core::controller::parse::ParserState;
use poe_trader_core::types::modifier::{ModifierInfo, ModifierType};

/// A modifier of a kind, carrying no stats.
///
/// The reference's cases build exactly this: the flag is read off the kinds
/// alone and never looks at what the modifiers granted.
fn modifier(kind: ModifierType) -> ParsedModifier {
    ParsedModifier {
        info: ModifierInfo {
            kind: Some(kind),
            ..ModifierInfo::default()
        },
        stats: Vec::new(),
    }
}

fn item_with(kinds: &[ModifierType]) -> ParserState<'static> {
    let mut state = ParserState::default();
    state.item.modifiers = kinds.iter().copied().map(modifier).collect();

    state
}

#[test]
fn one_fractured_modifier_marks_the_item() {
    // The reference's first case: fractured, explicit, explicit.
    let mut state = item_with(&[
        ModifierType::Fractured,
        ModifierType::Explicit,
        ModifierType::Explicit,
    ]);

    parse_fractured(&mut state).unwrap();

    assert!(state.item.is_fractured);
}

#[test]
fn no_fractured_modifier_leaves_the_item_alone() {
    // The reference's second case: implicit, explicit. It expects the flag to
    // stay unset rather than be set to false, which for us is the same thing.
    let mut state = item_with(&[ModifierType::Implicit, ModifierType::Explicit]);

    parse_fractured(&mut state).unwrap();

    assert!(!state.item.is_fractured);
}

#[test]
fn an_item_with_no_modifiers_is_not_fractured() {
    // A currency or a gem reaches this stage with nothing. Marking it would
    // send a fractured filter on an item that cannot carry one.
    let mut state = item_with(&[]);

    parse_fractured(&mut state).unwrap();

    assert!(!state.item.is_fractured);
}

#[test]
fn the_fractured_modifier_can_be_anywhere_in_the_list() {
    // A check that only looked at the first modifier would pass the
    // reference's own case, where the fractured one happens to come first.
    for position in 0..4 {
        let mut kinds = vec![ModifierType::Explicit; 4];
        kinds[position] = ModifierType::Fractured;

        let mut state = item_with(&kinds);

        parse_fractured(&mut state).unwrap();

        assert!(state.item.is_fractured, "fractured modifier at {position}");
    }
}

#[test]
fn every_other_modifier_kind_leaves_the_flag_alone() {
    // Only fractured means fractured. A kind that reads as a near miss, like
    // desecrated or crafted, must not set it.
    for kind in [
        ModifierType::Explicit,
        ModifierType::Implicit,
        ModifierType::Crafted,
        ModifierType::Enchant,
        ModifierType::Desecrated,
        ModifierType::Augment,
        ModifierType::Veiled,
        ModifierType::Scourge,
        ModifierType::Pseudo,
    ] {
        let mut state = item_with(&[kind]);

        parse_fractured(&mut state).unwrap();

        assert!(!state.item.is_fractured, "{kind:?} set the fractured flag");
    }
}

#[test]
fn the_flag_is_never_turned_back_off() {
    // The stage only ever sets it. An item already known to be fractured, from
    // the section marker the game prints, must stay fractured even when no
    // modifier carries the kind.
    let mut state = item_with(&[ModifierType::Explicit]);
    state.item.is_fractured = true;

    parse_fractured(&mut state).unwrap();

    assert!(state.item.is_fractured);
}
