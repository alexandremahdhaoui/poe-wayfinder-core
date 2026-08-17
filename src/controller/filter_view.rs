use crate::controller::price_check::PriceCheck;
use crate::types::query::{Range, TradeQuery};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericKey {
    ItemLevel,
    Quality,
    GemLevel,
    AreaLevel,
    MapTier,
    RequiresLevel,
    Armour,
    Evasion,
    EnergyShield,
    Block,
    CritChance,
    AttackSpeed,
    Dps,
    PhysicalDps,
    ElementalDps,
    RuneSockets,
    Spirit,
    ReloadTime,
    GemSockets,
}

impl NumericKey {
    pub fn label(self) -> &'static str {
        match self {
            NumericKey::ItemLevel => "Item Level",
            NumericKey::Quality => "Quality",
            NumericKey::GemLevel => "Gem Level",
            NumericKey::AreaLevel => "Area Level",
            NumericKey::MapTier => "Map Tier",
            NumericKey::RequiresLevel => "Requires Level",
            NumericKey::Armour => "Armour",
            NumericKey::Evasion => "Evasion",
            NumericKey::EnergyShield => "Energy Shield",
            NumericKey::Block => "Block",
            NumericKey::CritChance => "Critical Chance",
            NumericKey::AttackSpeed => "Attacks per Second",
            NumericKey::Dps => "DPS",
            NumericKey::PhysicalDps => "Physical DPS",
            NumericKey::ElementalDps => "Elemental DPS",
            NumericKey::RuneSockets => "Rune Sockets",
            NumericKey::Spirit => "Spirit",
            NumericKey::ReloadTime => "Reload Time",
            NumericKey::GemSockets => "Sockets",
        }
    }

    pub fn decimals(self) -> bool {
        matches!(
            self,
            NumericKey::CritChance
                | NumericKey::AttackSpeed
                | NumericKey::Dps
                | NumericKey::PhysicalDps
                | NumericKey::ElementalDps
                | NumericKey::ReloadTime
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlagKey {
    Corrupted,
    Mirrored,
    Sanctified,
    Veiled,
    Identified,
    AlternateArt,
    Fractured,
}

impl FlagKey {
    pub fn label(self) -> &'static str {
        match self {
            FlagKey::Corrupted => "Corrupted",
            FlagKey::Mirrored => "Mirrored",
            FlagKey::Sanctified => "Sanctified",
            FlagKey::Veiled => "Veiled",
            FlagKey::Identified => "Identified",
            FlagKey::AlternateArt => "Alternate Art",
            FlagKey::Fractured => "Fractured",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowKey {
    Numeric(NumericKey),
    Flag(FlagKey),
    Stat { group: usize, index: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub key: RowKey,
    pub label: String,
    pub enabled: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub roll: Option<f64>,
    pub bounds: Option<(f64, f64)>,
    pub decimals: bool,
    pub tier: Option<u32>,
    pub contributors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameMode {
    Exact,
    #[default]
    BaseType,
    Any,
}

impl NameMode {
    pub fn next(self) -> Self {
        match self {
            NameMode::Exact => NameMode::BaseType,
            NameMode::BaseType => NameMode::Any,
            NameMode::Any => NameMode::Exact,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NameMode::Exact => "this exact item",
            NameMode::BaseType => "this base type",
            NameMode::Any => "any base type",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NameRow {
    pub mode: NameMode,
    pub exact: Option<String>,
    pub base_type: Option<String>,
}

impl NameRow {
    pub fn caption(&self) -> String {
        match self.mode {
            NameMode::Exact => self
                .exact
                .clone()
                .or_else(|| self.base_type.clone())
                .unwrap_or_else(|| "any item".to_string()),
            NameMode::BaseType => self
                .base_type
                .clone()
                .unwrap_or_else(|| "any base type".to_string()),
            NameMode::Any => "any base type".to_string(),
        }
    }
}

pub fn tier_label(tier: Option<u32>) -> Option<String> {
    tier.map(|t| format!("T{t}"))
}

pub fn modifier_text(text: &str, roll: Option<f64>, decimals: bool) -> String {
    let Some(roll) = roll else {
        return text.to_string();
    };

    if !text.contains('#') {
        return text.to_string();
    }

    if !roll.is_finite() {
        return text.to_string();
    }

    if decimals {
        return text.replacen('#', &format!("{roll:.2}"), 1);
    }

    crate::controller::parse::magic_name::fill_placeholders(text, &[roll.round()])
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeEdit {
    pub sets_min: bool,
    pub value: f64,
}

pub fn gauge_edit(row: &Row, ratio: f64) -> Option<GaugeEdit> {
    let (low, high) = row.bounds?;

    if high <= low {
        return None;
    }

    let raw = low + ratio.clamp(0.0, 1.0) * (high - low);

    let value = match row.decimals {
        true => (raw * 100.0).round() / 100.0,
        false => raw.round(),
    };

    let min = row.min.unwrap_or(low);
    let max = row.max.unwrap_or(high);

    let sets_min = if value < min {
        true
    } else if value > max {
        false
    } else {
        (value - min).abs() <= (value - max).abs()
    };

    Some(GaugeEdit {
        sets_min,
        value: match sets_min {
            true => value.min(max),
            false => value.max(min),
        },
    })
}

impl Row {
    pub fn percent_of_bounds(&self) -> Option<f64> {
        let (low, high) = self.bounds?;
        let roll = self.roll?;

        match high > low {
            true => Some(((roll - low) / (high - low)).clamp(0.0, 1.0)),
            false => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagRow {
    pub key: FlagKey,
    pub label: String,
    pub enabled: bool,
    pub value: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterView {
    pub numerics: Vec<Row>,
    pub flags: Vec<FlagRow>,
    pub stats: Vec<Row>,
    pub name: NameRow,
}

impl FilterView {
    pub fn is_empty(&self) -> bool {
        self.numerics.is_empty() && self.flags.is_empty() && self.stats.is_empty()
    }

    pub fn row_mut(&mut self, key: RowKey) -> Option<&mut Row> {
        self.numerics
            .iter_mut()
            .chain(self.stats.iter_mut())
            .find(|row| row.key == key)
    }

    pub fn flag_mut(&mut self, key: FlagKey) -> Option<&mut FlagRow> {
        self.flags.iter_mut().find(|row| row.key == key)
    }

    pub fn set_all_stats(&mut self, enabled: bool) {
        for row in &mut self.stats {
            row.enabled = enabled;
        }
    }

    pub fn all_stats_on(&self) -> bool {
        !self.stats.is_empty() && self.stats.iter().all(|row| row.enabled)
    }

    pub fn cycle_name(&mut self) {
        self.name.mode = self.name.mode.next();
    }

    pub fn enabled_count(&self) -> usize {
        self.numerics.iter().filter(|r| r.enabled).count()
            + self.stats.iter().filter(|r| r.enabled).count()
            + self.flags.iter().filter(|r| r.enabled).count()
    }
}

fn numeric_row(key: NumericKey, range: Range, roll: Option<f64>) -> Row {
    Row {
        key: RowKey::Numeric(key),
        label: key.label().to_string(),
        enabled: !range.is_empty(),
        min: range.min.or(roll),
        max: range.max,
        roll,
        bounds: None,
        decimals: key.decimals(),
        tier: None,
        contributors: Vec::new(),
    }
}

fn flag_row(key: FlagKey, flag: Option<bool>) -> FlagRow {
    FlagRow {
        key,
        label: key.label().to_string(),
        enabled: flag.is_some(),
        value: flag.unwrap_or(true),
    }
}

pub fn build(check: &PriceCheck) -> FilterView {
    FilterView {
        numerics: numerics(check),
        flags: flags(check),
        stats: stats(check),
        name: name_rows(check),
    }
}

pub fn name_rows(check: &PriceCheck) -> NameRow {
    let exact = check.query.name.as_ref().map(|n| n.name().to_string());

    let base_type = check
        .query
        .type_name
        .as_ref()
        .map(|n| n.name().to_string())
        .or_else(|| non_empty(&check.item.info.reference_name));

    NameRow {
        mode: match (&exact, &base_type) {
            (Some(_), _) => NameMode::Exact,
            (None, Some(_)) => NameMode::BaseType,
            (None, None) => NameMode::Any,
        },
        exact: exact.or_else(|| non_empty(&check.item.info.name)),
        base_type,
    }
}

fn non_empty(text: &str) -> Option<String> {
    match text.is_empty() {
        true => None,
        false => Some(text.to_string()),
    }
}

fn numerics(check: &PriceCheck) -> Vec<Row> {
    let item = &check.item;
    let filters = &check.query.filters;
    let mut out = Vec::new();

    if item.item_level.is_some() {
        let mut row = numeric_row(
            NumericKey::ItemLevel,
            filters.type_filters.ilvl,
            item.item_level.map(f64::from),
        );

        row.enabled = row.enabled || item.is_unidentified;

        out.push(row);
    }

    if item.quality.unwrap_or(0) != 0 {
        out.push(numeric_row(
            NumericKey::Quality,
            filters.type_filters.quality,
            item.quality.map(f64::from),
        ));
    }

    if item.gem_level.is_some() {
        out.push(numeric_row(
            NumericKey::GemLevel,
            filters.misc_filters.gem_level,
            item.gem_level.map(f64::from),
        ));
    }

    if item.area_level.is_some() {
        out.push(numeric_row(
            NumericKey::AreaLevel,
            filters.misc_filters.area_level,
            item.area_level.map(f64::from),
        ));
    }

    if !filters.map_filters.map_tier.is_empty() {
        out.push(numeric_row(
            NumericKey::MapTier,
            filters.map_filters.map_tier,
            None,
        ));
    }

    if let Some(requires) = &item.requires {
        if requires.level > 0 {
            out.push(numeric_row(
                NumericKey::RequiresLevel,
                filters.req_filters.lvl,
                Some(f64::from(requires.level)),
            ));
        }
    }

    for (key, range, roll) in equipment_rows(check) {
        if range.is_empty() && roll.is_none() {
            continue;
        }

        if !fits_the_item(key, item.category) {
            continue;
        }

        let mut row = numeric_row(key, range, roll);

        row.bounds = property_bounds(check, key, roll);

        out.push(row);
    }

    if let Some(sockets) = &item.gem_sockets {
        if sockets.number > 0 && item.category.map(|c| c.grants_skill()).unwrap_or(true) {
            out.push(numeric_row(
                NumericKey::GemSockets,
                filters.misc_filters.gem_sockets,
                Some(f64::from(sockets.number)),
            ));
        }
    }

    out
}

fn fits_the_item(key: NumericKey, category: Option<crate::types::category::ItemCategory>) -> bool {
    let Some(category) = category else {
        return true;
    };

    let weapon_only = matches!(
        key,
        NumericKey::CritChance
            | NumericKey::AttackSpeed
            | NumericKey::Dps
            | NumericKey::PhysicalDps
            | NumericKey::ElementalDps
            | NumericKey::ReloadTime
    );

    if weapon_only {
        return category.is_weapon();
    }

    let gear_only = matches!(
        key,
        NumericKey::Armour
            | NumericKey::Evasion
            | NumericKey::EnergyShield
            | NumericKey::Block
            | NumericKey::RuneSockets
    );

    match gear_only {
        true => !category.is_accessory(),
        false => true,
    }
}

fn property_bounds(check: &PriceCheck, key: NumericKey, roll: Option<f64>) -> Option<(f64, f64)> {
    use crate::controller::aggregate::sum_stats_by_type;
    use crate::controller::calc::base::{
        contributions, ARMOUR, ATTACK_SPEED, BLOCK, ENERGY_SHIELD, EVASION,
    };
    use crate::controller::calc::quality::prop_bounds;

    let stats = match key {
        NumericKey::Armour => ARMOUR,
        NumericKey::Evasion => EVASION,
        NumericKey::EnergyShield => ENERGY_SHIELD,
        NumericKey::Block => BLOCK,
        NumericKey::AttackSpeed => ATTACK_SPEED,
        _ => return None,
    };

    let printed = roll?;
    let totals = sum_stats_by_type(&check.item.modifiers);
    let bounds = prop_bounds(printed, contributions(stats, &totals));

    if (bounds.max - bounds.min).abs() < f64::EPSILON {
        return None;
    }

    Some((bounds.min, bounds.max))
}

fn equipment_rows(check: &PriceCheck) -> Vec<(NumericKey, Range, Option<f64>)> {
    let e = &check.query.filters.equipment_filters;
    let weapon = check.item.weapon;

    let mut armour = check.item.armour;

    {
        use crate::controller::aggregate::sum_stats_by_type;
        use crate::controller::filter::item_property::{at_trade_quality, Scaling};

        let totals = sum_stats_by_type(&check.item.modifiers);
        let scaling = Scaling {
            quality: f64::from(check.item.quality.unwrap_or(0)),
            modifiable: check.item.is_modifiable(),
            totals: &totals,
        };

        for (field, stats) in [
            (&mut armour.ar, crate::controller::calc::base::ARMOUR),
            (&mut armour.ev, crate::controller::calc::base::EVASION),
            (&mut armour.es, crate::controller::calc::base::ENERGY_SHIELD),
        ] {
            if let Some(printed) = field.filter(|value| *value > 0.0) {
                *field = Some(at_trade_quality(printed, stats, scaling));
            }
        }
    }

    let source = |id: &str| {
        check
            .sources
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.roll)
            .map(|roll| roll.value)
    };

    vec![
        (
            NumericKey::Armour,
            e.ar,
            armour.ar.or_else(|| source("item.armour")),
        ),
        (
            NumericKey::Evasion,
            e.ev,
            armour.ev.or_else(|| source("item.evasion_rating")),
        ),
        (
            NumericKey::EnergyShield,
            e.es,
            armour.es.or_else(|| source("item.energy_shield")),
        ),
        (NumericKey::Block, e.block, armour.block),
        (NumericKey::CritChance, e.crit, weapon.crit),
        (NumericKey::AttackSpeed, e.aps, weapon.attack_speed),
        (NumericKey::Dps, e.dps, source("item.total_dps")),
        (NumericKey::PhysicalDps, e.pdps, source("item.physical_dps")),
        (
            NumericKey::ElementalDps,
            e.edps,
            source("item.elemental_dps"),
        ),
        (
            NumericKey::RuneSockets,
            e.rune_sockets,
            source("item.rune_sockets"),
        ),
        (NumericKey::Spirit, e.spirit, source("item.spirit")),
        (
            NumericKey::ReloadTime,
            e.reload_time,
            source("item.reload_time"),
        ),
    ]
}

fn flags(check: &PriceCheck) -> Vec<FlagRow> {
    let misc = &check.query.filters.misc_filters;
    let item = &check.item;
    let mut out = Vec::new();

    if item.is_corrupted || misc.corrupted.is_some() {
        out.push(flag_row(FlagKey::Corrupted, misc.corrupted));
    }

    if item.is_mirrored || misc.mirrored.is_some() {
        out.push(flag_row(FlagKey::Mirrored, misc.mirrored));
    }

    if item.is_sanctified || misc.sanctified.is_some() {
        out.push(flag_row(FlagKey::Sanctified, misc.sanctified));
    }

    if item.is_veiled || misc.veiled.is_some() {
        out.push(flag_row(FlagKey::Veiled, misc.veiled));
    }

    if item.is_unidentified || misc.identified.is_some() {
        out.push(flag_row(FlagKey::Identified, misc.identified));
    }

    if item.is_foil || misc.alternate_art.is_some() {
        out.push(flag_row(FlagKey::AlternateArt, misc.alternate_art));
    }

    if item.is_fractured || misc.fractured_item.is_some() {
        out.push(flag_row(FlagKey::Fractured, misc.fractured_item));
    }

    out
}

pub fn influence_labels(item: &crate::types::item::ParsedItem) -> Vec<&'static str> {
    use crate::types::item::Influence;

    [
        (Influence::Shaper, "Shaper"),
        (Influence::Elder, "Elder"),
        (Influence::Crusader, "Crusader"),
        (Influence::Hunter, "Hunter"),
        (Influence::Redeemer, "Redeemer"),
        (Influence::Warlord, "Warlord"),
    ]
    .into_iter()
    .filter(|(influence, _)| item.has_influence(*influence))
    .map(|(_, label)| label)
    .collect()
}

fn stats(check: &PriceCheck) -> Vec<Row> {
    let mut out = Vec::new();

    for (group, stat_group) in check.query.stats.iter().enumerate() {
        for (index, filter) in stat_group.filters.iter().enumerate() {
            if filter.hidden {
                continue;
            }

            let source = check.sources.iter().find(|s| s.id == filter.id);
            let roll = source.and_then(|s| s.roll);

            let exact = source
                .map(|s| crate::controller::filter::slots::is_cluster_socket_stat(&s.text))
                .unwrap_or(false);

            out.push(Row {
                key: RowKey::Stat { group, index },
                label: source
                    .map(|s| s.text.clone())
                    .unwrap_or_else(|| filter.id.clone()),
                enabled: !filter.disabled,
                min: filter.range.min,
                max: match exact {
                    true => filter.range.max.or(filter.range.min),
                    false => filter.range.max,
                },
                roll: roll.map(|r| r.value),
                bounds: roll.filter(|r| r.max > r.min).map(|r| (r.min, r.max)),
                decimals: roll.map(|r| r.decimals).unwrap_or(false),
                tier: source.and_then(|s| s.tier),
                contributors: source.map(|s| s.contributors.clone()).unwrap_or_default(),
            });
        }
    }

    out
}

pub fn apply(view: &FilterView, query: &mut TradeQuery) {
    apply_name(view, query);

    for row in &view.numerics {
        let RowKey::Numeric(key) = row.key else {
            continue;
        };

        *numeric_target(key, query) = range_of(row);
    }

    for row in &view.flags {
        *flag_target(row.key, query) = row.enabled.then_some(row.value);
    }

    for row in &view.stats {
        let RowKey::Stat { group, index } = row.key else {
            continue;
        };

        let Some(filter) = query
            .stats
            .get_mut(group)
            .and_then(|g| g.filters.get_mut(index))
        else {
            continue;
        };

        filter.disabled = !row.enabled;
        filter.range = range_of(row);
    }
}

fn apply_name(view: &FilterView, query: &mut TradeQuery) {
    use crate::types::query::NameField;

    match view.name.mode {
        NameMode::Exact => {
            query.name = view.name.exact.as_deref().map(|n| NameField::new(n, None));
            query.type_name = view
                .name
                .base_type
                .as_deref()
                .map(|n| NameField::new(n, None));
        }
        NameMode::BaseType => {
            query.name = None;
            query.type_name = view
                .name
                .base_type
                .as_deref()
                .map(|n| NameField::new(n, None));
        }
        NameMode::Any => {
            query.name = None;
            query.type_name = None;
        }
    }
}

fn range_of(row: &Row) -> Range {
    match row.enabled {
        true => Range {
            min: row.min,
            max: row.max,
        },
        false => Range::default(),
    }
}

fn numeric_target(key: NumericKey, query: &mut TradeQuery) -> &mut Range {
    let filters = &mut query.filters;

    match key {
        NumericKey::ItemLevel => &mut filters.type_filters.ilvl,
        NumericKey::Quality => &mut filters.type_filters.quality,
        NumericKey::GemLevel => &mut filters.misc_filters.gem_level,
        NumericKey::AreaLevel => &mut filters.misc_filters.area_level,
        NumericKey::MapTier => &mut filters.map_filters.map_tier,
        NumericKey::RequiresLevel => &mut filters.req_filters.lvl,
        NumericKey::Armour => &mut filters.equipment_filters.ar,
        NumericKey::Evasion => &mut filters.equipment_filters.ev,
        NumericKey::EnergyShield => &mut filters.equipment_filters.es,
        NumericKey::Block => &mut filters.equipment_filters.block,
        NumericKey::CritChance => &mut filters.equipment_filters.crit,
        NumericKey::AttackSpeed => &mut filters.equipment_filters.aps,
        NumericKey::Dps => &mut filters.equipment_filters.dps,
        NumericKey::PhysicalDps => &mut filters.equipment_filters.pdps,
        NumericKey::ElementalDps => &mut filters.equipment_filters.edps,
        NumericKey::RuneSockets => &mut filters.equipment_filters.rune_sockets,
        NumericKey::Spirit => &mut filters.equipment_filters.spirit,
        NumericKey::ReloadTime => &mut filters.equipment_filters.reload_time,
        NumericKey::GemSockets => &mut filters.misc_filters.gem_sockets,
    }
}

fn flag_target(key: FlagKey, query: &mut TradeQuery) -> &mut Option<bool> {
    let misc = &mut query.filters.misc_filters;

    match key {
        FlagKey::Corrupted => &mut misc.corrupted,
        FlagKey::Mirrored => &mut misc.mirrored,
        FlagKey::Sanctified => &mut misc.sanctified,
        FlagKey::Veiled => &mut misc.veiled,
        FlagKey::Identified => &mut misc.identified,
        FlagKey::AlternateArt => &mut misc.alternate_art,
        FlagKey::Fractured => &mut misc.fractured_item,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::controller::bulk::Endpoint;
    use crate::controller::filter::stat_filters::FilterSource;
    use crate::types::item::ParsedItem;
    use crate::types::query::{StatFilter, StatGroup};
    use crate::types::stat::StatRoll;

    fn check(item: ParsedItem, query: TradeQuery, sources: Vec<FilterSource>) -> PriceCheck {
        PriceCheck {
            item,
            query,
            endpoint: Endpoint::Search,
            trade_tag: None,
            sources,
        }
    }

    fn roll(value: f64, min: f64, max: f64) -> StatRoll {
        StatRoll {
            value,
            min,
            max,
            ..StatRoll::default()
        }
    }

    fn ring() -> ParsedItem {
        ParsedItem {
            item_level: Some(78),
            quality: Some(20),
            ..ParsedItem::default()
        }
    }

    fn one_stat() -> PriceCheck {
        let mut query = TradeQuery::default();
        query.stats.push(StatGroup::all(vec![StatFilter::range(
            "explicit.stat_life",
            Range::at_least(70.0),
        )]));

        check(
            ring(),
            query,
            vec![FilterSource {
                id: "explicit.stat_life".into(),
                text: "+80 to maximum Life".into(),
                roll: Some(roll(80.0, 60.0, 100.0)),
                tier: Some(3),
                contributors: Vec::new(),
            }],
        )
    }

    #[test]
    fn a_huge_roll_is_printed_as_a_number_rather_than_saturating() {
        let got = modifier_text("# to maximum Life", Some(1.0e18), false);

        assert!(
            !got.contains("9223372036854775807"),
            "an i64 cast saturates and prints a number nobody rolled: {got}"
        );
    }

    #[test]
    fn an_item_level_the_item_carries_becomes_an_editable_row() {
        let view = build(&one_stat());

        let row = view
            .numerics
            .iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::ItemLevel))
            .expect("an item level row");

        assert_eq!(row.roll, Some(78.0));
        assert!(!row.enabled, "an unconstrained filter starts off");
    }

    #[test]
    fn a_row_that_is_off_still_offers_the_items_value_as_its_starting_point() {
        let view = build(&one_stat());

        let row = view
            .numerics
            .iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::ItemLevel))
            .expect("an item level row");

        assert_eq!(row.min, Some(78.0));
    }

    #[test]
    fn a_constrained_numeric_reads_back_as_enabled() {
        let mut c = one_stat();
        c.query.filters.type_filters.ilvl = Range::at_least(80.0);

        let view = build(&c);
        let row = view
            .numerics
            .iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::ItemLevel))
            .expect("a row");

        assert!(row.enabled);
        assert_eq!(row.min, Some(80.0));
    }

    #[test]
    fn a_stat_filter_is_labelled_with_the_text_the_game_printed() {
        let view = build(&one_stat());

        assert_eq!(view.stats.len(), 1);
        assert_eq!(view.stats[0].label, "+80 to maximum Life");
    }

    #[test]
    fn a_stat_filter_with_no_source_falls_back_to_its_trade_id() {
        let mut query = TradeQuery::default();
        query.stats.push(StatGroup::all(vec![StatFilter::range(
            "explicit.stat_unknown",
            Range::default(),
        )]));

        let view = build(&check(ring(), query, Vec::new()));

        assert_eq!(view.stats[0].label, "explicit.stat_unknown");
    }

    fn gauge_row(min: Option<f64>, max: Option<f64>, decimals: bool) -> Row {
        Row {
            key: RowKey::Stat { group: 0, index: 0 },
            label: "increased Spirit".into(),
            enabled: true,
            min,
            max,
            roll: Some(11.0),
            bounds: Some((10.0, 20.0)),
            decimals,
            tier: None,
            contributors: Vec::new(),
        }
    }

    #[test]
    fn a_modifier_with_an_unreadable_roll_keeps_its_placeholder_rather_than_printing_i64_max() {
        for roll in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(
                modifier_text("+# to maximum Life", Some(roll), false),
                "+# to maximum Life",
                "{roll} rendered as a number"
            );
        }
    }

    #[test]
    fn the_two_handles_stay_usable_when_they_sit_on_the_same_value() {
        let mut row = gauge_row(Some(15.0), Some(15.0), false);
        row.bounds = Some((10.0, 20.0));

        let high = gauge_edit(&row, 1.0).unwrap();

        assert!(!high.sets_min, "the max handle can never be moved again");
        assert_eq!(high.value, 20.0);

        let low = gauge_edit(&row, 0.0).unwrap();

        assert!(low.sets_min);
        assert_eq!(low.value, 10.0);
    }

    #[test]
    fn a_min_typed_above_the_max_is_pulled_back_inside_the_tier() {
        let row = gauge_row(Some(50.0), Some(10.0), false);

        let edit = gauge_edit(&row, 0.5).unwrap();

        assert!(
            edit.value >= 10.0 && edit.value <= 20.0,
            "{edit:?} left the tier"
        );
    }

    #[test]
    fn clicking_the_gauge_reads_a_value_off_the_tier_range() {
        let edit = gauge_edit(&gauge_row(None, None, false), 0.5).unwrap();

        assert_eq!(edit.value, 15.0);
    }

    #[test]
    fn clicking_near_the_low_end_moves_the_minimum() {
        let edit = gauge_edit(&gauge_row(Some(12.0), Some(18.0), false), 0.1).unwrap();

        assert!(edit.sets_min);
        assert_eq!(edit.value, 11.0);
    }

    #[test]
    fn clicking_near_the_high_end_moves_the_maximum() {
        let edit = gauge_edit(&gauge_row(Some(12.0), Some(18.0), false), 0.9).unwrap();

        assert!(!edit.sets_min);
        assert_eq!(edit.value, 19.0);
    }

    #[test]
    fn a_handle_never_crosses_the_other_one() {
        for (min, max, ratio) in [
            (11.0, 12.0, 1.0),
            (18.0, 19.0, 0.0),
            (19.0, 12.0, 0.0),
            (19.0, 12.0, 0.5),
            (19.0, 12.0, 1.0),
            (15.0, 15.0, 0.9),
        ] {
            let edit = gauge_edit(&gauge_row(Some(min), Some(max), false), ratio).unwrap();

            match edit.sets_min {
                true => assert!(
                    edit.value <= max,
                    "min {} passed max {max} for row {min}..{max} at {ratio}",
                    edit.value
                ),
                false => assert!(
                    edit.value >= min,
                    "max {} passed min {min} for row {min}..{max} at {ratio}",
                    edit.value
                ),
            }
        }
    }

    #[test]
    fn a_decimal_row_keeps_two_places_rather_than_snapping_to_whole_numbers() {
        let edit = gauge_edit(&gauge_row(None, None, true), 0.333).unwrap();

        assert_eq!(edit.value, 13.33);
    }

    #[test]
    fn a_click_outside_the_bar_is_clamped_rather_than_read_as_a_wild_value() {
        assert_eq!(
            gauge_edit(&gauge_row(None, None, false), -5.0)
                .unwrap()
                .value,
            10.0
        );
        assert_eq!(
            gauge_edit(&gauge_row(None, None, false), 5.0)
                .unwrap()
                .value,
            20.0
        );
    }

    #[test]
    fn a_row_with_no_tier_range_has_no_gauge_to_click() {
        let mut row = gauge_row(None, None, false);
        row.bounds = None;

        assert_eq!(gauge_edit(&row, 0.5), None);

        row.bounds = Some((10.0, 10.0));

        assert_eq!(gauge_edit(&row, 0.5), None);
    }

    #[test]
    fn a_stat_filter_carries_its_roll_and_the_bounds_of_its_tier() {
        let view = build(&one_stat());

        assert_eq!(view.stats[0].roll, Some(80.0));
        assert_eq!(view.stats[0].bounds, Some((60.0, 100.0)));
    }

    #[test]
    fn a_roll_at_the_middle_of_its_bounds_reads_as_half() {
        let view = build(&one_stat());

        assert_eq!(view.stats[0].percent_of_bounds(), Some(0.5));
    }

    #[test]
    fn a_roll_with_no_spread_has_no_percentage_rather_than_dividing_by_zero() {
        let mut c = one_stat();
        c.sources[0].roll = Some(roll(80.0, 80.0, 80.0));

        assert_eq!(build(&c).stats[0].percent_of_bounds(), None);
    }

    #[test]
    fn a_hidden_stat_filter_is_not_offered_to_the_user() {
        let mut c = one_stat();
        c.query.stats[0].filters[0].hidden = true;

        assert!(build(&c).stats.is_empty());
    }

    #[test]
    fn editing_a_stat_minimum_reaches_the_query() {
        let mut c = one_stat();
        let mut view = build(&c);

        view.stats[0].min = Some(95.0);

        apply(&view, &mut c.query);

        assert_eq!(c.query.stats[0].filters[0].range.min, Some(95.0));
    }

    #[test]
    fn turning_a_stat_off_disables_it_rather_than_removing_it() {
        let mut c = one_stat();
        let mut view = build(&c);

        view.stats[0].enabled = false;

        apply(&view, &mut c.query);

        assert!(c.query.stats[0].filters[0].disabled);
        assert!(c.query.stats[0].filters[0].range.is_empty());
    }

    #[test]
    fn turning_a_numeric_on_writes_its_range_into_the_query() {
        let mut c = one_stat();
        let mut view = build(&c);

        let row = view
            .row_mut(RowKey::Numeric(NumericKey::ItemLevel))
            .expect("a row");
        row.enabled = true;
        row.min = Some(84.0);

        apply(&view, &mut c.query);

        assert_eq!(c.query.filters.type_filters.ilvl.min, Some(84.0));
    }

    #[test]
    fn turning_a_numeric_off_clears_the_range_it_had_set() {
        let mut c = one_stat();
        c.query.filters.type_filters.ilvl = Range::at_least(84.0);

        let mut view = build(&c);
        view.row_mut(RowKey::Numeric(NumericKey::ItemLevel))
            .expect("a row")
            .enabled = false;

        apply(&view, &mut c.query);

        assert!(c.query.filters.type_filters.ilvl.is_empty());
    }

    #[test]
    fn a_corrupted_item_offers_a_corruption_toggle() {
        let mut c = one_stat();
        c.item.is_corrupted = true;

        let view = build(&c);

        assert!(view.flags.iter().any(|f| f.key == FlagKey::Corrupted));
    }

    #[test]
    fn an_ordinary_item_offers_no_corruption_toggle() {
        assert!(build(&one_stat()).flags.is_empty());
    }

    #[test]
    fn turning_a_flag_on_asks_the_trade_site_for_it() {
        let mut c = one_stat();
        c.item.is_corrupted = true;

        let mut view = build(&c);
        let flag = view.flag_mut(FlagKey::Corrupted).expect("a flag");
        flag.enabled = true;
        flag.value = true;

        apply(&view, &mut c.query);

        assert_eq!(c.query.filters.misc_filters.corrupted, Some(true));
    }

    #[test]
    fn a_flag_left_off_asks_for_nothing_rather_than_asking_for_false() {
        let mut c = one_stat();
        c.item.is_corrupted = true;
        c.query.filters.misc_filters.corrupted = Some(true);

        let mut view = build(&c);
        view.flag_mut(FlagKey::Corrupted).expect("a flag").enabled = false;

        apply(&view, &mut c.query);

        assert_eq!(c.query.filters.misc_filters.corrupted, None);
    }

    #[test]
    fn a_flag_can_ask_for_the_absence_of_something() {
        let mut c = one_stat();
        c.item.is_mirrored = true;

        let mut view = build(&c);
        let flag = view.flag_mut(FlagKey::Mirrored).expect("a flag");
        flag.enabled = true;
        flag.value = false;

        apply(&view, &mut c.query);

        assert_eq!(c.query.filters.misc_filters.mirrored, Some(false));
    }

    #[test]
    fn a_quality_of_zero_is_not_worth_a_row() {
        let mut c = one_stat();
        c.item.quality = Some(0);

        assert!(!build(&c)
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::Quality)));
    }

    #[test]
    fn an_armour_value_becomes_a_row_even_when_nothing_constrains_it_yet() {
        let mut c = one_stat();
        c.item.armour.ar = Some(420.0);

        assert!(build(&c)
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::Armour)));
    }

    #[test]
    fn a_property_the_item_does_not_have_gets_no_row() {
        assert!(!build(&one_stat())
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::Spirit)));
    }

    #[test]
    fn a_dps_row_comes_from_the_filter_source_because_the_item_never_prints_it() {
        let mut c = one_stat();
        c.sources.push(FilterSource {
            id: "item.physical_dps".into(),
            text: "Physical DPS".into(),
            roll: Some(roll(310.5, 310.5, 310.5)),
            tier: None,
            contributors: Vec::new(),
        });

        let row = build(&c)
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::PhysicalDps))
            .expect("a physical dps row");

        assert_eq!(row.roll, Some(310.5));
        assert!(row.decimals);
    }

    #[test]
    fn a_view_counts_only_what_actually_constrains_the_search() {
        let mut c = one_stat();
        c.item.is_corrupted = true;

        let mut view = build(&c);
        view.stats[0].enabled = true;
        view.flag_mut(FlagKey::Corrupted).expect("a flag").enabled = true;

        assert_eq!(view.enabled_count(), 2);
    }

    #[test]
    fn an_item_with_nothing_to_filter_on_produces_an_empty_view() {
        let empty = check(ParsedItem::default(), TradeQuery::default(), Vec::new());

        assert!(build(&empty).is_empty());
    }

    #[test]
    fn a_row_that_is_gone_from_the_query_is_skipped_rather_than_panicking() {
        let mut c = one_stat();
        let view = build(&c);

        c.query.stats.clear();

        apply(&view, &mut c.query);
    }

    #[test]
    fn every_numeric_key_writes_somewhere_of_its_own() {
        let keys = [
            NumericKey::ItemLevel,
            NumericKey::Quality,
            NumericKey::GemLevel,
            NumericKey::AreaLevel,
            NumericKey::MapTier,
            NumericKey::RequiresLevel,
            NumericKey::Armour,
            NumericKey::Evasion,
            NumericKey::EnergyShield,
            NumericKey::Block,
            NumericKey::CritChance,
            NumericKey::AttackSpeed,
            NumericKey::Dps,
            NumericKey::PhysicalDps,
            NumericKey::ElementalDps,
            NumericKey::RuneSockets,
            NumericKey::Spirit,
            NumericKey::ReloadTime,
            NumericKey::GemSockets,
        ];

        for (at, key) in keys.iter().enumerate() {
            let mut query = TradeQuery::default();
            *numeric_target(*key, &mut query) = Range::exactly(at as f64);

            let mut seen = 0;

            for other in &keys {
                let mut probe = TradeQuery::default();
                *numeric_target(*other, &mut probe) = Range::exactly(at as f64);

                if probe == query {
                    seen += 1;
                }
            }

            assert_eq!(seen, 1, "{} shares a field with another key", key.label());
        }
    }

    #[test]
    fn every_flag_key_writes_somewhere_of_its_own() {
        let keys = [
            FlagKey::Corrupted,
            FlagKey::Mirrored,
            FlagKey::Sanctified,
            FlagKey::Veiled,
            FlagKey::Identified,
            FlagKey::AlternateArt,
            FlagKey::Fractured,
        ];

        for key in keys {
            let mut query = TradeQuery::default();
            *flag_target(key, &mut query) = Some(true);

            let matches = keys
                .iter()
                .filter(|other| {
                    let mut probe = TradeQuery::default();
                    *flag_target(**other, &mut probe) = Some(true);

                    probe == query
                })
                .count();

            assert_eq!(matches, 1, "{} shares a field", key.label());
        }
    }
    #[test]
    fn a_tier_reads_as_the_game_writes_it() {
        assert_eq!(tier_label(Some(3)).as_deref(), Some("T3"));
        assert_eq!(tier_label(None), None);
    }

    #[test]
    fn a_stat_row_carries_the_tier_its_modifier_rolled_at() {
        assert_eq!(build(&one_stat()).stats[0].tier, Some(3));
    }

    #[test]
    fn the_roll_is_written_into_the_modifier_text() {
        assert_eq!(
            modifier_text("# to maximum Life", Some(80.0), false),
            "80 to maximum Life"
        );
    }

    #[test]
    fn a_modifier_with_no_roll_keeps_its_placeholder() {
        assert_eq!(
            modifier_text("# to maximum Life", None, false),
            "# to maximum Life"
        );
    }

    #[test]
    fn a_decimal_roll_keeps_its_precision_in_the_text() {
        assert_eq!(
            modifier_text("#% increased Attack Speed", Some(12.5), true),
            "12.50% increased Attack Speed"
        );
    }

    #[test]
    fn only_the_first_placeholder_is_filled_because_the_rest_are_a_range() {
        assert_eq!(
            modifier_text("Adds # to # Fire Damage", Some(10.0), false),
            "Adds 10 to # Fire Damage"
        );
    }

    #[test]
    fn text_with_no_placeholder_is_left_alone() {
        assert_eq!(modifier_text("Corrupted", Some(1.0), false), "Corrupted");
    }

    #[test]
    fn the_whole_stat_block_can_be_switched_on_at_once() {
        let mut view = build(&one_stat());
        view.set_all_stats(false);

        assert!(!view.all_stats_on());

        view.set_all_stats(true);

        assert!(view.all_stats_on());
    }

    #[test]
    fn an_empty_stat_block_is_not_reported_as_all_on() {
        assert!(!FilterView::default().all_stats_on());
    }

    #[test]
    fn switching_the_whole_block_off_reaches_the_query() {
        let mut c = one_stat();
        let mut view = build(&c);

        view.set_all_stats(false);
        apply(&view, &mut c.query);

        assert!(c.query.stats[0].filters[0].disabled);
    }

    #[test]
    fn an_item_with_a_base_type_searches_by_that_base_type() {
        let mut c = one_stat();
        c.item.info.reference_name = "Sapphire Ring".into();

        let view = build(&c);

        assert_eq!(view.name.mode, NameMode::BaseType);
        assert_eq!(view.name.caption(), "Sapphire Ring");
    }

    #[test]
    fn a_uniques_own_name_is_used_when_the_query_carries_one() {
        let mut c = one_stat();
        c.query.name = Some(crate::types::query::NameField::new("Kaom's Heart", None));
        c.item.info.reference_name = "Glorious Plate".into();

        let view = build(&c);

        assert_eq!(view.name.mode, NameMode::Exact);
        assert_eq!(view.name.caption(), "Kaom's Heart");
    }

    #[test]
    fn the_name_can_be_widened_from_the_exact_item_to_the_base_type() {
        let mut c = one_stat();
        c.query.name = Some(crate::types::query::NameField::new("Kaom's Heart", None));
        c.item.info.reference_name = "Glorious Plate".into();

        let mut view = build(&c);
        view.cycle_name();

        apply(&view, &mut c.query);

        assert_eq!(view.name.mode, NameMode::BaseType);
        assert_eq!(c.query.name, None);
        assert_eq!(
            c.query.type_name.as_ref().map(|n| n.name().to_string()),
            Some("Glorious Plate".to_string())
        );
    }

    #[test]
    fn widening_twice_searches_any_base_type() {
        let mut c = one_stat();
        c.query.name = Some(crate::types::query::NameField::new("Kaom's Heart", None));
        c.item.info.reference_name = "Glorious Plate".into();

        let mut view = build(&c);
        view.cycle_name();
        view.cycle_name();

        apply(&view, &mut c.query);

        assert_eq!(view.name.mode, NameMode::Any);
        assert_eq!(c.query.name, None);
        assert_eq!(c.query.type_name, None);
    }

    #[test]
    fn widening_to_the_base_type_drops_the_exact_name_but_keeps_the_base() {
        let mut c = one_stat();
        c.item.info.reference_name = "Sapphire Ring".into();

        let mut view = build(&c);
        view.name.mode = NameMode::BaseType;

        apply(&view, &mut c.query);

        assert_eq!(c.query.name, None);
        assert_eq!(
            c.query.type_name.as_ref().map(|n| n.name().to_string()),
            Some("Sapphire Ring".to_string())
        );
    }

    #[test]
    fn narrowing_back_to_the_exact_item_restores_its_name() {
        let mut c = one_stat();
        c.item.info.name = "Doom Loop".into();
        c.item.info.reference_name = "Sapphire Ring".into();

        let mut view = build(&c);
        view.name.mode = NameMode::Exact;

        apply(&view, &mut c.query);

        assert_eq!(
            c.query.name.as_ref().map(|n| n.name().to_string()),
            Some("Doom Loop".to_string())
        );
    }

    #[test]
    fn an_item_with_no_name_at_all_searches_anything() {
        let bare = check(ParsedItem::default(), TradeQuery::default(), Vec::new());
        let view = build(&bare);

        assert_eq!(view.name.mode, NameMode::Any);
        assert_eq!(view.name.caption(), "any base type");
    }

    #[test]
    fn the_name_mode_cycles_back_around() {
        assert_eq!(NameMode::Exact.next(), NameMode::BaseType);
        assert_eq!(NameMode::BaseType.next(), NameMode::Any);
        assert_eq!(NameMode::Any.next(), NameMode::Exact);
    }

    #[test]
    fn every_name_mode_says_what_it_searches() {
        for mode in [NameMode::Exact, NameMode::BaseType, NameMode::Any] {
            assert!(!mode.label().is_empty());
        }
    }

    #[test]
    fn a_total_made_of_one_modifier_lists_no_contributors() {
        assert!(build(&one_stat()).stats[0].contributors.is_empty());
    }

    #[test]
    fn a_total_made_of_several_modifiers_lists_them() {
        let mut c = one_stat();
        c.sources[0].contributors =
            vec!["+40 to maximum Life".into(), "+40 to maximum Life".into()];

        assert_eq!(build(&c).stats[0].contributors.len(), 2);
    }

    #[test]
    fn an_unidentified_item_is_priced_by_its_item_level_without_being_asked() {
        let mut c = one_stat();
        c.item.is_unidentified = true;

        let row = build(&c)
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::ItemLevel))
            .expect("an item level row");

        assert!(row.enabled, "there is nothing else to price it by");
        assert_eq!(row.min, Some(78.0));
    }

    #[test]
    fn an_identified_item_leaves_the_item_level_alone() {
        let row = build(&one_stat())
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::ItemLevel))
            .expect("an item level row");

        assert!(!row.enabled);
    }

    #[test]
    fn the_item_level_an_unidentified_item_forces_on_reaches_the_query() {
        let mut c = one_stat();
        c.item.is_unidentified = true;

        let view = build(&c);
        apply(&view, &mut c.query);

        assert_eq!(c.query.filters.type_filters.ilvl.min, Some(78.0));
    }

    #[test]
    fn armour_is_offered_at_twenty_quality_so_two_items_compare() {
        let mut c = one_stat();
        c.item.quality = Some(0);
        c.item.armour.ar = Some(100.0);
        c.item.info.craftable = true;

        let row = build(&c)
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::Armour))
            .expect("an armour row");

        assert_eq!(row.roll, Some(120.0));
    }

    #[test]
    fn an_item_whose_quality_cannot_change_is_offered_as_printed() {
        let mut c = one_stat();

        c.item.quality = Some(0);
        c.item.armour.ar = Some(100.0);
        c.item.info.craftable = true;
        c.item.is_corrupted = true;

        let row = build(&c)
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::Armour))
            .expect("an armour row");

        assert_eq!(
            row.roll,
            Some(100.0),
            "a corrupted item cannot be raised to twenty quality, so nobody lists it that way"
        );
    }

    #[test]
    fn an_item_already_at_twenty_quality_is_left_alone() {
        let mut c = one_stat();
        c.item.quality = Some(20);
        c.item.armour.ar = Some(120.0);
        c.item.info.craftable = true;

        let row = build(&c)
            .numerics
            .into_iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::Armour))
            .expect("an armour row");

        assert_eq!(row.roll, Some(120.0));
    }

    #[test]
    fn a_weapon_only_property_is_not_offered_on_armour() {
        let mut c = one_stat();
        c.item.category = Some(crate::types::category::ItemCategory::BodyArmour);
        c.item.weapon.crit = Some(6.5);

        assert!(!build(&c)
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::CritChance)));
    }

    #[test]
    fn a_weapon_keeps_its_own_properties() {
        let mut c = one_stat();
        c.item.category = Some(crate::types::category::ItemCategory::Bow);
        c.item.weapon.crit = Some(6.5);

        assert!(build(&c)
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::CritChance)));
    }

    #[test]
    fn an_influenced_item_says_which_influences_it_carries() {
        use crate::types::item::Influence;

        let mut c = one_stat();
        c.item.influences = vec![Influence::Shaper, Influence::Elder];

        let labels = influence_labels(&c.item);

        assert_eq!(labels, vec!["Shaper", "Elder"]);
    }

    #[test]
    fn an_ordinary_item_carries_no_influence() {
        assert!(influence_labels(&one_stat().item).is_empty());
    }

    #[test]
    fn a_ring_is_not_offered_an_armour_filter() {
        let mut c = one_stat();
        c.item.category = Some(crate::types::category::ItemCategory::Ring);
        c.item.armour.ar = Some(50.0);

        assert!(!build(&c)
            .numerics
            .iter()
            .any(|r| r.key == RowKey::Numeric(NumericKey::Armour)));
    }

    #[test]
    fn an_armour_row_carries_the_span_its_modifiers_allow() {
        use crate::types::category::ItemCategory;
        use crate::types::item::ItemRarity;

        use crate::controller::parse::shared::modifiers::ParsedModifier;
        use crate::types::modifier::{ModifierInfo, ModifierType};
        use crate::types::stat::{ParsedStat, StatRoll};

        let mut item = ParsedItem {
            category: Some(ItemCategory::BodyArmour),
            rarity: Some(ItemRarity::Rare),
            armour: crate::types::item::ArmourStats {
                ar: Some(300.0),
                ..crate::types::item::ArmourStats::default()
            },
            ..ParsedItem::default()
        };

        item.modifiers.push(ParsedModifier {
            info: ModifierInfo {
                kind: Some(ModifierType::Explicit),
                ..ModifierInfo::default()
            },
            stats: vec![ParsedStat {
                reference: "#% increased Armour".into(),
                matched: "#% increased Armour".into(),
                roll: Some(StatRoll {
                    value: 50.0,
                    min: 40.0,
                    max: 60.0,
                    ..StatRoll::default()
                }),
            }],
        });

        let mut query = TradeQuery::default();

        query.filters.equipment_filters.ar = Range::at_least(300.0);

        let check = check(item, query, Vec::new());
        let view = build(&check);
        let row = view
            .numerics
            .iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::Armour))
            .expect("an armour row");

        let (min, max) = row.bounds.expect("the span the modifier allows");

        assert!(min < max, "{min} to {max}");
        assert!(min < 300.0 && max > 300.0, "{min} to {max}");
    }

    #[test]
    fn a_property_with_nothing_contributing_shows_no_span() {
        use crate::types::category::ItemCategory;
        use crate::types::item::ItemRarity;

        let item = ParsedItem {
            category: Some(ItemCategory::BodyArmour),
            rarity: Some(ItemRarity::Rare),
            armour: crate::types::item::ArmourStats {
                ar: Some(300.0),
                ..crate::types::item::ArmourStats::default()
            },
            ..ParsedItem::default()
        };

        let mut query = TradeQuery::default();

        query.filters.equipment_filters.ar = Range::at_least(300.0);

        let check = check(item, query, Vec::new());
        let view = build(&check);
        let row = view
            .numerics
            .iter()
            .find(|r| r.key == RowKey::Numeric(NumericKey::Armour))
            .expect("an armour row");

        assert_eq!(row.bounds, None);
    }
}
