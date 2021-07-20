use std::{
    fmt::Display,
    fs::{self, FileType},
    io,
    path::{Path, PathBuf},
};

use imprint::Imprint;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug, StructOpt)]
struct Opts {
    source: String,
    destination: String,

    /// copy hidden files (starting with .dot)
    #[structopt(short = "h", long = "hidden")]
    include_hidden_files: bool,
}

impl Opts {
    fn destination(&self) -> &Path {
        self.destination.as_ref()
    }
}

struct Object {
    file_type: FileType,
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

impl Object {
    fn new(base_path: impl AsRef<Path>, entry: DirEntry) -> io::Result<Self> {
        let absolute_path = entry.path().to_owned();
        let relative_path = absolute_path.strip_prefix(base_path).unwrap().to_owned();
        Ok(Object {
            file_type: entry.file_type(),
            absolute_path,
            relative_path,
        })
    }

    fn copy_to(&self, destination: &Path) -> io::Result<()> {
        Ok({
            fs::copy(&self.absolute_path, destination)?;
        })
    }
}

#[derive(Clone, Debug)]
struct BadCopy {
    source: PathBuf,
    destination: PathBuf,
}

impl BadCopy {
    fn new(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
        }
    }
}

impl Display for BadCopy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "bad copy:\n  source: {}\n  destination: {}",
            self.source.display(),
            self.destination.display()
        )
    }
}

impl std::error::Error for BadCopy {}

fn main() {
    let opts = Opts::from_args();
    if let Err(e) = run(&opts) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(opts: &Opts) -> io::Result<()> {
    let source_entries = WalkDir::new(&opts.source).into_iter().filter_map(|entry| {
        let entry = entry.ok()?;
        Object::new(&opts.source, entry).ok()
    });

    for object in source_entries {
        let destination = opts.destination().join(&object.relative_path);

        if object.file_type.is_dir() {
            if !destination.exists() {
                fs::create_dir_all(destination)?;
            }
            continue;
        }

        if object.file_type.is_file() {
            let source_imprint = Imprint::new(&object.absolute_path)?;
            if destination.exists() && source_imprint == Imprint::new(&destination)? {
                continue;
            }

            object.copy_to(&destination)?;
            let destination_imprint = Imprint::new(&destination)?;
            if source_imprint != destination_imprint {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    BadCopy::new(object.absolute_path, destination),
                ));
            }
        }
    }

    Ok(())
}
