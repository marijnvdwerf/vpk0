use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "vpk0", about = "VPK0 compression tool for N64 ROMs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compress raw data into VPK0 format
    #[command(alias = "c")]
    Compress {
        /// Input file (uncompressed)
        input: PathBuf,
        /// Output file (compressed VPK0)
        output: PathBuf,
    },
    /// Decompress a VPK0 file
    #[command(alias = "d")]
    Decompress {
        /// Input file (compressed VPK0)
        input: PathBuf,
        /// Output file (decompressed)
        output: PathBuf,
    },
}

fn main() {
    let result = match Cli::parse().command {
        Command::Compress { input, output } => compress(&input, &output),
        Command::Decompress { input, output } => decompress(&input, &output),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn compress(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read(input)?;
    let compressed = vpk0::Encoder::for_bytes(&raw)
        .two_sample()
        .lzss_backend(vpk0::LzssBackend::Snap)
        .encode_to_vec()?;
    std::fs::write(output, &compressed)?;
    Ok(())
}

fn decompress(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let compressed = std::fs::read(input)?;
    let decompressed = vpk0::decode_bytes(&compressed)?;
    std::fs::write(output, &decompressed)?;
    Ok(())
}
