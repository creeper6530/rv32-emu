use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::Parser;
use rv32_emu_lib as emu;

#[allow(unused_imports)]
use log::{LevelFilter, debug, error, info, trace, warn};

// --------------------------------------------------

// Levels ordered from least to most verbose.
const LOG_LEVELS_ARRAY: [LevelFilter; 6] = [
    LevelFilter::Off,
    LevelFilter::Error,
    LevelFilter::Warn,
    LevelFilter::Info,
    LevelFilter::Debug,
    LevelFilter::Trace,
];
const DEFAULT_LOG_LEVEL_INDEX: usize = 2; // Index of LevelFilter::Warn in LOG_LEVELS_ARRAY
const DEFAULT_LOG_LEVEL: LevelFilter = LOG_LEVELS_ARRAY[DEFAULT_LOG_LEVEL_INDEX]; // Warn

// --------------------------------------------------

fn hex_parser(s: &str) -> Result<u32, core::num::ParseIntError> {
    let s = s.trim_start_matches("0x").replace("_", "");
    u32::from_str_radix(&s, 16)
}

#[derive(Parser, Debug)]
#[command(version, about = "RISC-V 32bit Emulator", long_about = None)]
#[command(group(
    clap::ArgGroup::new("log_verbosity")
        .args(["log_level", "verbose", "quiet"])
        .multiple(false)
))]
struct Cli {
    /// Path to the RISC-V executable file to run
    file: PathBuf, // Positional

    /// Override file type
    ///
    /// Note that the HEX file type is expected to be a string of hex digits,
    /// and will have its bytes reversed in 4-byte chunks to match the little-endian representation
    /// (e.g., "0x20000537 0x0ff52583" will be stored as [37, 5, 0, 20, 83, 25, F5, F]).
    ///
    /// [default: auto-detect]
    #[clap(short, long, verbatim_doc_comment)]
    filetype: Option<FileType>,

    /// Override the entry point in hexadecimal
    ///
    /// Prefix "0x" and underscores are allowed (e.g., 0x1000_0000).
    ///
    /// [default: auto-detect from ELF header, or start of file for HEX/BIN]
    #[clap(short, long, value_parser = hex_parser)]
    entry_point: Option<u32>,

    /// Set the log level
    ///
    /// Mutually exclusive with --verbose and --quiet
    #[clap(short, long, default_value_t = DEFAULT_LOG_LEVEL)]
    log_level: LevelFilter,

    /// Increase verbosity (can be used multiple times)
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Decrease verbosity (can be used multiple times)
    #[clap(short, long, action = clap::ArgAction::Count)]
    quiet: u8,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum FileType {
    Elf,
    Hex,
    Bin,
}

// --------------------------------------------------

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Will not read environment variables, only CLI arguments
    let mut builder = env_logger::Builder::new();
    match (cli.verbose, cli.quiet) {
        (0, 0) => {
            builder.filter_level(cli.log_level);
        }
        (v, 0) if v > 0 => {
            // Make more verbose by moving towards Trace
            let log_level_index = std::cmp::min(
                5, // Index of LevelFilter::Trace in LOG_LEVELS_ARRAY
                DEFAULT_LOG_LEVEL_INDEX + (v as usize),
            );
            builder.filter_level(LOG_LEVELS_ARRAY[log_level_index]);
        }
        (0, q) if q > 0 => {
            // Make quieter by moving towards Off
            let log_level_index = DEFAULT_LOG_LEVEL_INDEX.saturating_sub(q as usize);
            builder.filter_level(LOG_LEVELS_ARRAY[log_level_index]);
        }
        _ => unreachable!("Cannot have both verbose and quiet flags set"),
    }
    builder.init();

    match cli.entry_point {
        Some(ep) => debug!("Using entry point: {:#08X}", ep),
        None => debug!("No entry point specified, will auto-decide from file"),
    }

    let filetype = cli.filetype.unwrap_or_else(|| {
        cli.file.extension()
            .and_then(|ext| match ext.to_str() {
                Some("elf") => Some(FileType::Elf),
                Some("hex") => Some(FileType::Hex),
                Some("bin") => Some(FileType::Bin),
                _ => None,
            })
            .unwrap_or_else(|| {
                error!("Error: Could not determine file type from extension. Please specify the file type using --filetype.");
                std::process::exit(1);
            })
    });

    debug!("Filetype: {:?}", filetype);

    // Bytecode to be loaded into the emulator's memory.
    let mut bytecode: Vec<u8>;

    let mut file = File::open(cli.file.as_path())
        .with_context(|| format!("Failed to open executable file: {}", cli.file.display()))?;
    file.lock_shared().with_context(|| {
        format!(
            "Failed to acquire shared lock on executable file: {}",
            cli.file.display()
        )
    })?;

    let mut memory: Box<dyn emu::AddressSpace>;

    match filetype {
        FileType::Hex => {
            let mut reader = std::io::BufReader::new(file);

            let mut hex_string = String::with_capacity(reader.get_ref().metadata()?.len() as usize);

            reader.read_to_string(&mut hex_string)?;
            bytecode = hex::decode(
                hex_string
                    .replace("0x", "")
                    .chars()
                    .filter(|c| c.is_ascii_hexdigit())
                    .collect::<String>(),
            )?;

            bytecode.chunks_mut(4).for_each(|chunk| {
                chunk.reverse(); // Reverse each 4-byte chunk for little-endian representation
            });

            memory = Box::new(emu::BoxedMemory::<1024, 1024>::new(Some(&bytecode))?);

            debug!("Loaded {0:#X} ({0}) bytes", bytecode.len());
            trace!("Bytecode: {:?}", bytecode);

            file = reader.into_inner(); // Reclaim ownership of the File
        }
        FileType::Bin => {
            memory = Box::new(unsafe { emu::MemmapMemory::new(&file, 1024 * 1024)? });

            debug!("Memory-mapped {0:#X} ({0}) bytes", file.metadata()?.len());
        }
        FileType::Elf => {
            todo!()
        }
    };

    // Cpu accepts a mutable reference to any AddressSpace implementation,
    // so we dereference the Box and borrow the underlying trait object.
    let mut cpu = emu::Cpu::new(&mut (*memory));

    cpu.reset(cli.entry_point)?;
    let (a0, _a1) = cpu.run()?;
    info!("Exit code: {:#}", a0);

    debug!("{:X?}", cpu);

    // Both Unix and Windows unlock the file when it is closed,
    // but Windows recommends explicitly unlocking it first,
    // because it could take a while for the OS to release the lock after us.
    file.unlock()
        .with_context(|| format!("Failed to unlock executable file: {}", cli.file.display()))?;
    Ok(())
}
