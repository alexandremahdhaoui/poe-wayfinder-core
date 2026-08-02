//! Which game an item came from.

/// The two supported games.
///
/// 35 of 50 parser stages are shared between them. The rest are game specific
/// and the pipeline is assembled per game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameVersion {
    /// Path of Exile 1. Trade API lives under `/api/trade/`.
    Poe1,
    /// Path of Exile 2. Trade API lives under `/api/trade2/`.
    Poe2,
}

impl GameVersion {
    /// The trade API path segment for this game.
    ///
    /// This is the only difference between the two trade APIs.
    pub fn trade_path(self) -> &'static str {
        match self {
            GameVersion::Poe1 => "trade",
            GameVersion::Poe2 => "trade2",
        }
    }

    /// The config and log spelling of this game.
    pub fn as_str(self) -> &'static str {
        match self {
            GameVersion::Poe1 => "poe1",
            GameVersion::Poe2 => "poe2",
        }
    }

    /// Parse the config spelling.
    ///
    /// Returns None for anything else so a typo in `--game` fails at startup
    /// rather than silently parsing PoE2 rules against a PoE1 item.
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
