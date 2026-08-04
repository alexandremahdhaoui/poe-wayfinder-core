//! The name plate.
//!
//! Ported from `parseNamePlate` and `markupConditionParser` in
//! `renderer/src/parser/Parser.ts`.
//!
//! Section zero holds up to four lines.
//!
//! ```text
//! Item Class: Bows
//! Rarity: Rare
//! Doom Fletch
//! Spine Bow
//! ```
//!
//! The class line is always first. The rarity line is optional. What follows is
//! the name, then the base type when the item has one.

use crate::types::category::ItemCategory;
use crate::types::client_strings as cs;
use crate::types::item::{ItemRarity, ParsedItem};

use super::ParseError;

/// What the name plate yielded.
#[derive(Debug, Clone, PartialEq)]
pub struct NamePlate {
    pub item: ParsedItem,
    /// The item's own name. Always present.
    pub name: String,
    /// The base type line. Absent on normal items and on currency.
    pub base_type: Option<String>,
}

/// Rarity lines that name a category rather than a rarity.
///
/// Currency, divination cards and gems have no rarity of their own, so the
/// game reuses the rarity line to say what they are.
fn category_from_rarity_text(text: &str) -> Option<ItemCategory> {
    match text {
        cs::RARITY_CURRENCY => Some(ItemCategory::Currency),
        cs::RARITY_DIVCARD => Some(ItemCategory::DivinationCard),
        cs::RARITY_GEM => Some(ItemCategory::Gem),
        _ => None,
    }
}

/// Rarity lines that really are a rarity.
///
/// A quest item reads as Normal. The trade site has no quest rarity and quest
/// items are not tradeable, so folding them into Normal loses nothing.
fn rarity_from_rarity_text(text: &str) -> Option<ItemRarity> {
    match text {
        cs::RARITY_NORMAL | cs::RARITY_QUEST => Some(ItemRarity::Normal),
        cs::RARITY_MAGIC => Some(ItemRarity::Magic),
        cs::RARITY_RARE => Some(ItemRarity::Rare),
        cs::RARITY_UNIQUE => Some(ItemRarity::Unique),
        _ => None,
    }
}

/// Read section zero.
///
/// Fails when the class line is missing, because every later stage assumes the
/// remaining lines line up.
pub fn parse_name_plate(section: &[String]) -> Result<NamePlate, ParseError> {
    let mut lines = section.iter();

    let class_line = lines.next().ok_or(ParseError::BadNamePlate)?;

    // A meta gem prints no item class line at all, so its rarity line comes
    // first. Rejecting it would make every meta gem unparseable.
    if is_missing_item_class(section) {
        return parse_without_class_line(section);
    }

    if !class_line.starts_with(cs::ITEM_CLASS) {
        // A rarity line in another language is the common cause. Saying so
        // beats a generic parse failure, because the fix is to change the game
        // client language and nothing about this app.
        if section.iter().any(|l| looks_like_a_foreign_rarity_line(l)) {
            return Err(ParseError::WrongLanguage);
        }

        return Err(ParseError::BadNamePlate);
    }

    let mut item = ParsedItem::default();

    let mut next = lines.next();

    if let Some(line) = next {
        if let Some(rarity_text) = line.strip_prefix(cs::RARITY) {
            item.rarity = rarity_from_rarity_text(rarity_text);
            item.category = category_from_rarity_text(rarity_text);

            next = lines.next();
        }
    }

    let name = strip_markup(next.ok_or(ParseError::BadNamePlate)?);
    let base_type = lines.next().map(|l| strip_markup(l));

    Ok(NamePlate {
        item,
        name,
        base_type,
    })
}

/// Whether the item printed no class line.
///
/// Ported from `isItemMissingItemClass`. A meta gem is the only item that does
/// this, and the signal is a short first section whose first line is the
/// rarity.
///
/// The length bound matters. A longer section starting with a rarity line is a
/// normal item whose class line is missing for a different reason, and reading
/// it this way would misalign every line after it.
pub fn is_missing_item_class(section: &[String]) -> bool {
    if section.len() > 3 || section.len() < 2 {
        return false;
    }

    section[0].starts_with(cs::RARITY)
}

