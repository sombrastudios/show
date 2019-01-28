extern crate regex;
extern crate term_size;
use regex::Regex;
use std::env::args;
use std::fs::*;

struct Config {
    show_header: bool,
    show_size: bool,
    show_creation: bool,
    show_modification: bool,
    print_all: bool,
    sort_by: Vec<SortOption>,
}

impl Config {
    fn new() -> Config {
        Config {
            show_header: false,
            show_size: false,
            show_creation: false,
            show_modification: false,
            print_all: false,
            sort_by: vec![SortOption::IsDirectory, SortOption::Name],
        }
    }

    fn from_args(args: std::env::Args) -> Config {
        args.skip(1)
            .fold(Config::new(), |acc, arg| match arg.as_str() {
                "-h" => acc.set_show_header(),
                "-s" => acc.set_show_size(),
                "-c" => acc.set_show_creation(),
                "-m" => acc.set_show_modification(),
                "-a" => acc.set_show_print_all(),
                sort if Regex::new("sort=(d|n|s|c|m|!)").unwrap().is_match(sort) => acc
                    .set_sort_by(sort.chars().skip(5).fold(
                        Vec::new(),
                        |mut acc: Vec<SortOption>, c: char| {
                            acc.push(match c {
                                'd' => SortOption::IsDirectory,
                                'n' => SortOption::Name,
                                's' => SortOption::Size,
                                'c' => SortOption::Creation,
                                'm' => SortOption::Modification,
                                '!' => SortOption::Reverse,
                                _ => panic!("Unexpected Option"),
                            });
                            return acc;
                        },
                    )),
                arg => {
                    eprintln!("Unknown paramter {}", arg);
                    std::process::exit(0);
                }
            })
    }

    fn set_sort_by(mut self, options: Vec<SortOption>) -> Config {
        self.sort_by = options;
        self
    }

    fn set_show_header(mut self) -> Config {
        self.show_header = true;
        self
    }
    fn set_show_size(mut self) -> Config {
        self.show_size = true;
        self
    }
    fn set_show_creation(mut self) -> Config {
        self.show_creation = true;
        self
    }
    fn set_show_modification(mut self) -> Config {
        self.show_modification = true;
        self
    }
    fn set_show_print_all(mut self) -> Config {
        self.print_all = true;
        self
    }
}

#[derive(Copy, Clone)]
enum SortOption {
    IsDirectory,
    Name,
    Size,
    Creation,
    Modification,
    Reverse,
}

impl SortOption {
    fn sort(self, mut vec: Vec<FileInfo>) -> Vec<FileInfo> {
        match self {
            SortOption::IsDirectory => vec.sort_by_key(|a| a.is_dir()),
            SortOption::Name => vec.sort_by_key(|a| a.name.to_lowercase()),
            SortOption::Size => vec.sort_by_key(|a| a.size),
            SortOption::Creation => vec.sort_by_key(|a| a.metadata.created().unwrap()),
            SortOption::Modification => vec.sort_by_key(|a| a.metadata.modified().unwrap()),
            SortOption::Reverse => vec.reverse(),
        };

        vec
    }
}

fn print_help() {
    println!("s: Show Files");

    println!("Show files in current directory\n");

    println!("Options:");
    println!("  -a All      Print all Files");
    std::process::exit(0);
}

fn main() {
    if args().len() == 2 {
        if args().nth(1).unwrap() == "--help" {
            print_help();
        }
    }

    let config = Config::from_args(args());

    let width = if let Some((width, _)) = term_size::dimensions() {
        width
    } else {
        140
    };

    let mut entries = read_dir(".")
        .unwrap()
        .map(|entry| FileInfo::from_dir_entry(entry.unwrap()))
        .filter(|entry| config.print_all || entry.name.chars().next().unwrap() != '.')
        .collect::<Vec<_>>();

    entries.sort_by(|entry, other| match (entry.is_dir(), other.is_dir()) {
        (true, true) => entry.name.to_lowercase().cmp(&other.name.to_lowercase()),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => entry.name.to_lowercase().cmp(&other.name.to_lowercase()),
    });

    let length_names = entries
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0);
    let length_suf = entries
        .iter()
        .map(|entry| entry.get_suffix_len())
        .max()
        .unwrap_or(0);
    let length_file_size = 6;

    entries
        .iter()
        .zip((0..).map(|n| n & 1 == 0))
        .for_each(|(info, is_even)| {
            let output = format!(
                "{}{} |{}{}|{}",
                info.name,
                spaces(length_names - info.name.len()),
                info.suffix.as_ref().unwrap_or(&"".to_string()),
                spaces(length_suf - info.get_suffix_len()),
                readable_size(info.size, length_file_size)
            );

            let output = format!("{}{}", output, spaces(width - output.len()));

            let colored = match (info.is_dir(), is_even) {
                (true, true) => set_background(output, 244),
                (true, false) => set_background(output, 243),
                (false, true) => set_background(output, 140),
                (false, false) => set_background(output, 146),
            };

            println!("{}", colored);
        });
}

fn set_background(s: String, value: u8) -> String {
    format!("\x1B[48;5;{}m{}\x1B[0m", value, s)
}

fn readable_size(size: u64, length: usize) -> String {
    let output = match size {
        gb if size >= 1024 * 1024 * 1024 => {
            format!("{:.2}GB", gb as f64 / 1024.0 * 1024.0 * 1024.0)
        }
        mb if size >= 1024 * 1024 => format!("{:.2}MB", mb as f64 / 1024.0 * 1024.0),
        kb if size >= 1024 => format!("{}KB", kb / 1024),
        b => format!("{}B", b),
    };

    format!("{}{}", spaces(length - output.len()), output)
}

#[derive(Debug)]
struct FileInfo {
    name: String,
    suffix: Option<String>,
    size: u64,
    metadata: Metadata,
}

impl FileInfo {
    fn from_dir_entry(entry: DirEntry) -> FileInfo {
        if let Ok(metadata) = entry.metadata() {
            // Don't attempt to take suffix, if file is a directory
            let (name, suffix) = if metadata.is_dir() {
                (entry.file_name().into_string().unwrap(), None)
            } else {
                split_suffix(entry.file_name().into_string().unwrap())
            };

            let info = FileInfo {
                name,
                suffix,
                size: metadata.len(),
                metadata: metadata,
            };

            return info;
        }

        panic!("Failed to retrieve metadata");
    }

    fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    fn get_suffix_len(&self) -> usize {
        if let Some(suffix) = &self.suffix {
            return suffix.len();
        }

        0
    }
}

fn split_suffix(name: String) -> (String, Option<String>) {
    if let Some(pos) = name.rfind('.') {
        if pos > 0 {
            let (name, suffix) = name.split_at(pos);

            return (name.to_string(), Some(suffix[1..].to_string()));
        }
    }

    (name, None)
}

fn spaces(n: usize) -> String {
    (0..n).map(|_| ' ').collect()
}
