use crate::controller::parse::{ParseError, ParserState};
use crate::types::item::BaseInfo;
use crate::types::modifier::ModifierType;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discriminator {
    pub prop_ar: bool,
    pub prop_ev: bool,
    pub prop_es: bool,
    pub has_implicit: Option<String>,
    pub has_explicit: Option<String>,
    pub section_text: Option<String>,
    pub map_tier: Option<MapTierBand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapTierBand {
    White,
    Yellow,
    Red,
}

impl MapTierBand {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "W" => Some(MapTierBand::White),
            "Y" => Some(MapTierBand::Yellow),
            "R" => Some(MapTierBand::Red),
            _ => None,
        }
    }

    pub fn contains(self, tier: u32) -> bool {
        match self {
            MapTierBand::White => tier <= 5,
            MapTierBand::Yellow => (6..=10).contains(&tier),
            MapTierBand::Red => tier >= 11,
        }
    }
}

impl Discriminator {
    pub fn matches(&self, state: &ParserState<'_>) -> bool {
        let item = &state.item;

        if self.prop_ar && item.armour.ar.is_none() {
            return false;
        }

        if self.prop_ev && item.armour.ev.is_none() {
            return false;
        }

        if self.prop_es && item.armour.es.is_none() {
            return false;
        }

        if let Some(band) = self.map_tier {
            let Some(tier) = item.map.tier else {
                return false;
            };

            if !band.contains(tier) {
                return false;
            }
        }

        if let Some(reference) = &self.has_implicit {
            if !has_stat(state, reference, ModifierType::Implicit) {
                return false;
            }
        }

        if let Some(reference) = &self.has_explicit {
            if !has_stat(state, reference, ModifierType::Explicit) {
                return false;
            }
        }

        if let Some(text) = &self.section_text {
            if !item.raw_text.contains(text.as_str()) {
                return false;
            }
        }

        true
    }
}

fn has_stat(state: &ParserState<'_>, reference: &str, kind: ModifierType) -> bool {
    state
        .item
        .modifiers
        .iter()
        .any(|m| m.info.kind == Some(kind) && m.stats.iter().any(|s| s.reference == reference))
}

pub fn choose_variant<'a>(
    state: &ParserState<'_>,
    candidates: &'a [(BaseInfo, Discriminator)],
) -> Option<&'a BaseInfo> {
    let mut chosen = None;

    for (info, condition) in candidates {
        if condition.matches(state) {
            chosen = Some(info);
        }
    }

    chosen
}

