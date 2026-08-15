use serde::{Deserialize, Serialize};

use crate::types::GameVersion;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePair<T> {
    poe1: T,
    poe2: T,
}

impl<T> GamePair<T> {
    pub fn new(poe1: T, poe2: T) -> Self {
        Self { poe1, poe2 }
    }

    pub fn get(&self, game: GameVersion) -> &T {
        match game {
            GameVersion::Poe1 => &self.poe1,
            GameVersion::Poe2 => &self.poe2,
        }
    }

    pub fn get_mut(&mut self, game: GameVersion) -> &mut T {
        match game {
            GameVersion::Poe1 => &mut self.poe1,
            GameVersion::Poe2 => &mut self.poe2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_game_reads_back_its_own_half() {
        let pair = GamePair::new("one", "two");

        assert_eq!(*pair.get(GameVersion::Poe1), "one");
        assert_eq!(*pair.get(GameVersion::Poe2), "two");
    }

    #[test]
    fn a_pair_survives_a_json_round_trip_so_it_can_be_stored_per_game() {
        let pair = GamePair::new("Standard".to_string(), "Rise of the Abyssal".to_string());

        let text = serde_json::to_string(&pair).unwrap();

        assert_eq!(
            serde_json::from_str::<GamePair<String>>(&text).unwrap(),
            pair
        );
        assert!(text.contains("poe1"), "{text}");
        assert!(text.contains("poe2"), "{text}");
    }

    #[test]
    fn writing_one_half_leaves_the_other_alone() {
        let mut pair = GamePair::new(1, 2);

        *pair.get_mut(GameVersion::Poe1) = 9;

        assert_eq!(*pair.get(GameVersion::Poe1), 9);
        assert_eq!(*pair.get(GameVersion::Poe2), 2);
    }
}
