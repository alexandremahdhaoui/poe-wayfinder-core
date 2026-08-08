use crate::types::client_strings as cs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    Augmented,
    Enchant,
    Fractured,
    Desecrated,
    Mutated,
    Sanctified,
    Unmet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayLine {
    pub text: String,
    pub tier: Option<String>,
    pub kind: LineKind,
}

impl DisplayLine {
    fn new(text: String, kind: LineKind) -> Self {
        Self {
            text,
            tier: None,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayProperty {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayItem {
    pub title: Vec<String>,
    pub properties: Vec<DisplayProperty>,
    pub granted_skills: Vec<DisplayProperty>,
    pub enchant_mods: Vec<DisplayLine>,
    pub rune_mods: Vec<DisplayLine>,
    pub implicit_mods: Vec<DisplayLine>,
    pub fractured_mods: Vec<DisplayLine>,
    pub explicit_mods: Vec<DisplayLine>,
    pub crafted_mods: Vec<DisplayLine>,
    pub desecrated_mods: Vec<DisplayLine>,
    pub mutated_mods: Vec<DisplayLine>,
    pub pseudo_mods: Vec<DisplayLine>,
    pub veiled_mods: Vec<DisplayLine>,
    pub tags: Vec<DisplayLine>,
}

pub fn parse_affix_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);

        let after = &rest[open + 1..];

        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);

            return out;
        };

        let inside = &after[..close];

        match inside.split_once('|') {
            Some((first, "")) => out.push_str(first),
            Some((_, second)) => out.push_str(second),
            None => out.push_str(inside),
        }

        rest = &after[close + 1..];
    }

    out.push_str(rest);

    out
}

pub fn tier_at(
    display_index: usize,
    tiers: &[Option<String>],
    hashes: &[Vec<usize>],
) -> Option<String> {
    if tiers.is_empty() {
        return None;
    }

    let indexes = hashes.get(display_index)?;

    let joined: Vec<String> = indexes
        .iter()
        .filter_map(|&i| tiers.get(i).cloned().flatten())
        .collect();

    if joined.is_empty() {
        return None;
    }

    Some(joined.join(" + "))
}

pub fn tier_of(tiers: &[Option<String>]) -> Option<String> {
    if tiers.is_empty() {
        return None;
    }

    let joined: Vec<String> = tiers.iter().flatten().cloned().collect();

    if joined.is_empty() {
        return None;
    }

    Some(joined.join(" + "))
}

pub fn mod_block(
    texts: &[String],
    kind: LineKind,
    tiers: &[Option<String>],
    hashes: &[Vec<usize>],
) -> Vec<DisplayLine> {
    texts
        .iter()
        .enumerate()
        .map(|(index, text)| DisplayLine {
            text: parse_affix_strings(text),
            tier: tier_at(index, tiers, hashes),
            kind,
        })
        .collect()
}

pub fn item_properties(
    item_level: Option<u32>,
    requirements: &[(String, String)],
) -> Vec<DisplayProperty> {
    let mut out = Vec::new();

    if let Some(level) = item_level.filter(|l| *l > 0) {
        out.push(DisplayProperty {
            label: "Item Level".to_string(),
            value: level.to_string(),
        });
    }

    if let Some((name, value)) = requirements.first() {
        let mut rendered = value.clone();

        for (other_name, other_value) in &requirements[1..] {
            rendered.push_str(", ");
            rendered.push_str(other_value);
            rendered.push(' ');
            rendered.push_str(other_name);
        }

        out.push(DisplayProperty {
            label: parse_affix_strings(name),
            value: parse_affix_strings(&rendered),
        });
    }

    out
}

