use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter, Write};
use std::path::Path;
use serde::Serialize;
use serde_json;
use anyhow::{Result, Context};

// Use OrderbookData from models
use super::models::OrderbookData;

/// Orderbook data format for backtest
#[derive(Debug, Serialize)]
struct BacktestOrderbookData {
    topic: String,
    ts: i64,
    data: OrderbookSnapshot,
}

#[derive(Debug, Serialize)]
struct OrderbookSnapshot {
    s: String, // symbol
    b: Vec<[String; 2]>, // bids
    a: Vec<[String; 2]>, // asks
    u: i64, // update id
    seq: i64, // sequence number
}

/// Convert reader format to backtest format
pub fn convert_reader_to_backtest(input_path: &Path, output_path: &Path) -> Result<()> {
    let input_file = File::open(input_path)
        .context("Failed to open input file")?;
    let reader = BufReader::new(input_file);
    
    let output_file = File::create(output_path)
        .context("Failed to create output file")?;
    let mut writer = BufWriter::new(output_file);
    
    let mut line_count = 0;
    let mut error_count = 0;
    
    for line in reader.lines() {
        line_count += 1;
        
        match line {
            Ok(json_line) => {
                match serde_json::from_str::<OrderbookData>(&json_line) {
                    Ok(reader_data) => {
                        // Convert to backtest format
                        let backtest_data = BacktestOrderbookData {
                            topic: format!("orderbook.50.{}", reader_data.symbol),
                            ts: reader_data.timestamp,
                            data: OrderbookSnapshot {
                                s: reader_data.symbol,
                                b: reader_data.bids,
                                a: reader_data.asks,
                                u: reader_data.update_id,
                                seq: reader_data.update_id, // Use update_id as sequence
                            },
                        };
                        
                        // Write to output
                        let json_line = serde_json::to_string(&backtest_data)
                            .context("Failed to serialize backtest data")?;
                        writeln!(writer, "{}", json_line)
                            .context("Failed to write to output file")?;
                    }
                    Err(e) => {
                        error_count += 1;
                        eprintln!("Error parsing line {}: {}", line_count, e);
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("Error reading line {}: {}", line_count, e);
            }
        }
    }
    
    writer.flush().context("Failed to flush writer")?;
    
    println!("Conversion complete:");
    println!("  Total lines: {}", line_count);
    println!("  Errors: {}", error_count);
    println!("  Output file: {}", output_path.display());
    
    Ok(())
}