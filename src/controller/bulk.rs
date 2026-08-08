use crate::types::category::ItemCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    Search,
    Exchange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteFacts {
    pub trade_tag: Option<String>,
    pub category: Option<ItemCategory>,
    pub stack_size_active: bool,
    pub has_stack_size_filter: bool,
    pub any_stat_enabled: bool,
}

pub fn endpoint_for(facts: &RouteFacts) -> Endpoint {
    if facts.any_stat_enabled {
        return Endpoint::Search;
    }

    if facts.has_stack_size_filter
        && matches!(
            facts.category,
            Some(ItemCategory::DivinationCard | ItemCategory::Map)
        )
    {
        return if facts.stack_size_active {
            Endpoint::Exchange
        } else {
            Endpoint::Search
        };
    }

    if facts.trade_tag.is_some() {
        Endpoint::Exchange
    } else {
        Endpoint::Search
    }
}

pub fn trade_tag(facts: &RouteFacts) -> Option<&str> {
    facts.trade_tag.as_deref()
}

pub fn queue_wait(estimated_millis: u64, clean_millis: u64) -> Option<u64> {
    if estimated_millis == clean_millis {
        return None;
    }

    if estimated_millis < 1500 {
        return None;
    }

    Some(estimated_millis)
}

pub fn longest_queue_wait(waits: &[(u64, u64)]) -> Option<u64> {
    waits
        .iter()
        .filter_map(|&(estimated, clean)| queue_wait(estimated, clean))
        .max()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellerStatus {
    Online,
    Afk,
    Offline,
}

pub fn seller_status(online: bool, away: bool) -> SellerStatus {
    if !online {
        return SellerStatus::Offline;
    }

    if away {
        SellerStatus::Afk
    } else {
        SellerStatus::Online
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BulkListing {
    pub id: String,
    pub exchange_amount: f64,
    pub item_amount: f64,
    pub stock: u32,
    pub is_mine: bool,
    pub account_name: String,
    pub character_name: String,
    pub status: SellerStatus,
}

pub fn exchange_rate(listing: &BulkListing) -> Option<f64> {
    if listing.item_amount == 0.0 {
        return None;
    }

    Some(listing.exchange_amount / listing.item_amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn currency() -> RouteFacts {
        RouteFacts {
            trade_tag: Some("chaos".into()),
            category: Some(ItemCategory::Currency),
            stack_size_active: false,
            has_stack_size_filter: false,
            any_stat_enabled: false,
        }
    }

    fn rare_ring() -> RouteFacts {
        RouteFacts {
            trade_tag: None,
            category: Some(ItemCategory::Ring),
            stack_size_active: false,
            has_stack_size_filter: false,
            any_stat_enabled: true,
        }
    }

    #[test]
    fn a_currency_goes_to_the_exchange() {
        assert_eq!(endpoint_for(&currency()), Endpoint::Exchange);
    }

    #[test]
    fn a_rare_goes_to_the_search() {
        assert_eq!(endpoint_for(&rare_ring()), Endpoint::Search);
    }

    #[test]
    fn an_item_with_no_bulk_tag_goes_to_the_search() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                trade_tag: None,
                any_stat_enabled: false,
                ..currency()
            }),
            Endpoint::Search
        );
    }

    #[test]
    fn a_modifier_filter_wins_outright() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                any_stat_enabled: true,
                ..currency()
            }),
            Endpoint::Search
        );
    }

    #[test]
    fn a_modifier_filter_beats_an_active_stack_size() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: Some(ItemCategory::DivinationCard),
                has_stack_size_filter: true,
                stack_size_active: true,
                any_stat_enabled: true,
                ..currency()
            }),
            Endpoint::Search
        );
    }

    #[test]
    fn a_card_with_an_active_stack_size_goes_to_the_exchange() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: Some(ItemCategory::DivinationCard),
                trade_tag: None,
                has_stack_size_filter: true,
                stack_size_active: true,
                any_stat_enabled: false,
            }),
            Endpoint::Exchange
        );
    }

    #[test]
    fn a_card_with_the_stack_size_switched_off_goes_to_the_search() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: Some(ItemCategory::DivinationCard),
                trade_tag: Some("card".into()),
                has_stack_size_filter: true,
                stack_size_active: false,
                any_stat_enabled: false,
            }),
            Endpoint::Search
        );
    }

    #[test]
    fn a_map_follows_the_same_stack_size_rule_as_a_card() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: Some(ItemCategory::Map),
                trade_tag: None,
                has_stack_size_filter: true,
                stack_size_active: true,
                any_stat_enabled: false,
            }),
            Endpoint::Exchange
        );
    }

    #[test]
    fn a_stack_size_filter_on_something_else_does_not_route_it() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: Some(ItemCategory::Ring),
                trade_tag: None,
                has_stack_size_filter: true,
                stack_size_active: true,
                any_stat_enabled: false,
            }),
            Endpoint::Search
        );
    }

    #[test]
    fn an_item_with_no_category_at_all_routes_by_its_tag() {
        assert_eq!(
            endpoint_for(&RouteFacts {
                category: None,
                ..currency()
            }),
            Endpoint::Exchange
        );
    }

    #[test]
    fn a_bulk_tradeable_item_reports_its_tag() {
        assert_eq!(trade_tag(&currency()), Some("chaos"));
    }

    #[test]
    fn an_item_our_data_does_not_carry_reports_no_tag() {
        assert_eq!(trade_tag(&rare_ring()), None);
    }

    #[test]
    fn a_search_that_can_run_now_does_not_wait() {
        assert_eq!(queue_wait(0, 0), None);
    }

    #[test]
    fn a_long_queue_is_refused() {
        assert_eq!(queue_wait(4000, 0), Some(4000));
    }

    #[test]
    fn a_short_queue_is_allowed_through() {
        assert_eq!(queue_wait(900, 0), None);
    }

    #[test]
    fn a_wait_exactly_on_the_threshold_is_refused() {
        assert_eq!(queue_wait(1500, 0), Some(1500));
    }

    #[test]
    fn a_wait_that_is_the_same_when_clean_is_not_a_queue() {
        assert_eq!(queue_wait(9000, 9000), None);
    }

    #[test]
    fn the_slower_endpoint_decides() {
        assert_eq!(longest_queue_wait(&[(2000, 0), (5000, 0)]), Some(5000));
    }

    #[test]
    fn nothing_waiting_means_no_wait() {
        assert_eq!(longest_queue_wait(&[(0, 0), (900, 0)]), None);
        assert_eq!(longest_queue_wait(&[]), None);
    }

    #[test]
    fn an_offline_seller_reads_as_offline() {
        assert_eq!(seller_status(false, false), SellerStatus::Offline);
    }

    #[test]
    fn an_offline_seller_cannot_be_away() {
        assert_eq!(seller_status(false, true), SellerStatus::Offline);
    }

    #[test]
    fn an_away_seller_is_its_own_state() {
        assert_eq!(seller_status(true, true), SellerStatus::Afk);
    }

    #[test]
    fn an_online_seller_reads_as_online() {
        assert_eq!(seller_status(true, false), SellerStatus::Online);
    }

    fn listing(exchange: f64, item: f64) -> BulkListing {
        BulkListing {
            id: "x".into(),
            exchange_amount: exchange,
            item_amount: item,
            stock: 100,
            is_mine: false,
            account_name: "seller".into(),
            character_name: "Char".into(),
            status: SellerStatus::Online,
        }
    }

    #[test]
    fn the_rate_is_what_the_seller_wants_per_unit_given() {
        assert_eq!(exchange_rate(&listing(100.0, 1.0)), Some(100.0));
        assert_eq!(exchange_rate(&listing(200.0, 2.0)), Some(100.0));
    }

    #[test]
    fn a_listing_that_gives_nothing_has_no_rate() {
        assert_eq!(exchange_rate(&listing(100.0, 0.0)), None);
    }

    #[test]
    fn a_fractional_rate_is_kept() {
        assert_eq!(exchange_rate(&listing(1.0, 4.0)), Some(0.25));
    }
}
