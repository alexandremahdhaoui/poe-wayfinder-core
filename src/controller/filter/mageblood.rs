#[derive(Debug, Clone, PartialEq)]
pub struct CountFilter {
    pub at_least: u32,
    pub floors: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Duplicates {
    Plain,
    AtStrength(u32),
    Counted(CountFilter),
}

pub const MAX_LEGACY: u32 = 4;

pub fn duplicates_filter(known: u32, duplicates: u32, active: bool) -> Option<Duplicates> {
    if known + duplicates > MAX_LEGACY {
        return None;
    }

    if !active || duplicates == 0 {
        return Some(Duplicates::Plain);
    }

    if known + duplicates == MAX_LEGACY {
        return Some(match duplicates {
            3 => Duplicates::AtStrength(4),
            2 => Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }),
            1 => Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }),
            _ => Duplicates::AtStrength(1),
        });
    }

    Some(Duplicates::Counted(CountFilter {
        at_least: 1,
        floors: vec![duplicates + 1],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_belt_with_no_duplicates_sends_its_readable_modifiers() {
        assert_eq!(duplicates_filter(4, 0, true), Some(Duplicates::Plain));
        assert_eq!(duplicates_filter(1, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn switching_the_duplicate_filter_off_sends_the_readable_modifiers() {
        assert_eq!(duplicates_filter(2, 2, false), Some(Duplicates::Plain));
    }

    #[test]
    fn one_modifier_filling_all_four_slots_is_asked_for_at_full_strength() {
        assert_eq!(
            duplicates_filter(1, 3, true),
            Some(Duplicates::AtStrength(4))
        );
    }

    #[test]
    fn two_duplicates_alongside_two_readable_becomes_a_count_filter() {
        assert_eq!(
            duplicates_filter(2, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 4,
                floors: vec![1, 2, 3],
            }))
        );
    }

    #[test]
    fn one_duplicate_alongside_three_readable_becomes_a_count_filter() {
        assert_eq!(
            duplicates_filter(3, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 7,
                floors: vec![1, 2],
            }))
        );
    }

    #[test]
    fn a_belt_with_an_empty_slot_asks_only_for_a_floor() {
        assert_eq!(
            duplicates_filter(1, 1, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![2],
            }))
        );
        assert_eq!(
            duplicates_filter(1, 2, true),
            Some(Duplicates::Counted(CountFilter {
                at_least: 1,
                floors: vec![3],
            }))
        );
    }

    #[test]
    fn a_belt_that_cannot_exist_reports_nothing() {
        assert_eq!(duplicates_filter(3, 3, true), None);
        assert_eq!(duplicates_filter(5, 0, true), None);
    }

    #[test]
    fn the_boundary_at_four_is_allowed() {
        assert!(duplicates_filter(4, 0, true).is_some());
        assert!(duplicates_filter(0, 4, true).is_some());
        assert!(duplicates_filter(4, 1, true).is_none());
    }

    #[test]
    fn a_belt_with_nothing_on_it_sends_nothing_special() {
        assert_eq!(duplicates_filter(0, 0, true), Some(Duplicates::Plain));
    }

    #[test]
    fn every_count_filter_asks_for_at_least_one() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                assert!(count.at_least >= 1, "{known} known {duplicates} dup");
                assert!(!count.floors.is_empty(), "{known} known {duplicates} dup");
            }
        }
    }

    #[test]
    fn every_floor_is_within_what_a_belt_can_carry() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=MAX_LEGACY {
                let Some(Duplicates::Counted(count)) = duplicates_filter(known, duplicates, true)
                else {
                    continue;
                };

                for floor in count.floors {
                    assert!(
                        (1..=MAX_LEGACY).contains(&floor),
                        "{known} known {duplicates} dup gave floor {floor}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_valid_combination_is_left_without_an_answer() {
        for known in 0..=MAX_LEGACY {
            for duplicates in 0..=(MAX_LEGACY - known) {
                assert!(
                    duplicates_filter(known, duplicates, true).is_some(),
                    "{known} known {duplicates} dup"
                );
            }
        }
    }
}
