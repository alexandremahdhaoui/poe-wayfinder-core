//! The game data the parser needs.

use crate::types::GameVersion;

/// A stat and the trade ids it maps to.
///
/// Shape comes from `data/stats.ndjson`. See STUDY section 7.
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    /// The canonical template with `#` for each number.
    pub reference: String,
    /// Every literal string the game can print for this stat, including
    /// singular forms that bake a value into the text.
    pub matchers: Vec<StatMatcher>,
    /// Trade ids keyed by modifier namespace.
    pub trade_ids: Vec<(String, Vec<String>)>,
}

/// One literal form of a stat line.
#[derive(Debug, Clone, PartialEq)]
pub struct StatMatcher {
    pub string: String,
    /// Present when this form bakes a value into the text.
    pub value: Option<f64>,
    /// The game prints a reduction as a positive number for these.
    pub negate: bool,
}

/// An item base or area or currency.
///
/// Shape comes from `data/items.ndjson`.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemInfo {
    pub name: String,
    pub reference_name: String,
    pub namespace: String,
    pub trade_discriminator: Option<String>,
}

/// Game data lookups the parser depends on.
///
/// Declared here because this crate consumes it and implements nothing. The
/// implementation lives in `poe-trader-app` and reads the ndjson.
pub trait GameData: Send + Sync {
    /// Find an item base by its displayed name.
    fn item_by_name(&self, name: &str, game: GameVersion) -> Option<&ItemInfo>;

    /// Find a stat whose matcher equals this normalised text.
    fn stat_by_matcher(&self, text: &str, game: GameVersion) -> Option<&Stat>;
}
