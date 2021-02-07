
// /// Money matters.
// pub mod currency {
//     use crate::common::Balance;
//
//     pub const BDTS: Balance = 1_000_000_000_000;
//     pub const DOLLARS: Balance = BDTS / 100;       // 10_000_000_000
//     pub const CENTS: Balance = DOLLARS / 100;      // 100_000_000
//     pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000
// }

/// Time and blocks.
pub mod time {
    use crate::common::{Moment, BlockNumber};

    pub const MILLISECS_PER_BLOCK: Moment = 6000;

    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

/// Fee-related.
pub mod fee {
    use sp_runtime::Perbill;
    pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
}