/// Read a name plate that carries no class line.
///
/// The rarity is first and the name second. Everything else follows the normal
/// rules.
fn parse_without_class_line(section: &[String]) -> Result<NamePlate, ParseError> {
    let mut lines = section.iter();

    let rarity_line = lines.next().ok_or(ParseError::BadNamePlate)?;

    let rarity_text = rarity_line
        .strip_prefix(cs::RARITY)
        .ok_or(ParseError::BadNamePlate)?;

    let mut item = ParsedItem {
        rarity: rarity_from_rarity_text(rarity_text),
        category: category_from_rarity_text(rarity_text),
        ..ParsedItem::default()
    };

    // Only a gem prints this way, and the reference forces the category for
    // exactly that reason.
    if item.category.is_none() {
        item.category = Some(ItemCategory::Gem);
    }

    let name = strip_markup(lines.next().ok_or(ParseError::BadNamePlate)?);
    let base_type = lines.next().map(|l| strip_markup(l));

    Ok(NamePlate {
        item,
        name,
        base_type,
    })
}

/// The rarity words of the eight non English clients the reference supports.
///
/// Present only to tell a user their client is not in English. We never parse
/// these items.
const FOREIGN_RARITY_LINES: [&str; 8] = [
    "Seltenheit: ",
    "Rareté: ",
    "Rareza: ",
    "Raridade: ",
    "Редкость: ",
    "아이템 희귀도: ",
    "稀有度: ",
    "レアリティ: ",
];

fn looks_like_a_foreign_rarity_line(line: &str) -> bool {
    FOREIGN_RARITY_LINES.iter().any(|p| line.starts_with(p))
}

/// Remove the game's inline markup from a name.
///
/// The game embeds conditional markup in item names. Two forms matter.
///
/// - `<<set:X>>` sets state. Always dropped.
/// - `<if:X>{body}` and friends. The reference always takes the first branch
///   and drops every `elif` and `else`, so we do the same.
///
/// Hand written rather than regex driven. The grammar is two tokens deep and a
/// scanner is easier to read than two nested patterns.
pub fn strip_markup(text: &str) -> String {
    let without_set = remove_set_directives(text);

    resolve_conditionals(&without_set)
}

/// Drop every `<<set:...>>` directive.
fn remove_set_directives(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("<<set:") {
        out.push_str(&rest[..start]);

        match rest[start..].find(">>") {
            Some(end) => rest = &rest[start + end + 2..],
            None => {
                // Unterminated. Keep it verbatim rather than eating the name.
                out.push_str(&rest[start..]);

                return out;
            }
        }
    }

    out.push_str(rest);

    out
}

