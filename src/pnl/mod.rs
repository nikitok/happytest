pub mod models;
pub mod fifo;
pub mod position;
pub mod unrealized;
pub mod calculator;

#[cfg(test)]
mod tests;

pub use models::{PnlMethod, PnlRecord};
pub use calculator::{PnlReport, PnlProcessor};
pub use fifo::FifoPnlProcessor;
pub use position::PositionPnlProcessor;
pub use unrealized::calculate_unrealized_pnl;