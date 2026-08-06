//! Choosing between the two trade endpoints, and reading a bulk listing.
//!
//! Ported from `web/price-check/trade/common.ts` and
//! `web/price-check/trade/pathofexile-bulk.ts`.
//!
//! # Why there are two endpoints
//!
//! The search endpoint prices one item against listings of that item. The
//! exchange endpoint prices a stack of currency against a stack of other
//! currency. They take different requests and return different shapes.
//!
//! Sending a chaos orb to the search endpoint returns the handful of people
//! who listed one individually rather than the market rate. Sending a rare
//! ring to the exchange endpoint returns nothing at all.

use crate::types::category::ItemCategory;

/// Which endpoint a search should go to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    /// One item against listings of that item.
    Search,
    /// A stack against a stack.
    Exchange,
}

/// What the routing decision needs to know.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteFacts {
    /// The item's own bulk trading tag. Absent for anything not traded in bulk.
    pub trade_tag: Option<String>,
    pub category: Option<ItemCategory>,
    /// The user asked for a minimum stack size.
    pub stack_size_active: bool,
    /// The user left a stack size filter in place but switched it off.
    pub has_stack_size_filter: bool,
    /// Any modifier filter is switched on.
    pub any_stat_enabled: bool,
}

/// Which endpoint satisfies this search.
///
/// Ported from `apiToSatisfySearch`.
///
/// # The order of the checks matters
///
/// A modifier filter wins outright. The exchange endpoint has no concept of a
/// modifier, so a currency item with a stat filter switched on must still go
/// to the search endpoint or the filter is silently dropped.
pub fn endpoint_for(facts: &RouteFacts) -> Endpoint {
    // The exchange endpoint cannot express a modifier filter, so honouring one
    // means using the search endpoint whatever else is true.
    if facts.any_stat_enabled {
        return Endpoint::Search;
    }

    // A divination card and a map are the two things that are both bulk
    // tradeable and worth filtering by stack size. The user switching that
    // filter off is them saying they want the individual listings.
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

/// The tag the exchange endpoint knows this item by.
///
/// Ported from `tradeTag`. It is the base's own tag and nothing derived, so a
/// base our data does not carry has none and routes to the search endpoint
/// rather than to an exchange request the server would reject.
pub fn trade_tag(facts: &RouteFacts) -> Option<&str> {
    facts.trade_tag.as_deref()
}

/// Whether a queue would form before this search could run.
///
/// Ported from `preventQueueCreation`. Returns the wait in milliseconds when
/// the search would sit behind other requests, and nothing when it can go now.
///
/// # Why refuse rather than queue
///
/// The user pressed a key expecting a price. A search that runs four seconds
/// later prices an item they have already put down, and the rate limiter is
/// not optional: GGG bans for violations, so the queue cannot simply be
/// skipped.
///
/// A wait that is the same with a clean limiter is not a queue. It is the
/// endpoint being slow by nature, and refusing there would refuse every
/// search forever.
pub fn queue_wait(estimated_millis: u64, clean_millis: u64) -> Option<u64> {
    if estimated_millis == clean_millis {
        return None;
    }

    // Under this the user does not notice, and refusing costs them a price
    // check that would have worked.
    if estimated_millis < 1500 {
        return None;
    }

    Some(estimated_millis)
}

/// The longest wait across several endpoints.
///
/// A search that hits two endpoints waits for the slower one, so the shorter
/// wait says nothing about whether it can run.
pub fn longest_queue_wait(waits: &[(u64, u64)]) -> Option<u64> {
    waits
        .iter()
        .filter_map(|&(estimated, clean)| queue_wait(estimated, clean))
        .max()
}

/// Whether a seller can be whispered right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellerStatus {
    Online,
    /// Logged in but away. A whisper arrives and may sit unread.
    Afk,
    Offline,
}

/// Read a seller's status from what the listing says.
///
/// Ported from the `accountStatus` mapping in `toPricingResult`. Away is its
/// own state rather than online, because a buyer picking between two identical
/// prices picks the one who will answer.
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

