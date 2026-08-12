#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Happening {
    EnteredArea { name: String },
    LevelledUp { character: String, level: u32 },
    LeagueChanged { league: String },
    Whisper { from: String },
}

impl Happening {
    pub fn line(&self) -> String {
        match self {
            Happening::EnteredArea { name } => format!("entered {name}"),
            Happening::LevelledUp { character, level } => {
                format!("{character} reached level {level}")
            }
            Happening::LeagueChanged { league } => format!("league is now {league}"),
            Happening::Whisper { from } => format!("{from} whispered you"),
        }
    }

    pub fn is_worth_showing(&self) -> bool {
        !matches!(self, Happening::EnteredArea { name } if is_hideout(name))
    }
}

pub fn is_private_league(league: &str) -> bool {
    league.contains('(') && league.contains(')')
}

pub fn is_hideout(area: &str) -> bool {
    let lower = area.to_lowercase();

    lower.contains("hideout") || lower.contains("the rogue harbour")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Background {
    pub area: Option<String>,
    pub character: Option<String>,
    pub level: Option<u32>,
    pub whispers: usize,
}

impl Background {
    pub fn apply(&mut self, happening: &Happening) -> bool {
        match happening {
            Happening::EnteredArea { name } => {
                if self.area.as_deref() == Some(name.as_str()) {
                    return false;
                }

                self.area = Some(name.clone());
            }
            Happening::LevelledUp { character, level } => {
                self.character = Some(character.clone());
                self.level = Some(*level);
            }
            Happening::LeagueChanged { .. } => {}
            Happening::Whisper { .. } => self.whispers += 1,
        }

        true
    }

    pub fn where_you_are(&self) -> String {
        match &self.area {
            Some(area) => area.clone(),
            None => "not in game".to_string(),
        }
    }

    pub fn who_you_are(&self) -> String {
        match (&self.character, self.level) {
            (Some(name), Some(level)) => format!("{name}, level {level}"),
            (Some(name), None) => name.clone(),
            _ => "no character seen".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_an_area_is_remembered() {
        let mut bg = Background::default();

        assert!(bg.apply(&Happening::EnteredArea {
            name: "Clearfell".into()
        }));
        assert_eq!(bg.where_you_are(), "Clearfell");
    }

    #[test]
    fn entering_the_same_area_twice_is_not_news() {
        let mut bg = Background::default();
        let entered = Happening::EnteredArea {
            name: "Clearfell".into(),
        };

        bg.apply(&entered);

        assert!(!bg.apply(&entered), "the log repeats itself on a reconnect");
    }

    #[test]
    fn a_hideout_is_not_worth_announcing() {
        assert!(!Happening::EnteredArea {
            name: "Felled Hideout".into()
        }
        .is_worth_showing());

        assert!(Happening::EnteredArea {
            name: "Clearfell".into()
        }
        .is_worth_showing());
    }

    #[test]
    fn a_private_league_is_told_apart_by_its_bracketed_owner() {
        assert!(is_private_league("Standard (PL12345)"));
        assert!(!is_private_league("Standard"));
        assert!(!is_private_league("Hardcore Ruthless"));
    }

    #[test]
    fn a_hideout_is_recognised_however_it_is_named() {
        assert!(is_hideout("Coastal Hideout"));
        assert!(is_hideout("felled hideout"));
        assert!(is_hideout("The Rogue Harbour"));
        assert!(!is_hideout("Clearfell"));
    }

    #[test]
    fn levelling_up_records_the_character_and_the_level() {
        let mut bg = Background::default();

        bg.apply(&Happening::LevelledUp {
            character: "Zelina".into(),
            level: 34,
        });

        assert_eq!(bg.who_you_are(), "Zelina, level 34");
    }

    #[test]
    fn whispers_are_counted() {
        let mut bg = Background::default();

        bg.apply(&Happening::Whisper {
            from: "Ghodrati".into(),
        });
        bg.apply(&Happening::Whisper {
            from: "Someone".into(),
        });

        assert_eq!(bg.whispers, 2);
    }

    #[test]
    fn nothing_seen_yet_says_so_rather_than_showing_a_blank() {
        let bg = Background::default();

        assert_eq!(bg.where_you_are(), "not in game");
        assert_eq!(bg.who_you_are(), "no character seen");
    }

    #[test]
    fn every_happening_reads_as_a_sentence() {
        for happening in [
            Happening::EnteredArea {
                name: "Clearfell".into(),
            },
            Happening::LevelledUp {
                character: "Zelina".into(),
                level: 3,
            },
            Happening::LeagueChanged {
                league: "Standard".into(),
            },
            Happening::Whisper {
                from: "Ghodrati".into(),
            },
        ] {
            assert!(!happening.line().trim().is_empty());
        }
    }
}
