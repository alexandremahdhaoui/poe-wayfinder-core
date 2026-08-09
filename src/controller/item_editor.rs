use crate::controller::filter::augments::{
    effect_for_category, preview_filters, Augment, FilterEdit,
};
use crate::types::category::ItemCategory;
use crate::types::item::{ItemRarity, ParsedItem};
use crate::types::query::{StatFilter, TradeQuery};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    None,
    Augment,
    Catalyst,
}

pub fn editor_kind(item: &ParsedItem) -> EditorKind {
    if item.is_virtual() {
        return EditorKind::None;
    }

    let Some(category) = item.category else {
        return EditorKind::None;
    };

    if matches!(category, ItemCategory::Ring | ItemCategory::Amulet) {
        return EditorKind::Catalyst;
    }

    if item.rarity == Some(ItemRarity::Unique) {
        return EditorKind::None;
    }

    match takes_an_augment(category) {
        true => EditorKind::Augment,
        false => EditorKind::None,
    }
}

fn takes_an_augment(category: ItemCategory) -> bool {
    use ItemCategory::*;

    matches!(
        category,
        Bow | Claw
            | Crossbow
            | Dagger
            | Flail
            | OneHandedAxe
            | OneHandedMace
            | OneHandedSword
            | Spear
            | Staff
            | Warstaff
            | TwoHandedAxe
            | TwoHandedMace
            | TwoHandedSword
            | Wand
            | Sceptre
            | Talisman
            | Helmet
            | BodyArmour
            | Boots
            | Gloves
            | Shield
            | Buckler
            | Focus
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct AugmentOption {
    pub reference_name: String,
    pub name: String,
    pub text: String,
    pub value: f64,
    pub element: Option<crate::controller::calc::preview::RuneElement>,
}

pub fn augment_options(
    augments: &[Augment],
    item: &ParsedItem,
    sockets: u32,
) -> Vec<AugmentOption> {
    if editor_kind(item) != EditorKind::Augment {
        return Vec::new();
    }

    if sockets == 0 {
        return Vec::new();
    }

    let Some(category) = item.category else {
        return Vec::new();
    };

    let mut out: Vec<AugmentOption> = augments
        .iter()
        .filter_map(|augment| {
            let effect = effect_for_category(augment, category)?;

            Some(AugmentOption {
                reference_name: augment.reference_name.clone(),
                name: augment.name.clone(),
                text: effect.reference.clone(),
                value: effect.value * f64::from(sockets),
                element: crate::controller::calc::preview::rune_element(&augment.name),
            })
        })
        .collect();

    out.sort_by(|a, b| {
        rank(&a.reference_name)
            .cmp(&rank(&b.reference_name))
            .then_with(|| a.name.cmp(&b.name))
    });

    out
}

fn rank(reference: &str) -> u8 {
    if reference.contains(" Rune") {
        return 0;
    }

    if reference.contains("Soul Core of") {
        return 1;
    }

    if reference.contains("Talisman") {
        return 2;
    }

    3
}

pub fn empty_sockets(item: &ParsedItem) -> u32 {
    item.augment_sockets.map(|s| s.empty).unwrap_or(0)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemEditor {
    chosen: Option<String>,
    edit: FilterEdit,
}

impl ItemEditor {
    pub fn chosen(&self) -> Option<&str> {
        self.chosen.as_deref()
    }

    pub fn is_applied(&self) -> bool {
        self.edit.is_applied()
    }

    pub fn choose(
        &mut self,
        reference_name: &str,
        augments: &[Augment],
        item: &ParsedItem,
        query: &mut TradeQuery,
    ) -> bool {
        self.clear_augment(query);

        let Some(augment) = augments.iter().find(|a| a.reference_name == reference_name) else {
            return false;
        };

        let Some(category) = item.category else {
            return false;
        };

        let sockets = empty_sockets(item);

        let Some(group) = first_group_mut(query) else {
            return false;
        };

        let Some(replacement) = preview_filters(augment, category, sockets, group) else {
            return false;
        };

        self.edit.apply(group, replacement);
        self.chosen = Some(reference_name.to_string());

        true
    }

    pub fn clear_augment(&mut self, query: &mut TradeQuery) {
        self.chosen = None;

        if let Some(group) = first_group_mut(query) {
            self.edit.remove(group);
        }
    }
}

fn first_group_mut(query: &mut TradeQuery) -> Option<&mut Vec<StatFilter>> {
    query.stats.first_mut().map(|group| &mut group.filters)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::controller::filter::augments::AugmentEffect;
    use crate::types::item::AugmentSockets;
    use crate::types::query::{Range, StatGroup};

    fn rune(name: &str, reference: &str, value: f64) -> Augment {
        Augment {
            reference_name: name.to_string(),
            name: name.to_string(),
            effects: vec![AugmentEffect {
                reference: reference.to_string(),
                trade_id: format!("rune.{reference}"),
                value,
                categories: vec![ItemCategory::BodyArmour],
            }],
        }
    }

    fn armour() -> ParsedItem {
        ParsedItem {
            category: Some(ItemCategory::BodyArmour),
            rarity: Some(ItemRarity::Rare),
            augment_sockets: Some(AugmentSockets {
                empty: 2,
                current: 0,
                normal: 2,
            }),
            ..ParsedItem::default()
        }
    }

    fn rune_id(reference: &str) -> String {
        format!("rune.{reference}")
    }

    fn query_with(id: &str, min: f64) -> TradeQuery {
        let mut query = TradeQuery::default();
        query.stats.push(StatGroup::all(vec![StatFilter::range(
            id,
            Range::at_least(min),
        )]));

        query
    }

    #[test]
    fn a_rare_body_armour_takes_an_augment() {
        assert_eq!(editor_kind(&armour()), EditorKind::Augment);
    }

    #[test]
    fn a_unique_takes_none_because_its_rolls_are_fixed() {
        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            ..armour()
        };

        assert_eq!(editor_kind(&item), EditorKind::None);
    }

    #[test]
    fn a_ring_takes_a_catalyst_rather_than_a_rune() {
        let item = ParsedItem {
            category: Some(ItemCategory::Ring),
            ..armour()
        };

        assert_eq!(editor_kind(&item), EditorKind::Catalyst);
    }

    #[test]
    fn an_item_with_no_category_takes_nothing() {
        let item = ParsedItem {
            category: None,
            ..armour()
        };

        assert_eq!(editor_kind(&item), EditorKind::None);
    }

    #[test]
    fn an_item_the_overlay_made_up_takes_nothing() {
        let item = ParsedItem {
            raw_text: "VIRTUAL_ITEM".to_string(),
            ..armour()
        };

        assert_eq!(editor_kind(&item), EditorKind::None);
    }

    #[test]
    fn a_flask_takes_nothing() {
        let item = ParsedItem {
            category: Some(ItemCategory::Flask),
            ..armour()
        };

        assert_eq!(editor_kind(&item), EditorKind::None);
    }

    #[test]
    fn only_the_augments_that_fit_the_category_are_offered() {
        let augments = vec![
            rune("Body Rune", "# to Armour", 10.0),
            Augment {
                reference_name: "Bow Rune".into(),
                name: "Bow Rune".into(),
                effects: vec![AugmentEffect {
                    reference: "# to Damage".into(),
                    trade_id: "rune.damage".into(),
                    value: 5.0,
                    categories: vec![ItemCategory::Bow],
                }],
            },
        ];

        let got = augment_options(&augments, &armour(), 2);

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Body Rune");
    }

    #[test]
    fn the_offered_value_counts_every_empty_socket() {
        let augments = vec![rune("Body Rune", "# to Armour", 10.0)];

        assert_eq!(augment_options(&augments, &armour(), 2)[0].value, 20.0);
    }

    #[test]
    fn an_item_with_no_empty_socket_is_offered_nothing() {
        let augments = vec![rune("Body Rune", "# to Armour", 10.0)];

        assert!(augment_options(&augments, &armour(), 0).is_empty());
    }

    #[test]
    fn a_unique_is_offered_nothing_even_with_sockets() {
        let augments = vec![rune("Body Rune", "# to Armour", 10.0)];

        let item = ParsedItem {
            rarity: Some(ItemRarity::Unique),
            ..armour()
        };

        assert!(augment_options(&augments, &item, 2).is_empty());
    }

    #[test]
    fn an_elemental_rune_is_recognised_as_one() {
        use crate::controller::calc::preview::RuneElement;

        let augments = vec![rune("Desert Rune", "# to Fire Damage", 10.0)];

        assert_eq!(
            augment_options(&augments, &armour(), 1)[0].element,
            Some(RuneElement::Fire)
        );
    }

    #[test]
    fn an_ordinary_rune_carries_no_element() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];

        assert_eq!(augment_options(&augments, &armour(), 1)[0].element, None);
    }

    #[test]
    fn runes_are_listed_before_soul_cores() {
        let augments = vec![
            rune("Soul Core of Zantipi", "# to Armour", 10.0),
            rune("Iron Rune", "# to Armour", 10.0),
        ];

        let got = augment_options(&augments, &armour(), 1);

        assert_eq!(got[0].name, "Iron Rune");
        assert_eq!(got[1].name, "Soul Core of Zantipi");
    }

    #[test]
    fn choosing_an_augment_raises_the_filter_it_grants() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let mut editor = ItemEditor::default();

        assert!(editor.choose("Iron Rune", &augments, &armour(), &mut query));

        assert_eq!(query.stats[0].filters[0].range.min, Some(120.0));
    }

    #[test]
    fn choosing_an_augment_the_item_does_not_carry_adds_the_filter() {
        let augments = vec![rune("Iron Rune", "# to Evasion", 10.0)];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let mut editor = ItemEditor::default();

        editor.choose("Iron Rune", &augments, &armour(), &mut query);

        assert_eq!(query.stats[0].filters.len(), 2);
    }

    #[test]
    fn taking_the_augment_back_off_restores_what_was_there() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let before = query.clone();
        let mut editor = ItemEditor::default();

        editor.choose("Iron Rune", &augments, &armour(), &mut query);
        editor.clear_augment(&mut query);

        assert_eq!(query, before);
        assert_eq!(editor.chosen(), None);
    }

    #[test]
    fn choosing_a_second_augment_replaces_the_first_rather_than_stacking() {
        let augments = vec![
            rune("Iron Rune", "# to Armour", 10.0),
            rune("Desert Rune", "# to Armour", 30.0),
        ];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let mut editor = ItemEditor::default();

        editor.choose("Iron Rune", &augments, &armour(), &mut query);
        editor.choose("Desert Rune", &augments, &armour(), &mut query);

        assert_eq!(query.stats[0].filters[0].range.min, Some(160.0));
        assert_eq!(editor.chosen(), Some("Desert Rune"));
    }

    #[test]
    fn choosing_an_augment_that_is_not_in_the_list_changes_nothing() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let before = query.clone();
        let mut editor = ItemEditor::default();

        assert!(!editor.choose("Nothing", &augments, &armour(), &mut query));
        assert_eq!(query, before);
    }

    #[test]
    fn choosing_an_augment_on_a_query_with_no_filters_changes_nothing() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];
        let mut query = TradeQuery::default();
        let mut editor = ItemEditor::default();

        assert!(!editor.choose("Iron Rune", &augments, &armour(), &mut query));
        assert!(!editor.is_applied());
    }

    #[test]
    fn clearing_when_nothing_was_chosen_is_harmless() {
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let before = query.clone();

        ItemEditor::default().clear_augment(&mut query);

        assert_eq!(query, before);
    }

    #[test]
    fn an_applied_augment_says_so() {
        let augments = vec![rune("Iron Rune", "# to Armour", 10.0)];
        let mut query = query_with(&rune_id("# to Armour"), 100.0);
        let mut editor = ItemEditor::default();

        editor.choose("Iron Rune", &augments, &armour(), &mut query);

        assert!(editor.is_applied());
    }
}
