use std::convert::{TryFrom, TryInto};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    #[structopt(parse(from_os_str))]
    dir: PathBuf,
}

struct FileTree {
    _root_path: PathBuf,
    root_entry: Entry,
}

struct Entry {
    name: OsString,
    size: u64,
    data: EntryData,
}

enum EntryData {
    File,
    Symlink(PathBuf),
    Directory(Vec<Entry>),
    Unknown,
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn fmt_entry(
            entry: &Entry,
            f: &mut fmt::Formatter,
            depth: usize,
            is_last: bool,
        ) -> fmt::Result {
            for _ in 0..depth {
                write!(f, " \u{2502}")?;
            }
            if depth > 0 {
                if is_last {
                    write!(f, " \u{2514}")?;
                } else {
                    write!(f, " \u{251c}")?;
                }
            }

            write!(f, "{}", &entry.name.to_string_lossy())?;

            match &entry.data {
                EntryData::File | EntryData::Symlink(..) | EntryData::Unknown => {}
                EntryData::Directory(children) => {
                    writeln!(f)?;

                    let mut iter = children.iter();
                    let mut next = iter.next();
                    while let Some(child) = next {
                        next = iter.next();
                        fmt_entry(child, f, depth + 1, next.is_none())?;
                    }
                }
            }
            writeln!(f)
        }

        fmt_entry(self, f, 0, true)
    }
}

fn get_file_tree(path: impl AsRef<Path>) -> io::Result<FileTree> {
    let path = path.as_ref().to_path_buf();

    let (children, size) = get_child_entries(&path)?;
    let name = path
        .file_name()
        .unwrap_or(AsRef::<OsStr>::as_ref(""))
        .to_os_string();
    let root_entry = Entry {
        name,
        size,
        data: EntryData::Directory(children),
    };

    Ok(FileTree {
        root_entry,
        _root_path: path,
    })
}

impl TryFrom<fs::DirEntry> for Entry {
    type Error = io::Error;

    fn try_from(dir_entry: fs::DirEntry) -> Result<Self, Self::Error> {
        let name = dir_entry.file_name();
        let file_type = dir_entry.file_type()?;

        Ok(if file_type.is_dir() {
            let (children, size) = get_child_entries(dir_entry.path())?;
            Entry {
                name,
                size,
                data: EntryData::Directory(children),
            }
        } else if file_type.is_symlink() {
            let link_to = fs::read_link(dir_entry.path())?;
            Entry {
                name,
                size: 0,
                data: EntryData::Symlink(link_to),
            }
        } else if file_type.is_file() {
            let metadata = dir_entry.metadata()?;
            Entry {
                name,
                size: metadata.len(),
                data: EntryData::File,
            }
        } else {
            Entry {
                name,
                size: 0,
                data: EntryData::Unknown,
            }
        })
    }
}

fn get_child_entries(path: impl AsRef<Path>) -> io::Result<(Vec<Entry>, u64)> {
    let dir_entries = fs::read_dir(path)?;

    let mut entries = vec![];
    let mut total_size = 0;
    for dir_entry in dir_entries {
        let entry: Entry = dir_entry?.try_into()?;

        total_size += entry.size;
        entries.push(entry);
    }

    Ok((entries, total_size))
}

#[paw::main]
fn main(options: Options) -> io::Result<()> {
    let file_tree = get_file_tree(&options.dir)?;

    println!("{:?}", &file_tree.root_entry);
    Ok(())
}