/// Keep the body of an `if` branch and drop `elif` and `else` bodies.
fn resolve_conditionals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    loop {
        let Some(open) = rest.find('<') else {
            out.push_str(rest);

            return out;
        };

        let Some(close_rel) = rest[open..].find('>') else {
            out.push_str(rest);

            return out;
        };
        let close = open + close_rel;

        let tag = &rest[open + 1..close];
        let is_conditional = tag.starts_with("if:") || tag.starts_with("elif:") || tag == "else";

        if !is_conditional || rest.as_bytes().get(close + 1) != Some(&b'{') {
            out.push_str(&rest[..=close]);
            rest = &rest[close + 1..];

            continue;
        }

        let body_start = close + 2;
        let Some(body_end_rel) = rest[body_start..].find('}') else {
            out.push_str(rest);

            return out;
        };
        let body_end = body_start + body_end_rel;

        out.push_str(&rest[..open]);

        if tag.starts_with("if:") {
            out.push_str(&rest[body_start..body_end]);
        }

        rest = &rest[body_end + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_a_rare_item_with_a_base_type() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Bows",
            "Rarity: Rare",
            "Doom Fletch",
            "Spine Bow",
        ]))
        .unwrap();

        assert_eq!(plate.item.rarity, Some(ItemRarity::Rare));
        assert_eq!(plate.name, "Doom Fletch");
        assert_eq!(plate.base_type.as_deref(), Some("Spine Bow"));
    }

    #[test]
    fn reads_a_normal_item_with_no_base_type() {
        let plate =
            parse_name_plate(&lines(&["Item Class: Bows", "Rarity: Normal", "Spine Bow"])).unwrap();

        assert_eq!(plate.item.rarity, Some(ItemRarity::Normal));
        assert_eq!(plate.name, "Spine Bow");
        assert_eq!(plate.base_type, None);
    }

    #[test]
    fn a_quest_item_reads_as_normal() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Quest Items",
            "Rarity: Quest",
            "Medicine Chest",
        ]))
        .unwrap();

        assert_eq!(plate.item.rarity, Some(ItemRarity::Normal));
    }

    #[test]
    fn currency_gets_a_category_and_no_rarity() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Stackable Currency",
            "Rarity: Currency",
            "Chaos Orb",
        ]))
        .unwrap();

        assert_eq!(plate.item.rarity, None);
        assert_eq!(plate.item.category, Some(ItemCategory::Currency));
        assert_eq!(plate.name, "Chaos Orb");
    }

    #[test]
    fn a_divination_card_gets_its_category() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Divination Cards",
            "Rarity: Divination Card",
            "The Doctor",
        ]))
        .unwrap();

        assert_eq!(plate.item.category, Some(ItemCategory::DivinationCard));
    }

    #[test]
    fn a_gem_gets_its_category() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Skill Gems",
            "Rarity: Gem",
            "Fireball",
        ]))
        .unwrap();

        assert_eq!(plate.item.category, Some(ItemCategory::Gem));
        assert_eq!(plate.item.rarity, None);
    }

    #[test]
    fn a_missing_rarity_line_leaves_the_name_where_it_is() {
        // Some items print no rarity line at all. The third line must not be
        // read as a rarity and lost.
        let plate = parse_name_plate(&lines(&["Item Class: Maps", "Chimeral Wetlands"])).unwrap();

        assert_eq!(plate.item.rarity, None);
        assert_eq!(plate.name, "Chimeral Wetlands");
        assert_eq!(plate.base_type, None);
    }

    #[test]
    fn an_unknown_rarity_word_sets_neither_field() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Rings",
            "Rarity: Legendary",
            "Some Ring",
        ]))
        .unwrap();

        assert_eq!(plate.item.rarity, None);
        assert_eq!(plate.item.category, None);
    }

    #[test]
    fn a_meta_gem_prints_no_class_line_and_still_parses() {
        // Found by running our parser against the upstream project's own test
        // fixtures. A meta gem is the only item that does this, and rejecting
        // it made every one unparseable.
        let plate = parse_name_plate(&lines(&["Rarity: Gem", "Mirage Archer"])).unwrap();

        assert_eq!(plate.name, "Mirage Archer");
        assert_eq!(plate.item.category, Some(ItemCategory::Gem));
    }

    #[test]
    fn a_long_section_starting_with_a_rarity_is_not_a_meta_gem() {
        // Reading it that way would misalign every line after it.
        assert!(!is_missing_item_class(&lines(&[
            "Rarity: Rare",
            "Doom Fletch",
            "Spine Bow",
            "extra"
        ])));

        let err = parse_name_plate(&lines(&[
            "Rarity: Rare",
            "Doom Fletch",
            "Spine Bow",
            "extra",
        ]))
        .unwrap_err();

        assert_eq!(err, ParseError::BadNamePlate);
    }

    #[test]
    fn a_one_line_section_is_not_a_meta_gem() {
        assert!(!is_missing_item_class(&lines(&["Rarity: Gem"])));
    }

    #[test]
    fn a_section_not_starting_with_a_rarity_is_not_a_meta_gem() {
        assert!(!is_missing_item_class(&lines(&["Mirage Archer", "x"])));
    }

    #[test]
    fn an_empty_section_fails() {
        assert_eq!(parse_name_plate(&[]).unwrap_err(), ParseError::BadNamePlate);
    }

    #[test]
    fn a_class_line_with_no_name_after_it_fails() {
        assert_eq!(
            parse_name_plate(&lines(&["Item Class: Bows", "Rarity: Rare"])).unwrap_err(),
            ParseError::BadNamePlate
        );
    }

    #[test]
    fn a_foreign_client_is_named_as_such() {
        // The fix is to change the game client language, so the error has to
        // say that rather than blame the item.
        for line in [
            "Seltenheit: Selten",
            "Rareté: Rare",
            "Редкость: Редкий",
            "レアリティ: レア",
        ] {
            let err =
                parse_name_plate(&lines(&["Gegenstandsklasse: Bögen", line, "Name"])).unwrap_err();

            assert_eq!(err, ParseError::WrongLanguage, "{line}");
        }
    }

    #[test]
    fn plain_text_survives_markup_stripping_unchanged() {
        assert_eq!(strip_markup("Spine Bow"), "Spine Bow");
    }

    #[test]
    fn set_directives_are_dropped() {
        assert_eq!(strip_markup("<<set:MS>>Spine Bow"), "Spine Bow");
        assert_eq!(strip_markup("A<<set:X>>B<<set:Y>>C"), "ABC");
    }

    #[test]
    fn the_first_branch_wins_and_the_rest_are_dropped() {
        let got = strip_markup("<if:MS>{Superior }<elif:M>{Fine }<else>{Crude }Spine Bow");

        assert_eq!(got, "Superior Spine Bow");
    }

    #[test]
    fn an_elif_alone_is_dropped_entirely() {
        assert_eq!(strip_markup("<elif:M>{Fine }Spine Bow"), "Spine Bow");
    }

    #[test]
    fn an_else_alone_is_dropped_entirely() {
        assert_eq!(strip_markup("<else>{Crude }Spine Bow"), "Spine Bow");
    }

    #[test]
    fn set_and_conditional_markup_combine() {
        let got = strip_markup("<<set:MS>><if:MS>{Superior }<else>{}Spine Bow");

        assert_eq!(got, "Superior Spine Bow");
    }

    #[test]
    fn an_unterminated_set_directive_keeps_the_text() {
        // Dropping to the end of the string here would erase the item name and
        // the failure would look like a database miss.
        assert_eq!(strip_markup("Spine <<set:MS Bow"), "Spine <<set:MS Bow");
    }

    #[test]
    fn an_unterminated_conditional_keeps_the_text() {
        assert_eq!(
            strip_markup("<if:MS>{Superior Spine Bow"),
            "<if:MS>{Superior Spine Bow"
        );
    }

    #[test]
    fn an_unclosed_angle_bracket_keeps_the_text() {
        assert_eq!(strip_markup("Spine <Bow"), "Spine <Bow");
    }

    #[test]
    fn a_tag_with_no_body_brace_is_left_alone() {
        // Not markup we understand, so it stays. Silently deleting it would
        // corrupt a name we could otherwise look up.
        assert_eq!(strip_markup("Spine <if:MS> Bow"), "Spine <if:MS> Bow");
    }

    #[test]
    fn markup_is_stripped_from_the_base_type_too() {
        let plate = parse_name_plate(&lines(&[
            "Item Class: Bows",
            "Rarity: Rare",
            "<if:MS>{Doom }Fletch",
            "<<set:X>>Spine Bow",
        ]))
        .unwrap();

        assert_eq!(plate.name, "Doom Fletch");
        assert_eq!(plate.base_type.as_deref(), Some("Spine Bow"));
    }
}
