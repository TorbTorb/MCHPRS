use super::{Plot, PlotWorld, PLOT_WIDTH};
use crate::world::storage::ChunkData;
use crate::world::TickEntry;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

static PLOT_MAGIC: &[u8; 8] = b"\x86MCHPRS\x00";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tps {
    Limited(u32),
    Unlimited,
}

impl Tps {
    pub fn sleep_time(self) -> Duration {
        match self {
            Tps::Limited(tps) => {
                if tps > 10 {
                    Duration::from_micros(1_000_000 / tps as u64)
                } else {
                    Duration::from_millis(50)
                }
            }
            Tps::Unlimited => Duration::ZERO,
        }
    }

    pub fn from_data(data: u32) -> Tps {
        match data {
            u32::MAX => Tps::Unlimited,
            tps => Tps::Limited(tps),
        }
    }

    pub fn to_data(self) -> u32 {
        match self {
            Tps::Unlimited => u32::MAX,
            Tps::Limited(tps) => tps,
        }
    }
}

impl fmt::Display for Tps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tps::Limited(tps) => write!(f, "{}", tps),
            Tps::Unlimited => write!(f, "unlimited"),
        }
    }
}

static EMPTY_PLOT: LazyLock<PlotData> = LazyLock::new(|| {
    let template_path = Path::new("./world/plots/pTEMPLATE");
    if template_path.exists() {
        PlotData::read_from_file(template_path).expect("failed to read template plot")
    } else {
        let mut chunks = Vec::new();
        for chunk_x in 0..PLOT_WIDTH {
            for chunk_z in 0..PLOT_WIDTH {
                chunks.push(Plot::generate_chunk(8, chunk_x, chunk_z));
            }
        }
        let mut world = PlotWorld {
            x: 0,
            z: 0,
            chunks,
            to_be_ticked: Vec::new(),
            packet_senders: Vec::new(),
        };
        let chunk_data: Vec<ChunkData> = world.chunks.iter_mut().map(|c| c.save()).collect();
        PlotData {
            tps: Tps::Limited(10).to_data(),
            show_redstone: true,
            chunk_data,
            pending_ticks: Vec::new(),
        }
    }
});

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlotData {
    pub tps: u32,
    pub show_redstone: bool,
    pub chunk_data: Vec<ChunkData>,
    pub pending_ticks: Vec<TickEntry>,
}

impl PlotData {
    pub fn read_from_file(path: impl AsRef<Path>) -> Result<PlotData> {
        let mut file = File::open(path)?;

        let mut magic = [0; 8];
        file.read_exact(&mut magic)?;
        if &magic != PLOT_MAGIC {
            // TODO: convert plot data
            bail!("plot data header incorrect");
        }

        Ok(bincode::deserialize_from(file)?)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut file = OpenOptions::new().write(true).create(true).open(path)?;
        file.write_all(PLOT_MAGIC)?;
        bincode::serialize_into(&file, self)?;
        file.sync_data()?;
        Ok(())
    }
}

impl Default for PlotData {
    fn default() -> PlotData {
        EMPTY_PLOT.clone()
    }
}
