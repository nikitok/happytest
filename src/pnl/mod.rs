pub mod models;
pub mod fifo;
pub mod position;
pub mod unrealized;
pub mod calculator;

#[cfg(test)]
mod tests {
    mod unit;
    mod integration;
}

pub use models::{Method, Record};
pub use calculator::{PnlReport, Processor};
pub use fifo::FifoProcessor;
pub use position::PositionProcessor;
pub use unrealized::calculate_unrealized_pnl;