pub fn granted_skills(skills: &[(String, String)]) -> Vec<DisplayProperty> {
    skills
        .iter()
        .map(|(name, value)| DisplayProperty {
            label: parse_affix_strings(name),
            value: parse_affix_strings(value),
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemFlags {
    pub identified: bool,
    pub unidentified_tier: Option<u32>,
    pub corrupted: bool,
    pub double_corrupted: bool,
    pub mirrored: bool,
    pub sanctified: bool,
}

impl ItemFlags {
    pub fn identified() -> Self {
        Self {
            identified: true,
            ..Self::default()
        }
    }
}

pub fn item_tags(flags: &ItemFlags) -> Vec<DisplayLine> {
    let mut out = Vec::new();

    if !flags.identified {
        out.push(DisplayLine::new(
            match flags.unidentified_tier {
                Some(tier) => format!("Unidentified (Tier {tier})"),
                None => cs::UNIDENTIFIED.to_string(),
            },
            LineKind::Unmet,
        ));
    }

    if flags.double_corrupted {
        out.push(DisplayLine::new(
            cs::DOUBLE_CORRUPTED.to_string(),
            LineKind::Unmet,
        ));
    } else if flags.corrupted {
        out.push(DisplayLine::new(cs::CORRUPTED.to_string(), LineKind::Unmet));
    }

    if flags.mirrored {
        out.push(DisplayLine::new(
            cs::MIRRORED.to_string(),
            LineKind::Augmented,
        ));
    }

    if flags.sanctified {
        out.push(DisplayLine::new(
            cs::SANCTIFIED.to_string(),
            LineKind::Sanctified,
        ));
    }

    out
}

pub fn veiled_mods(veiled: &[String]) -> Vec<DisplayLine> {
    veiled
        .iter()
        .map(|entry| {
            DisplayLine::new(
                if entry.starts_with("Prefix") {
                    "Unrevealed Prefix".to_string()
                } else {
                    "Unrevealed Suffix".to_string()
                },
                LineKind::Desecrated,
            )
        })
        .collect()
}

pub fn title(name: Option<&str>, type_line: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(name) = name.filter(|n| !n.is_empty()) {
        out.push(name.to_string());
    }

    if let Some(type_line) = type_line.filter(|t| !t.is_empty()) {
        out.push(type_line.to_string());
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_readable_half_of_a_pair_is_kept() {
        assert_eq!(
            parse_affix_strings("+#% to [Evasion|Evasion Rating]"),
            "+#% to Evasion Rating"
        );
    }

    #[test]
    fn a_lone_form_is_its_own_display_text() {
        assert_eq!(parse_affix_strings("[Evasion]"), "Evasion");
    }

    #[test]
    fn an_empty_second_form_falls_back_to_the_first() {
        assert_eq!(parse_affix_strings("[Evasion|]"), "Evasion");
    }

    #[test]
    fn several_pairs_in_one_line_are_all_resolved() {
        assert_eq!(
            parse_affix_strings("[a|A] and [b|B] and [c]"),
            "A and B and c"
        );
    }

    #[test]
    fn text_with_no_brackets_is_unchanged() {
        assert_eq!(
            parse_affix_strings("+25 to maximum Life"),
            "+25 to maximum Life"
        );
    }

    #[test]
    fn an_unterminated_bracket_is_shown_as_sent() {
        assert_eq!(parse_affix_strings("a [b|c"), "a [b|c");
    }

    #[test]
    fn empty_text_stays_empty() {
        assert_eq!(parse_affix_strings(""), "");
    }

    #[test]
    fn an_empty_pair_resolves_to_nothing() {
        assert_eq!(parse_affix_strings("a[]b"), "ab");
    }

    #[test]
    fn resolving_is_idempotent() {
        let once = parse_affix_strings("+#% to [Evasion|Evasion Rating]");

        assert_eq!(parse_affix_strings(&once), once);
    }

    #[test]
    fn non_ascii_text_survives() {
        assert_eq!(parse_affix_strings("é [a|ü] é"), "é ü é");
    }

    fn tiers(v: &[&str]) -> Vec<Option<String>> {
        v.iter().map(|s| Some((*s).to_string())).collect()
    }

    #[test]
    fn a_tier_is_read_through_the_hash_list_and_not_by_position() {
        let got = tier_at(0, &tiers(&["T3", "T1"]), &[vec![1], vec![0]]);

        assert_eq!(got.as_deref(), Some("T1"));
    }

    #[test]
    fn a_line_from_two_modifiers_shows_both_tiers() {
        let got = tier_at(0, &tiers(&["T1", "T2"]), &[vec![0, 1]]);

        assert_eq!(got.as_deref(), Some("T1 + T2"));
    }

    #[test]
    fn a_line_past_the_hash_list_has_no_tier() {
        assert_eq!(tier_at(5, &tiers(&["T1"]), &[vec![0]]), None);
    }

    #[test]
    fn no_metadata_means_no_tier() {
        assert_eq!(tier_at(0, &[], &[vec![0]]), None);
    }

    #[test]
    fn no_hashes_mean_no_tier() {
        assert_eq!(tier_at(0, &tiers(&["T1"]), &[]), None);
    }

    #[test]
    fn a_hash_pointing_past_the_metadata_is_skipped() {
        assert_eq!(
            tier_at(0, &tiers(&["T1"]), &[vec![0, 99]]).as_deref(),
            Some("T1")
        );
    }

    #[test]
    fn a_modifier_with_no_tier_of_its_own_is_skipped() {
        let got = tier_at(0, &[None, Some("T2".into())], &[vec![0, 1]]);

        assert_eq!(got.as_deref(), Some("T2"));
    }

    #[test]
    fn a_line_whose_modifiers_all_lack_tiers_has_none() {
        assert_eq!(tier_at(0, &[None, None], &[vec![0, 1]]), None);
    }

    #[test]
    fn a_grouped_response_needs_no_index_join() {
        assert_eq!(tier_of(&tiers(&["T1", "T2"])).as_deref(), Some("T1 + T2"));
    }

    #[test]
    fn an_empty_group_has_no_tier() {
        assert_eq!(tier_of(&[]), None);
        assert_eq!(tier_of(&[None]), None);
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_block_carries_its_text_tier_and_kind() {
        let got = mod_block(
            &strings(&["+25 to maximum Life"]),
            LineKind::Fractured,
            &tiers(&["T2"]),
            &[vec![0]],
        );

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].text, "+25 to maximum Life");
        assert_eq!(got[0].tier.as_deref(), Some("T2"));
        assert_eq!(got[0].kind, LineKind::Fractured);
    }

    #[test]
    fn a_block_resolves_alternative_spellings() {
        let got = mod_block(
            &strings(&["+#% to [Evasion|Evasion Rating]"]),
            LineKind::Normal,
            &[],
            &[],
        );

        assert_eq!(got[0].text, "+#% to Evasion Rating");
    }

    #[test]
    fn a_line_with_no_tier_data_still_shows_its_text() {
        let got = mod_block(
            &strings(&["+25 to maximum Life"]),
            LineKind::Normal,
            &[],
            &[],
        );

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].tier, None);
    }

    #[test]
    fn tier_data_shorter_than_the_text_does_not_drop_lines() {
        let got = mod_block(
            &strings(&["one", "two", "three"]),
            LineKind::Normal,
            &tiers(&["T1"]),
            &[vec![0]],
        );

        assert_eq!(got.len(), 3);
        assert_eq!(got[1].tier, None);
    }

    #[test]
    fn an_empty_block_produces_no_lines() {
        assert!(mod_block(&[], LineKind::Normal, &[], &[]).is_empty());
    }

    #[test]
    fn the_item_level_gets_its_own_row() {
        let got = item_properties(Some(84), &[]);

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].label, "Item Level");
        assert_eq!(got[0].value, "84");
    }

    #[test]
    fn an_item_level_of_zero_is_dropped() {
        assert!(item_properties(Some(0), &[]).is_empty());
    }

    #[test]
    fn no_level_and_no_requirements_produce_nothing() {
        assert!(item_properties(None, &[]).is_empty());
    }

    #[test]
    fn a_single_requirement_reads_as_itself() {
        let got = item_properties(None, &[("Level".into(), "68".into())]);

        assert_eq!(got[0].label, "Level");
        assert_eq!(got[0].value, "68");
    }

    #[test]
    fn several_requirements_fold_into_one_row() {
        let got = item_properties(
            None,
            &[
                ("Level".into(), "68".into()),
                ("Str".into(), "153".into()),
                ("Dex".into(), "82".into()),
            ],
        );

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].value, "68, 153 Str, 82 Dex");
    }

    #[test]
    fn a_requirement_resolves_alternative_spellings() {
        let got = item_properties(None, &[("[Strength|Str]".into(), "153".into())]);

        assert_eq!(got[0].label, "Str");
    }

    #[test]
    fn a_level_and_a_requirement_are_two_rows_in_order() {
        let got = item_properties(Some(84), &[("Level".into(), "68".into())]);

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].label, "Item Level");
        assert_eq!(got[1].label, "Level");
    }

    #[test]
    fn a_granted_skill_becomes_a_labelled_row() {
        let got = granted_skills(&[("[Skill|Grace]".into(), "Level 20".into())]);

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].label, "Grace");
        assert_eq!(got[0].value, "Level 20");
    }

    #[test]
    fn no_granted_skills_produce_no_rows() {
        assert!(granted_skills(&[]).is_empty());
    }

    #[test]
    fn a_plain_item_carries_no_tags() {
        assert!(item_tags(&ItemFlags::identified()).is_empty());
    }

    #[test]
    fn an_unidentified_item_says_so() {
        let got = item_tags(&ItemFlags::default());

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].kind, LineKind::Unmet);
    }

    #[test]
    fn a_known_unidentified_tier_is_shown() {
        let got = item_tags(&ItemFlags {
            unidentified_tier: Some(3),
            ..ItemFlags::default()
        });

        assert_eq!(got[0].text, "Unidentified (Tier 3)");
    }

    #[test]
    fn double_corruption_replaces_plain_corruption() {
        let got = item_tags(&ItemFlags {
            corrupted: true,
            double_corrupted: true,
            ..ItemFlags::identified()
        });

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].text, cs::DOUBLE_CORRUPTED);
    }

    #[test]
    fn a_corrupted_item_says_so() {
        let got = item_tags(&ItemFlags {
            corrupted: true,
            ..ItemFlags::identified()
        });

        assert_eq!(got[0].text, cs::CORRUPTED);
    }

    #[test]
    fn a_mirrored_item_says_so() {
        let got = item_tags(&ItemFlags {
            mirrored: true,
            ..ItemFlags::identified()
        });

        assert_eq!(got[0].text, cs::MIRRORED);
        assert_eq!(got[0].kind, LineKind::Augmented);
    }

    #[test]
    fn a_sanctified_item_says_so() {
        let got = item_tags(&ItemFlags {
            sanctified: true,
            ..ItemFlags::identified()
        });

        assert_eq!(got[0].kind, LineKind::Sanctified);
    }

    #[test]
    fn several_tags_appear_together() {
        let got = item_tags(&ItemFlags {
            corrupted: true,
            mirrored: true,
            sanctified: true,
            ..ItemFlags::identified()
        });

        assert_eq!(got.len(), 3);
    }

    #[test]
    fn a_veiled_modifier_names_the_half_it_occupies() {
        let got = veiled_mods(&strings(&["Prefix1", "Suffix2"]));

        assert_eq!(got[0].text, "Unrevealed Prefix");
        assert_eq!(got[1].text, "Unrevealed Suffix");
    }

    #[test]
    fn no_veiled_modifiers_produce_no_lines() {
        assert!(veiled_mods(&[]).is_empty());
    }

    #[test]
    fn a_rare_shows_its_rolled_name_above_its_base() {
        assert_eq!(
            title(Some("Rage Hold"), Some("Sapphire Ring")),
            vec!["Rage Hold", "Sapphire Ring"]
        );
    }

    #[test]
    fn a_white_base_shows_only_its_base() {
        assert_eq!(title(None, Some("Sapphire Ring")), vec!["Sapphire Ring"]);
    }

    #[test]
    fn an_empty_name_does_not_become_an_empty_line() {
        assert_eq!(
            title(Some(""), Some("Sapphire Ring")),
            vec!["Sapphire Ring"]
        );
    }

    #[test]
    fn a_listing_with_no_name_at_all_has_no_title() {
        assert!(title(None, None).is_empty());
    }
}
