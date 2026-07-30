use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::Parser;
use rv32_emu_lib as emu;

use env_logger::{Builder, Env};
#[allow(unused_imports)]
use log::{LevelFilter, debug, error, info, trace, warn};

#[derive(Parser, Debug)]
#[command(version, about = "RISC-V 32bit Emulator", long_about = None)]
struct Cli {
    /// Path to the RISC-V executable file to run
    file: PathBuf, // Positional

    /// Override file type (default: auto-detect)
    ///
    /// Note that the HEX file type is expected to be a string of hex digits,
    /// and will have its bytes reversed in 4-byte chunks to match the little-endian representation
    /// (e.g., "0x20000537 0x0ff52583" will be stored as [37, 5, 0, 20, 83, 25, F5, F]).
    #[clap(short, long, verbatim_doc_comment)]
    filetype: Option<FileType>,

    /// Override the entry point in hexadecimal (default: auto-detect from ELF header, or start of file for HEX/BIN)
    #[clap(short, long)]
    entry_point: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum FileType {
    Elf,
    Hex,
    Bin,
}

fn main() -> anyhow::Result<()> {
    // Override the default log level to "info" if not set in the environment
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let entry_point: Option<u32> = cli.entry_point.map(|ep| {
        // Cannot replace undesired underscores in place
        let trim_ep = ep.trim().trim_start_matches("0x").replace("_", "");

        u32::from_str_radix(&trim_ep, 16).unwrap_or_else(|_| {
            error!("Error: Invalid entry point format. Please provide a valid hexadecimal value (e.g., 0x10000000).");
            std::process::exit(1);
        })
    });

    match entry_point {
        Some(ep) => info!("Using entry point: {:#08X}", ep),
        None => info!("No entry point specified, will auto-decide from file"),
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

    info!("Filetype: {:?}", filetype);

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

            info!("Loaded {0:#X} ({0}) bytes", bytecode.len());
            trace!("Bytecode: {:?}", bytecode);

            file = reader.into_inner(); // Reclaim ownership of the File
        }
        FileType::Bin => {
            memory = Box::new(unsafe { emu::MemmapMemory::new(&file, 1024 * 1024)? });

            info!("Memory-mapped {0:#X} ({0}) bytes", file.metadata()?.len());
        }
        FileType::Elf => {
            todo!()
        }
    };

    // Cpu accepts a mutable reference to any AddressSpace implementation,
    // so we can dereference the Box and borrow the underlying trait object.
    let mut cpu = emu::Cpu::new(&mut (*memory));

    cpu.reset(entry_point)?;
    cpu.run()?;

    println!("{:X?}", cpu);
    // From `man 1 flock`: a lock is automatically dropped when the file is closed...
    // ...but from [https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex]:
    // If a process terminates with a portion of a file locked or closes a file that has outstanding locks,
    // the locks are unlocked by the operating system. However, the time it takes for the operating system
    // to unlock these locks depends upon available system resources. Therefore, it is recommended
    // that your process explicitly unlock all files (...) when it terminates.

    file.unlock()
        .with_context(|| format!("Failed to unlock executable file: {}", cli.file.display()))?;
    Ok(())
}
