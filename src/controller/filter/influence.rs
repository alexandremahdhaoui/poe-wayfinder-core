use crate::types::item::Influence;

pub fn stat_reference(influence: Influence) -> &'static str {
    match influence {
        Influence::Crusader => "Has Crusader Influence",
        Influence::Elder => "Has Elder Influence",
        Influence::Hunter => "Has Hunter Influence",
        Influence::Redeemer => "Has Redeemer Influence",
        Influence::Shaper => "Has Shaper Influence",
        Influence::Warlord => "Has Warlord Influence",
    }
}

pub fn filterable(influences: &[Influence]) -> &[Influence] {
    if influences.is_empty() || influences.len() > 2 {
        return &[];
    }

    influences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_influence_is_filterable() {
        assert_eq!(filterable(&[Influence::Shaper]), &[Influence::Shaper]);
    }

    #[test]
    fn two_influences_are_filterable() {
        let both = [Influence::Shaper, Influence::Elder];

        assert_eq!(filterable(&both), &both);
    }

    #[test]
    fn three_influences_are_not_filterable() {
        let three = [Influence::Shaper, Influence::Elder, Influence::Hunter];

        assert!(filterable(&three).is_empty());
    }

    #[test]
    fn no_influence_is_not_filterable() {
        assert!(filterable(&[]).is_empty());
    }

    #[test]
    fn every_influence_has_a_reference() {
        for influence in [
            Influence::Crusader,
            Influence::Elder,
            Influence::Hunter,
            Influence::Redeemer,
            Influence::Shaper,
            Influence::Warlord,
        ] {
            let reference = stat_reference(influence);

            assert!(reference.starts_with("Has "), "{reference}");
            assert!(reference.ends_with(" Influence"), "{reference}");
        }
    }

    #[test]
    fn no_two_influences_share_a_reference() {
        let all = [
            Influence::Crusader,
            Influence::Elder,
            Influence::Hunter,
            Influence::Redeemer,
            Influence::Shaper,
            Influence::Warlord,
        ];

        let mut seen: Vec<&str> = all.iter().map(|i| stat_reference(*i)).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();

        assert_eq!(seen.len(), before);
    }
}
