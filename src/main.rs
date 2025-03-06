use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use clap::{Parser, Subcommand};
use colored::Colorize;
use hotwatch::notify::Event;
use hotwatch::{
    blocking::{Flow, Hotwatch},
    EventKind,
};
use miette::{bail, IntoDiagnostic, Result};

use lace::features::Features;
use lace::{reset_state, with_symbol_table, Air, AsmLine, RunState, StaticSource};

/// Lace is a complete & convenient assembler toolchain for the LC3 assembly language.
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Quickly provide a `.asm` file to run
    path: Option<PathBuf>,
    #[command(flatten)]
    run_options: RunOptions,
}

#[derive(Subcommand)]
enum Command {
    /// Run text `.asm` or binary `.lc3` file directly and output to terminal
    Run {
        /// .asm file to run
        name: PathBuf,
        #[command(flatten)]
        run_options: RunOptions,
    },
    /// Create binary `.lc3` file to run later or view compiled data
    Compile {
        /// `.asm` file to compile
        name: PathBuf,
        /// Destination to output .lc3 file
        dest: Option<PathBuf>,
        #[command(flatten)]
        run_options: RunOptions,
    },
    /// Check a `.asm` file without running or outputting binary
    Check {
        /// File to check
        name: PathBuf,
    },
    /// Remove compilation artifacts for specified source
    Clean {
        /// `.asm` file to try remove artifacts for
        name: PathBuf,
    },
    /// Place a watch on a `.asm` file to receive constant assembler updates
    Watch {
        /// `.asm` file to watch
        name: PathBuf,
    },
    /// Format `.asm` file to adhere to recommended style
    Fmt {
        /// `.asm` file to format
        name: PathBuf,
    },
}

#[derive(clap::Args)]
struct RunOptions {
    /// Feature flags to enable non-standard extensions to the LC3 specification
    ///
    /// Available flags: 'stack'
    #[arg(
        short,
        long,
        value_parser = clap::value_parser!(Features),
        default_value_t = Default::default(),
    )]
    features: Features,
}

fn main() -> miette::Result<()> {
    use MsgColor::*;
    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Command::Run {
                name,
                run_options: RunOptions { features },
            } => {
                lace::features::init(features);
                run(&name)?;
                Ok(())
            }
            Command::Compile {
                name,
                dest,
                run_options: RunOptions { features },
            } => {
                lace::features::init(features);
                file_message(Green, "Assembling", &name);
                let contents = StaticSource::new(fs::read_to_string(&name).into_diagnostic()?);
                let air = assemble(&contents)?;

                let mut files = OutFiles::from(dest, name);
                let orig = air.orig().unwrap_or(0x3000u16);

                // Deal with .orig
                files.write_header(orig)?;

                // Write lines
                for stmt in air {
                    files.write_line(&stmt).unwrap();
                }

                // Symbol table
                files.write_other(orig)?;

                message(Green, "Finished", "emit binary");
                // TODO(feat): Print file name after save
                // file_message(Green, "Saved", files.code_name());
                message(Green, "Saved", "(some files)");
                Ok(())
            }
            Command::Check { name } => {
                file_message(Green, "Checking", &name);
                let contents = StaticSource::new(fs::read_to_string(&name).into_diagnostic()?);
                let _ = assemble(&contents)?;
                message(Green, "Success", "no errors found!");
                Ok(())
            }
            Command::Clean { name: _ } => todo!("There are no debug files implemented to clean!"),
            Command::Watch { name } => {
                if !name.exists() {
                    bail!("File does not exist. Exiting...")
                }
                // Vim breaks if watching a single file
                let folder_path = match name.parent() {
                    Some(pth) if pth.is_dir() => pth.to_path_buf(),
                    _ => Path::new(".").to_path_buf(),
                };

                // Clear screen and move cursor to top left
                print!("\x1B[2J\x1B[2;1H");
                file_message(Green, "Watching", &name);
                message(Cyan, "Help", "press CTRL+C to exit");

                let mut watcher = Hotwatch::new_with_custom_delay(Duration::from_millis(500))
                    .into_diagnostic()?;

                watcher
                    .watch(folder_path, move |event: Event| match event.kind {
                        // Watch remove for vim changes
                        EventKind::Modify(_) | EventKind::Remove(_) => {
                            // Clear screen
                            print!("\x1B[2J\x1B[2;1H");
                            file_message(Green, "Watching", &name);
                            message(Green, "Re-checking", "file change detected");
                            message(Cyan, "Help", "press CTRL+C to exit");

                            // Now we are developing software (makes reruns more obvious)
                            sleep(Duration::from_millis(50));

                            let mut contents = StaticSource::new(match fs::read_to_string(&name) {
                                Ok(cts) => cts,
                                Err(e) => {
                                    eprintln!("{e}. Exiting...");
                                    std::process::exit(1)
                                }
                            });
                            let _ = match assemble(&contents) {
                                Ok(_) => {
                                    message(Green, "Success", "no errors found!");
                                }
                                Err(e) => {
                                    println!("\n{:?}", e);
                                }
                            };

                            reset_state();
                            // To avoid leaking memory
                            contents.reclaim();
                            Flow::Continue
                        }
                        _ => Flow::Continue,
                    })
                    .into_diagnostic()?;
                watcher.run();
                Ok(())
            }
            Command::Fmt { name: _ } => todo!("Formatting is not currently implemented"),
        }
    } else {
        if let Some(path) = args.path {
            lace::features::init(args.run_options.features);
            run(&path)?;
            Ok(())
        } else {
            println!("\n~ lace v{VERSION} - Copyright (c) 2024 Artemis Rosman ~");
            println!("{}", LOGO.truecolor(255, 183, 197).bold());
            println!("{SHORT_INFO}");
            std::process::exit(0);
        }
    }
}

