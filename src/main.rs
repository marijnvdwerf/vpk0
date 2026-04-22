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
        /// Reference VPK0 file to verify against
        #[arg(long)]
        verify: Option<PathBuf>,
    },
    /// Decompress a VPK0 file
    #[command(alias = "d")]
    Decompress {
        /// Input file (compressed VPK0)
        input: PathBuf,
        /// Output file (decompressed)
        output: PathBuf,
    },
    /// Show VPK0 header info
    Info {
        /// Input VPK0 file
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Compress {
            input,
            output,
            verify,
        } => compress(&input, &output, verify.as_deref()),
        Command::Decompress { input, output } => decompress(&input, &output),
        Command::Info { input } => info(&input),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn compress(
    input: &PathBuf,
    output: &PathBuf,
    verify: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read(input)?;

    let compressed = vpk0::Encoder::for_bytes(&raw)
        .two_sample()
        .lzss_backend(vpk0::LzssBackend::Snap)
        .encode_to_vec()?;

    std::fs::write(output, &compressed)?;

    eprintln!(
        "{}: {} -> {} bytes",
        input.display(),
        raw.len(),
        compressed.len()
    );

    if let Some(ref_path) = verify {
        let reference = std::fs::read(ref_path)?;
        if compressed != reference {
            let first_diff = compressed
                .iter()
                .zip(reference.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(compressed.len().min(reference.len()));
            eprintln!(
                "verification FAILED: sizes {}/{}, first diff at 0x{:X}",
                compressed.len(),
                reference.len(),
                first_diff
            );
            process::exit(1);
        }
        eprintln!("verification OK");
    }

    Ok(())
}

fn decompress(
    input: &PathBuf,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let compressed = std::fs::read(input)?;
    let decompressed = vpk0::decode_bytes(&compressed)?;
    std::fs::write(output, &decompressed)?;

    eprintln!(
        "{}: {} -> {} bytes",
        input.display(),
        compressed.len(),
        decompressed.len()
    );

    Ok(())
}

fn info(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(input)?;
    let (header, trees) = vpk0::vpk_info(std::io::Cursor::new(&data))?;

    println!("size:    {} bytes (decompressed)", header.size);
    println!("method:  {:?}", header.method);
    println!("offsets: {}", trees.offsets);
    println!("lengths: {}", trees.lengths);

    Ok(())
}
