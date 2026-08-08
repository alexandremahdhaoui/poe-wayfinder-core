#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameVersion {
    Poe1,
    Poe2,
}

impl GameVersion {
    pub fn trade_path(self) -> &'static str {
        match self {
            GameVersion::Poe1 => "trade",
            GameVersion::Poe2 => "trade2",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            GameVersion::Poe1 => "poe1",
            GameVersion::Poe2 => "poe2",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "poe1" => Some(GameVersion::Poe1),
            "poe2" => Some(GameVersion::Poe2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trade_path_differs_per_game() {
        assert_eq!(GameVersion::Poe1.trade_path(), "trade");
        assert_eq!(GameVersion::Poe2.trade_path(), "trade2");
    }

    #[test]
    fn parse_round_trips_as_str() {
        for game in [GameVersion::Poe1, GameVersion::Poe2] {
            assert_eq!(GameVersion::parse(game.as_str()), Some(game));
        }
    }

    #[test]
    fn parse_rejects_anything_else() {
        for s in ["poe", "POE2", "poe3", ""] {
            assert_eq!(GameVersion::parse(s), None, "{s} was accepted");
        }
    }
}