#[allow(unused)]
enum MsgColor {
    Green,
    Cyan,
    Red,
}

fn file_message(color: MsgColor, left: &str, right: &PathBuf) {
    let right = format!("target {}", right.to_str().unwrap());
    message(color, left, &right);
}

fn message<S>(color: MsgColor, left: S, right: S)
where
    S: Colorize + std::fmt::Display,
{
    let left = match color {
        MsgColor::Green => left.green(),
        MsgColor::Cyan => left.cyan(),
        MsgColor::Red => left.red(),
    };
    println!("{left:>12} {right}");
}

fn run(name: &PathBuf) -> Result<()> {
    file_message(MsgColor::Green, "Assembling", &name);
    let mut program = if let Some(ext) = name.extension() {
        match ext.to_str().unwrap() {
            "lc3" | "obj" => {
                // Read to byte buffer
                let mut file = File::open(&name).into_diagnostic()?;
                let f_size = file.metadata().unwrap().len();
                let mut buffer = Vec::with_capacity(f_size as usize);
                file.read_to_end(&mut buffer).into_diagnostic()?;

                if buffer.len() % 2 != 0 {
                    bail!("File is not aligned to 16 bits")
                }

                let u16_buf: Vec<u16> = buffer
                    .chunks_exact(2)
                    .map(|word| u16::from_be_bytes([word[0], word[1]]))
                    .collect();
                RunState::from_raw(&u16_buf)?
            }
            "asm" => {
                let contents = StaticSource::new(fs::read_to_string(&name).into_diagnostic()?);
                let air = assemble(&contents)?;
                RunState::try_from(air)?
            }
            _ => {
                bail!("File has unknown extension. Exiting...")
            }
        }
    } else {
        bail!("File has no extension. Exiting...");
    };

    message(MsgColor::Green, "Running", "emitted binary");
    program.run();

    file_message(MsgColor::Green, "Completed", &name);
    Ok(())
}

/// Return assembly intermediate representation of source file for further processing
fn assemble(contents: &StaticSource) -> Result<Air> {
    let parser = lace::AsmParser::new(contents.src())?;
    let mut air = parser.parse()?;
    air.backpatch()?;
    Ok(air)
}

// TODO(doc)
struct OutFiles {
    code: File,
    hex: File,
    bin: File,
    sym: File,
}

impl OutFiles {
    const FILE_COUNT: u32 = 4;

    // TODO(doc)
    pub fn from(dest: Option<PathBuf>, mut path: PathBuf) -> Self {
        let mut path = dest.unwrap_or_else(|| {
            path.set_extension("lc3");
            path
        });
        let code = File::create(&path).unwrap();

        path.set_extension("hex");
        let hex = File::create(&path).unwrap();

        path.set_extension("bin");
        let bin = File::create(&path).unwrap();

        path.set_extension("sym");
        let sym = File::create(&path).unwrap();

        Self {
            code,
            hex,
            bin,
            sym,
        }
    }

    // TODO(doc)
    pub fn write_header(&mut self, orig: u16) -> miette::Result<()> {
        let orig = orig;

        self.code.write(&orig.to_be_bytes()).unwrap();
        writeln!(self.hex, "{:04X}", orig).unwrap();
        writeln!(self.bin, "{:016b}", orig).unwrap();

        Ok(())
    }

    // TODO(doc)
    pub fn write_line(&mut self, stmt: &AsmLine) -> miette::Result<()> {
        let word = stmt.emit()?;

        self.code.write(&word.to_be_bytes()).unwrap();
        writeln!(self.hex, "{:04X}", word).unwrap();
        writeln!(self.bin, "{:016b}", word).unwrap();

        Ok(())
    }

    // TODO(doc)
    pub fn write_other(&mut self, orig: u16) -> miette::Result<()> {
        with_symbol_table(|sym| {
            for (symbol, addr) in sym {
                writeln!(self.sym, "{:-74} x{:04X}", symbol, *addr + orig - 1).unwrap();
            }
            Ok(())
        })
    }
}

const LOGO: &str = r#"
      ..                                  
x .d88"                                   
 5888R                                    
 '888R         u           .        .u    
  888R      us888u.   .udR88N    ud8888.  
  888R   .@88 "8888" <888'888k :888'8888. 
  888R   9888  9888  9888 'Y"  d888 '88%" 
  888R   9888  9888  9888      8888.+"    
  888R   9888  9888  9888      8888L      
 .888B . 9888  9888  ?8888u../ '8888c. .+ 
 ^*888%  "888*""888"  "8888P'   "88888%   
   "%     ^Y"   ^Y'     "P'       "YP'"#;

const SHORT_INFO: &str = r"
Welcome to lace (from LAIS - LC3 Assembler & Interpreter System),
an all-in-one toolchain for working with LC3 assembly code.
Please use `-h` or `--help` to access the usage instructions and documentation.
";

const VERSION: &str = env!("CARGO_PKG_VERSION");