pub fn pick_correct_variant(state: &mut ParserState<'_>) -> Result<(), ParseError> {
    let candidates = state.variants.clone();

    if candidates.is_empty() {
        return Ok(());
    }

    if let Some(info) = choose_variant(state, &candidates) {
        state.item.info = info.clone();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::parse::shared::modifiers::ParsedModifier;
    use crate::types::item::{ArmourStats, MapStats};
    use crate::types::modifier::ModifierInfo;
    use crate::types::stat::ParsedStat;

    fn base(disc: &str) -> BaseInfo {
        BaseInfo {
            name: "Two-Stone Ring".into(),
            reference_name: "Two-Stone Ring".into(),
            trade_discriminator: Some(disc.into()),
            ..BaseInfo::default()
        }
    }

    fn state() -> ParserState<'static> {
        ParserState::default()
    }

    fn with_stat(kind: ModifierType, reference: &str) -> ParsedModifier {
        ParsedModifier {
            info: ModifierInfo {
                kind: Some(kind),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: reference.into(),
                matched: reference.into(),
                roll: None,
            }],
        }
    }

    #[test]
    fn an_empty_condition_matches_anything() {
        assert!(Discriminator::default().matches(&state()));
    }

    #[test]
    fn a_property_condition_needs_that_property() {
        let condition = Discriminator {
            prop_ar: true,
            ..Discriminator::default()
        };

        assert!(!condition.matches(&state()));

        let mut s = state();
        s.item.armour = ArmourStats {
            ar: Some(100.0),
            ..ArmourStats::default()
        };

        assert!(condition.matches(&s));
    }

    #[test]
    fn each_defensive_property_is_checked_separately() {
        let mut s = state();
        s.item.armour = ArmourStats {
            ar: Some(100.0),
            ..ArmourStats::default()
        };

        let needs_evasion = Discriminator {
            prop_ev: true,
            ..Discriminator::default()
        };

        assert!(!needs_evasion.matches(&s));
    }

    #[test]
    fn an_implicit_condition_needs_that_implicit() {
        let condition = Discriminator {
            has_implicit: Some("#% to Fire and Cold Resistances".into()),
            ..Discriminator::default()
        };

        assert!(!condition.matches(&state()));

        let mut s = state();
        s.item.modifiers.push(with_stat(
            ModifierType::Implicit,
            "#% to Fire and Cold Resistances",
        ));

        assert!(condition.matches(&s));
    }

    #[test]
    fn the_namespace_of_the_condition_is_checked() {
        let condition = Discriminator {
            has_implicit: Some("#% to Fire and Cold Resistances".into()),
            ..Discriminator::default()
        };

        let mut s = state();
        s.item.modifiers.push(with_stat(
            ModifierType::Explicit,
            "#% to Fire and Cold Resistances",
        ));

        assert!(!condition.matches(&s));
    }

    #[test]
    fn a_section_text_condition_searches_the_raw_item_text() {
        let condition = Discriminator {
            section_text: Some("Vaal".into()),
            ..Discriminator::default()
        };

        assert!(!condition.matches(&state()));

        let mut s = state();
        s.item.raw_text = "Item Class: Rings\nVaal Ring".into();

        assert!(condition.matches(&s));
    }

    #[test]
    fn each_map_tier_band_covers_its_own_range() {
        assert!(MapTierBand::White.contains(1));
        assert!(MapTierBand::White.contains(5));
        assert!(!MapTierBand::White.contains(6));

        assert!(MapTierBand::Yellow.contains(6));
        assert!(MapTierBand::Yellow.contains(10));
        assert!(!MapTierBand::Yellow.contains(11));

        assert!(MapTierBand::Red.contains(11));
        assert!(MapTierBand::Red.contains(16));
        assert!(!MapTierBand::Red.contains(10));
    }

    #[test]
    fn the_bands_cover_every_tier_exactly_once() {
        for tier in 0..=20 {
            let hits = [MapTierBand::White, MapTierBand::Yellow, MapTierBand::Red]
                .iter()
                .filter(|b| b.contains(tier))
                .count();

            assert_eq!(hits, 1, "tier {tier}");
        }
    }

    #[test]
    fn the_band_letters_round_trip() {
        assert_eq!(MapTierBand::parse("W"), Some(MapTierBand::White));
        assert_eq!(MapTierBand::parse("Y"), Some(MapTierBand::Yellow));
        assert_eq!(MapTierBand::parse("R"), Some(MapTierBand::Red));
        assert_eq!(MapTierBand::parse("X"), None);
    }

    #[test]
    fn a_tier_condition_on_an_item_with_no_tier_fails() {
        let condition = Discriminator {
            map_tier: Some(MapTierBand::Red),
            ..Discriminator::default()
        };

        assert!(!condition.matches(&state()));
    }

    #[test]
    fn a_tier_in_the_band_matches() {
        let condition = Discriminator {
            map_tier: Some(MapTierBand::Red),
            ..Discriminator::default()
        };

        let mut s = state();
        s.item.map = MapStats {
            tier: Some(16),
            ..MapStats::default()
        };

        assert!(condition.matches(&s));
    }

    #[test]
    fn every_condition_in_a_discriminator_must_hold() {
        let condition = Discriminator {
            prop_ar: true,
            has_implicit: Some("x".into()),
            ..Discriminator::default()
        };

        let mut s = state();
        s.item.armour = ArmourStats {
            ar: Some(100.0),
            ..ArmourStats::default()
        };

        assert!(!condition.matches(&s));
    }

    #[test]
    fn the_variant_whose_condition_holds_is_chosen() {
        let candidates = vec![
            (
                base("fire_cold"),
                Discriminator {
                    has_implicit: Some("#% to Fire and Cold Resistances".into()),
                    ..Discriminator::default()
                },
            ),
            (
                base("fire_lightning"),
                Discriminator {
                    has_implicit: Some("#% to Fire and Lightning Resistances".into()),
                    ..Discriminator::default()
                },
            ),
        ];

        let mut s = state();
        s.item.modifiers.push(with_stat(
            ModifierType::Implicit,
            "#% to Fire and Lightning Resistances",
        ));

        let chosen = choose_variant(&s, &candidates).unwrap();

        assert_eq!(
            chosen.trade_discriminator.as_deref(),
            Some("fire_lightning")
        );
    }

    #[test]
    fn nothing_is_chosen_when_no_condition_holds() {
        let candidates = vec![(
            base("fire_cold"),
            Discriminator {
                has_implicit: Some("#% to Fire and Cold Resistances".into()),
                ..Discriminator::default()
            },
        )];

        assert!(choose_variant(&state(), &candidates).is_none());
    }

    #[test]
    fn the_last_matching_variant_wins() {
        let candidates = vec![
            (base("first"), Discriminator::default()),
            (base("second"), Discriminator::default()),
        ];

        let chosen = choose_variant(&state(), &candidates).unwrap();

        assert_eq!(chosen.trade_discriminator.as_deref(), Some("second"));
    }

    #[test]
    fn no_candidates_leaves_the_item_alone() {
        let mut s = state();
        s.item.info = base("original");

        pick_correct_variant(&mut s).unwrap();

        assert_eq!(s.item.info.trade_discriminator.as_deref(), Some("original"));
    }

    #[test]
    fn the_chosen_variant_lands_on_the_item() {
        let mut s = state();
        s.item.info = base("original");
        s.variants = vec![(base("chosen"), Discriminator::default())];

        pick_correct_variant(&mut s).unwrap();

        assert_eq!(s.item.info.trade_discriminator.as_deref(), Some("chosen"));
    }

    #[test]
    fn a_variant_list_where_nothing_matches_leaves_the_item_alone() {
        let mut s = state();
        s.item.info = base("original");
        s.variants = vec![(
            base("chosen"),
            Discriminator {
                prop_ar: true,
                ..Discriminator::default()
            },
        )];

        pick_correct_variant(&mut s).unwrap();

        assert_eq!(s.item.info.trade_discriminator.as_deref(), Some("original"));
    }
}
