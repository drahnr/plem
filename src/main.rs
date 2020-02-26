use std::io;

use common_failures::prelude::*;
use common_failures::quick_main;
use failure::{bail, format_err};

use docopt::Docopt;
use serde::Deserialize;

use log;
use log::{debug, info, trace, warn};
use pretty_env_logger;
use std::convert::{Into, TryInto};

mod header;

use header::*;

#[derive(Debug, Deserialize)]
struct Record {
    idx: u32,
    time_ms: u32,
}

impl Record {
    pub fn as_tuple(&self) -> (f64, f64) {
        (self.idx.into(), self.time_ms.into())
    }
}

const USAGE: &'static str = "
plem

Usage:
  plem [--title=<title>] [--label=<label>]  <file>
  plem --version

Options:
  --version            Show version.
  -h --help            Show this screen.
  --title=<title>      Header to use in the plot title.
  --label=<label>      Label name for the legend.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_version: bool,
    arg_file: std::path::PathBuf,
    flag_label: String,
    flag_title: String,
}

use plotters::prelude::*;

fn plot(
    dest: &std::path::Path,
    label: &str,
    title: &str,
    data: &[(f64, f64)],
    ranged: (std::ops::Range<f32>, std::ops::Range<f32>),
) -> Result<()> {
    let root = BitMapBackend::new(dest.to_str().unwrap(), (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .margin(32u32)
        .build_ranged(ranged.0, ranged.1)?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(
            data.iter()
                .map(|(x, y)| (*x as f32, *y as f32))
                .map(|point| Cross::new(point, 4, Into::<ShapeStyle>::into(&RED).filled())),
        )?
        .label(label)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    Ok(())
}

fn run() -> Result<()> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    pretty_env_logger::formatted_builder()
        .default_format()
        .filter_level(log::LevelFilter::Warn)
        .init();

    if args.flag_version {
        println!("{} - {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let label = args.flag_label;
    let title = args.flag_title;

    let file: std::path::PathBuf = args.arg_file.try_into()?;

    let mut data = Vec::with_capacity(128);
    let buffered = std::io::BufReader::with_capacity(4096, io::stdin());
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .flexible(true)
        .from_reader(buffered);

    let mut y_max = u32::min_value();
    let mut x_max = u32::min_value();
    let mut y_min = u32::max_value();
    let mut x_min = u32::max_value();

    let mut first_valid_record = false;
    for rec in rdr.records() {
        let rec = rec.map_err(|_e| format_err!("Failed to parse csv line"))?;

        rec.deserialize::<Record>(None)
            .map_err(|_e| format_err!("Failed to parse record"))
            .and_then(|record: Record| {
                first_valid_record = true;
                if record.idx > x_max {
                    x_max = record.idx;
                }
                if record.time_ms > y_max {
                    y_max = record.time_ms;
                }
                if record.idx < x_min {
                    x_min = record.idx;
                }
                if record.time_ms < y_min {
                    y_min = record.time_ms;
                }

                data.push(record.as_tuple());
                Ok::<(), failure::Error>(())
            })
            .or_else(|e| {
                if !first_valid_record {
                    println!("Found header {:?}", rec);
                    let columns = header::parse_header_columns(rec.as_slice());
                    println!("Found header columns {:?}", columns);
                    let info = header::parse_header_info(rec.as_slice());
                    println!("Found header info {:?}", info);
                    Ok::<(), failure::Error>(())
                } else {
                    Err(e)
                }
            })
            .unwrap_or_else(|e| {
                warn!("Failed to convert {:?}", e);
                ()
            });
    }

    if data.len() < 2 {
        bail!("Only one datapoint, go home");
    }
    let y_max: f32 = y_max as f32;
    let x_max: f32 = x_max as f32;
    let y_min: f32 = y_min as f32;
    let x_min: f32 = x_min as f32;

    plot(&file, &label, &title, &data, (x_min..x_max, y_min..y_max))?;

    Ok(())
}

quick_main!(run);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn full_check() {
        const X: &'static str = r#"
Pallet: "pallet-utility", Extrinsic: "as_sub", Steps: 30, Repeat: 11
A,I,time
77,0,2"#;
    }
}
