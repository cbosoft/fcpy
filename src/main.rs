use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::fs;

use kdam::{Bar, BarExt};
use clap::Parser;


#[derive(Parser)]
struct Args {
    /// Enable to copy files recursively from a directory.
    #[arg(short, default_value_t=false)]
    pub recursive: bool,

    /// Enable verbose logging.
    #[arg(short, default_value_t=false)]
    pub verbose: bool,
    
    #[arg(required=true)]
    pub sources: Vec<PathBuf>,

    /// Destination name of file or, if multiple sources were selected, name of directory into
    /// which the sources will be copied.
    pub destination: PathBuf,
}

fn copy_one_file(progress_tx: Sender<usize>, source: PathBuf, destination: PathBuf) {
    let mut bytes = fs::metadata(source.clone()).unwrap().len() as usize;
    let block = 500_000usize.max(bytes / 10usize);
    let mut ifile = fs::OpenOptions::new().read(true).open(source).unwrap();
    let mut ofile = fs::OpenOptions::new().write(true).create(true).truncate(true).open(destination).unwrap();
    while bytes > 0usize {
        let n = bytes.min(block);
        bytes -= n;
        let mut buf: Vec<u8> = Vec::new();
        for _ in 0..n {
            buf.push(0u8);
        }
        ifile.read(buf.as_mut_slice()).unwrap();
        ofile.write(buf.as_slice()).unwrap();
        progress_tx.send(n).unwrap();
    }
}

fn main() {
    let mut args = Args::parse();
    if args.recursive {
        let mut all_sources = Vec::new();
        for source in args.sources {
            let md = fs::metadata(source.clone()).expect("file/dir not found");
            if md.is_dir() {
                // recursively add contents to sources
                todo!();
            }
            else {
                all_sources.push(source);
            }
        }
        args.sources = all_sources;
    }

    let (tx, rx) = channel();
    let mut handles = Vec::new();

    let total_bytes: u64 = args.sources.iter().map(|p|{ fs::metadata(p).unwrap().len()}).sum();
    let total_bytes = total_bytes as usize;

    if args.sources.len() > 1 {
        // multiple sources, copy into directory
        for source in args.sources {
            let base = source.file_name().unwrap();
            let dest = args.destination.clone().join(base);
            let tx = tx.clone();
            let handle = thread::spawn(move || { copy_one_file(tx, source, dest); });
            handles.push(handle);
        }
    }
    else {
        // single file
        let src = args.sources.first().unwrap().clone();
        let dest = args.destination;
        let handle = thread::spawn(move || { copy_one_file(tx, src, dest); });
        handles.push(handle);
    }

    let mut bar = Bar::new(total_bytes);
    loop {
        match rx.recv() {
            Ok(bytes) => {
                bar.update(bytes).unwrap();
            }
            Err(_) => {
                break;
            }
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