/// One bulk listing, read into what the overlay shows.
#[derive(Debug, Clone, PartialEq)]
pub struct BulkListing {
    pub id: String,
    /// What the seller wants.
    pub exchange_amount: f64,
    /// What the seller gives.
    pub item_amount: f64,
    /// How many they have.
    pub stock: u32,
    /// The listing belongs to the user.
    ///
    /// A user pricing against their own listing prices against themselves, so
    /// the overlay marks it rather than silently including it.
    pub is_mine: bool,
    pub account_name: String,
    pub character_name: String,
    pub status: SellerStatus,
}

/// The rate at which a listing trades, as a multiple.
///
/// A listing of 100 chaos for 1 divine is a rate of 100. Comparing raw amounts
/// across listings is meaningless because sellers list different stack sizes.
///
/// Returns nothing when the seller gives nothing, which is a malformed listing
/// rather than a free item.
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
        // The search endpoint returns the handful of people who listed one
        // individually rather than the market rate.
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
        // The exchange endpoint has no concept of a modifier, so sending it
        // there drops the filter silently.
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
        // Switching it off is the user saying they want individual listings.
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
        // Only cards and maps are both bulk tradeable and worth filtering by
        // stack size.
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

    // -----------------------------------------------------------------
    // Queueing
    // -----------------------------------------------------------------

    #[test]
    fn a_bulk_tradeable_item_reports_its_tag() {
        assert_eq!(trade_tag(&currency()), Some("chaos"));
    }

    #[test]
    fn an_item_our_data_does_not_carry_reports_no_tag() {
        // It then routes to the search endpoint rather than to an exchange
        // request the server would reject.
        assert_eq!(trade_tag(&rare_ring()), None);
    }

    #[test]
    fn a_search_that_can_run_now_does_not_wait() {
        assert_eq!(queue_wait(0, 0), None);
    }

    #[test]
    fn a_long_queue_is_refused() {
        // A search that runs four seconds later prices an item the user has
        // already put down.
        assert_eq!(queue_wait(4000, 0), Some(4000));
    }

    #[test]
    fn a_short_queue_is_allowed_through() {
        // Under a second and a half the user does not notice, and refusing
        // costs them a price check that would have worked.
        assert_eq!(queue_wait(900, 0), None);
    }

    #[test]
    fn a_wait_exactly_on_the_threshold_is_refused() {
        assert_eq!(queue_wait(1500, 0), Some(1500));
    }

    #[test]
    fn a_wait_that_is_the_same_when_clean_is_not_a_queue() {
        // It is the endpoint being slow by nature, and refusing there would
        // refuse every search forever.
        assert_eq!(queue_wait(9000, 9000), None);
    }

    #[test]
    fn the_slower_endpoint_decides() {
        // A search that hits two endpoints waits for the slower one.
        assert_eq!(longest_queue_wait(&[(2000, 0), (5000, 0)]), Some(5000));
    }

    #[test]
    fn nothing_waiting_means_no_wait() {
        assert_eq!(longest_queue_wait(&[(0, 0), (900, 0)]), None);
        assert_eq!(longest_queue_wait(&[]), None);
    }

    // -----------------------------------------------------------------
    // Listings
    // -----------------------------------------------------------------

    #[test]
    fn an_offline_seller_reads_as_offline() {
        assert_eq!(seller_status(false, false), SellerStatus::Offline);
    }

    #[test]
    fn an_offline_seller_cannot_be_away() {
        // Away is a state of being logged in.
        assert_eq!(seller_status(false, true), SellerStatus::Offline);
    }

    #[test]
    fn an_away_seller_is_its_own_state() {
        // A buyer picking between two identical prices picks the one who will
        // answer.
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
        // Comparing raw amounts is meaningless because sellers list different
        // stack sizes.
        assert_eq!(exchange_rate(&listing(100.0, 1.0)), Some(100.0));
        assert_eq!(exchange_rate(&listing(200.0, 2.0)), Some(100.0));
    }

    #[test]
    fn a_listing_that_gives_nothing_has_no_rate() {
        // That is a malformed listing rather than a free item.
        assert_eq!(exchange_rate(&listing(100.0, 0.0)), None);
    }

    #[test]
    fn a_fractional_rate_is_kept() {
        assert_eq!(exchange_rate(&listing(1.0, 4.0)), Some(0.25));
    }
}
