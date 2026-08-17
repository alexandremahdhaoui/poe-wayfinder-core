use crate::controller::parse::magic_name::mods_equal;
use crate::controller::parse::shared::modifiers::ParsedModifier;
use crate::types::modifier::Generation;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Change {
    pub added: Vec<ParsedModifier>,
    pub removed: Vec<ParsedModifier>,
}

impl Change {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

pub fn crafted_changes(before: &[ParsedModifier], after: &[ParsedModifier]) -> Change {
    let was: Vec<&ParsedModifier> = before.iter().filter(is_affix).collect();
    let now: Vec<&ParsedModifier> = after.iter().filter(is_affix).collect();

    Change {
        added: now
            .iter()
            .filter(|m| !was.iter().any(|old| mods_equal(m, old)))
            .map(|m| (*m).clone())
            .collect(),
        removed: was
            .iter()
            .filter(|m| !now.iter().any(|new| mods_equal(m, new)))
            .map(|m| (*m).clone())
            .collect(),
    }
}

pub fn caption(change: &Change) -> String {
    match (change.added.len(), change.removed.len()) {
        (0, 0) => "nothing changed since the last check".to_string(),
        (added, 0) => format!("{added} gained since the last check"),
        (0, removed) => format!("{removed} lost since the last check"),
        (added, removed) => format!("{added} gained, {removed} lost since the last check"),
    }
}

fn is_affix(modifier: &&ParsedModifier) -> bool {
    matches!(
        modifier.info.generation,
        Some(Generation::Prefix) | Some(Generation::Suffix)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::modifier::{Generation, ModifierInfo};
    use crate::types::stat::ParsedStat;

    fn affix(generation: Generation, reference: &str) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                generation: Some(generation),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: reference.to_string(),
                matched: reference.to_string(),
                roll: None,
            }],
        }
    }

    #[test]
    fn a_new_affix_is_reported_as_gained() {
        let before = vec![affix(Generation::Prefix, "+# to maximum Life")];
        let after = vec![
            affix(Generation::Prefix, "+# to maximum Life"),
            affix(Generation::Suffix, "+#% to Fire Resistance"),
        ];

        let change = crafted_changes(&before, &after);

        assert_eq!(change.added.len(), 1);
        assert!(change.removed.is_empty());
        assert_eq!(caption(&change), "1 gained since the last check");
    }

    #[test]
    fn an_affix_that_is_gone_is_reported_as_lost() {
        let before = vec![affix(Generation::Prefix, "+# to maximum Life")];

        let change = crafted_changes(&before, &[]);

        assert_eq!(change.removed.len(), 1);
        assert_eq!(caption(&change), "1 lost since the last check");
    }

    #[test]
    fn the_same_item_twice_changed_nothing() {
        let mods = vec![affix(Generation::Prefix, "+# to maximum Life")];

        let change = crafted_changes(&mods, &mods);

        assert!(change.is_empty());
        assert_eq!(caption(&change), "nothing changed since the last check");
    }

    #[test]
    fn a_reroll_is_both_a_loss_and_a_gain() {
        let before = vec![affix(Generation::Prefix, "+# to maximum Life")];
        let after = vec![affix(Generation::Prefix, "+# to maximum Mana")];

        let change = crafted_changes(&before, &after);

        assert_eq!(caption(&change), "1 gained, 1 lost since the last check");
    }

    #[test]
    fn a_corrupted_implicit_is_not_an_affix_and_is_ignored() {
        let before = vec![affix(Generation::Corrupted, "+# to maximum Life")];
        let after = vec![affix(Generation::Corrupted, "+# to maximum Mana")];

        assert!(crafted_changes(&before, &after).is_empty());
    }
}
