pub mod calculator;
pub mod fifo;
pub mod models;
pub mod position;
pub mod unrealized;

#[cfg(test)]
mod tests {
    mod integration;
    mod unit;
}

pub use calculator::{EquityMetrics, PnlReport, Processor};
pub use fifo::FifoProcessor;
pub use models::{Method, Record};
pub use position::PositionProcessor;
pub use unrealized::calculate_unrealized_pnl;
