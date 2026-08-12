#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub act: u32,
    pub from_level: u32,
    pub zone: String,
    pub doing: String,
}

pub fn guide() -> Vec<Step> {
    [
        (
            1,
            1,
            "The Riverbank",
            "kill Hillock, take the first support gem",
        ),
        (1, 4, "Clearfell", "find Una, pick up a second support"),
        (1, 8, "Mud Burrow", "kill the Devourer for a skill point"),
        (
            1,
            12,
            "Ogham Farmlands",
            "clear to the village, buy a two link",
        ),
        (2, 16, "Vastiri Outskirts", "trade for a movement skill"),
        (
            2,
            20,
            "Traitor's Passage",
            "kill Balbala, then the ziggurat",
        ),
        (
            2,
            24,
            "Valley of the Titans",
            "grab the ancient runed spike",
        ),
        (
            3,
            28,
            "Jungle Ruins",
            "check resistances before the Azak bog",
        ),
        (3, 32, "Infested Barrens", "clear for the Blackjaw arena"),
        (3, 36, "Drowned City", "cap fire resistance for Ignagduk"),
        (
            4,
            40,
            "Cruel difficulty",
            "resistances go negative, re-cap them",
        ),
        (4, 50, "Maps", "start at tier one and check every waystone"),
    ]
    .iter()
    .map(|(act, from_level, zone, doing)| Step {
        act: *act,
        from_level: *from_level,
        zone: (*zone).to_string(),
        doing: (*doing).to_string(),
    })
    .collect()
}

pub fn step_for(level: u32) -> Option<Step> {
    guide().into_iter().rfind(|step| step.from_level <= level)
}

pub fn upcoming(level: u32, how_many: usize) -> Vec<Step> {
    guide()
        .into_iter()
        .filter(|step| step.from_level > level)
        .take(how_many)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_starts_at_level_one_so_a_new_character_is_covered() {
        assert_eq!(guide()[0].from_level, 1);
        assert!(step_for(1).is_some());
    }

    #[test]
    fn every_step_is_further_along_than_the_one_before() {
        let steps = guide();

        for pair in steps.windows(2) {
            assert!(
                pair[1].from_level > pair[0].from_level,
                "{:?} does not come after {:?}",
                pair[1],
                pair[0]
            );
            assert!(pair[1].act >= pair[0].act);
        }
    }

    #[test]
    fn the_step_shown_is_the_last_one_reached() {
        let got = step_for(30).expect("a step");

        assert_eq!(got.from_level, 28);
        assert_eq!(got.act, 3);
    }

    #[test]
    fn a_level_past_the_end_still_shows_the_last_step() {
        let got = step_for(90).expect("a step");

        assert_eq!(got.from_level, 50);
    }

    #[test]
    fn a_level_below_the_first_step_has_none() {
        assert!(step_for(0).is_none());
    }

    #[test]
    fn what_comes_next_is_listed_without_repeating_where_you_are() {
        let next = upcoming(30, 2);

        assert_eq!(next.len(), 2);
        assert!(next.iter().all(|s| s.from_level > 30));
        assert_eq!(next[0].from_level, 32);
    }

    #[test]
    fn nothing_comes_after_the_last_step() {
        assert!(upcoming(99, 3).is_empty());
    }

    #[test]
    fn every_step_says_where_and_what() {
        for step in guide() {
            assert!(!step.zone.trim().is_empty());
            assert!(!step.doing.trim().is_empty());
            assert!(step.act >= 1);
        }
    }
}
