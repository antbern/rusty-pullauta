use log::debug;
use log::info;
use pullauta::config::Config;
use pullauta::io::fs::memory::MemoryFileSystem;
use pullauta::io::fs::FileSystem;
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::{thread, time};

fn main() {
    // setup and configure logging, default to INFO when RUST_LOG is not set
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            let ts = buf.timestamp_seconds();
            let level_style = buf.default_level_style(record.level());

            writeln!(
                buf,
                "[{} {:?} {level_style}{}{level_style:#} {}] {}",
                ts,
                std::thread::current().id(),
                record.level(),
                record.module_path().unwrap_or(""),
                record.args()
            )
        })
        .init();

    let mut thread: String = String::new();

    let config =
        Arc::new(Config::load_or_create_default().expect("Could not open or create config file"));

    let fs = pullauta::io::fs::local::LocalFileSystem;

    let mut args: Vec<String> = env::args().collect();

    args.remove(0); // program name

    if !args.is_empty() && args[0].trim().parse::<usize>().is_ok() {
        thread = args.remove(0);
    }

    let command = if !args.is_empty() {
        args.remove(0)
    } else {
        String::new()
    };

    let command_lowercase = command.to_lowercase();

    if command.is_empty()
        || command_lowercase.ends_with(".las")
        || command_lowercase.ends_with(".laz")
        || command_lowercase.ends_with(".xyz")
        || command_lowercase.ends_with(".xyz.bin")
    {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        println!(
            "Karttapullautin v{}\nThere is no warranty. Use it at your own risk!\n",
            VERSION
        );
    }

    let batch: bool = config.batch;

    let tmpfolder = PathBuf::from(format!("temp{}", thread));
    fs::create_dir_all(&tmpfolder).expect("Could not create tmp folder");

    let pnorthlinesangle = config.pnorthlinesangle;
    let pnorthlineswidth = config.pnorthlineswidth;

    if command.is_empty() && fs.exists(tmpfolder.join("vegetation.png")) && !batch {
        info!("Rendering png map with depressions");
        pullauta::render::render(
            &fs,
            &config,
            &thread,
            &tmpfolder,
            pnorthlinesangle,
            pnorthlineswidth,
            false,
        )
        .unwrap();
        info!("Rendering png map without depressions");
        pullauta::render::render(
            &fs,
            &config,
            &thread,
            &tmpfolder,
            pnorthlinesangle,
            pnorthlineswidth,
            true,
        )
        .unwrap();
        info!("\nAll done!");
        return;
    }

    if command.is_empty() && !batch {
        println!("USAGE:\npullauta [parameter 1] [parameter 2] [parameter 3] ... [parameter n]\nSee README.MD for more details");
        return;
    }

    if command == "cliffgeneralize" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "ground" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "ground2" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "groundfix" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "profile" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "makecliffsold" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "makeheight" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "xyzfixer" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "vege" {
        info!("Not implemented in this version, use the perl version");
        return;
    }

    if command == "internal2xyz" {
        if args.len() < 2 {
            info!("USAGE: internal2xyz [input file] [output file]");
            return;
        }

        let input = &args[0];
        let output = &args[1];
        pullauta::io::internal2xyz(&fs, input, output).unwrap();
        return;
    }

    if command == "blocks" {
        pullauta::blocks::blocks(&fs, &tmpfolder).unwrap();
        return;
    }

    if command == "dotknolls" {
        pullauta::knolls::dotknolls(&fs, &config, &tmpfolder).unwrap();
        return;
    }

    if command == "dxfmerge" || command == "merge" {
        pullauta::merge::dxfmerge(&fs, &config).unwrap();
        if command == "merge" {
            let mut scale = 1.0;
            if !args.is_empty() {
                scale = args[0].parse::<f64>().unwrap();
            }
            pullauta::merge::pngmergevege(&fs, &config, scale).unwrap();
        }
        return;
    }

    if command == "knolldetector" {
        pullauta::knolls::knolldetector(&fs, &config, &tmpfolder).unwrap();
        return;
    }

    if command == "makecliffs" {
        pullauta::cliffs::makecliffs(&fs, &config, &tmpfolder).unwrap();
        return;
    }

    if command == "makevege" {
        pullauta::vegetation::makevege(&fs, &config, &tmpfolder).unwrap();
    }

    if command == "pngmerge" || command == "pngmergedepr" {
        let mut scale = 4.0;
        if !args.is_empty() {
            scale = args[0].parse::<f64>().unwrap();
        }
        pullauta::merge::pngmerge(&fs, &config, scale, command == "pngmergedepr").unwrap();
        return;
    }

    if command == "pngmergevege" {
        let mut scale = 1.0;
        if !args.is_empty() {
            scale = args[0].parse::<f64>().unwrap();
        }
        pullauta::merge::pngmergevege(&fs, &config, scale).unwrap();
        return;
    }

    if command == "polylinedxfcrop" {
        let dxffilein = Path::new(&args[0]);
        let dxffileout = Path::new(&args[1]);
        let minx = args[2].parse::<f64>().unwrap();
        let miny = args[3].parse::<f64>().unwrap();
        let maxx = args[4].parse::<f64>().unwrap();
        let maxy = args[5].parse::<f64>().unwrap();
        pullauta::crop::polylinedxfcrop(&fs, dxffilein, dxffileout, minx, miny, maxx, maxy)
            .unwrap();
        return;
    }

    if command == "pointdxfcrop" {
        let dxffilein = Path::new(&args[0]);
        let dxffileout = Path::new(&args[1]);
        let minx = args[2].parse::<f64>().unwrap();
        let miny = args[3].parse::<f64>().unwrap();
        let maxx = args[4].parse::<f64>().unwrap();
        let maxy = args[5].parse::<f64>().unwrap();
        pullauta::crop::pointdxfcrop(&fs, dxffilein, dxffileout, minx, miny, maxx, maxy).unwrap();
        return;
    }

    if command == "smoothjoin" {
        pullauta::merge::smoothjoin(&fs, &config, &tmpfolder).unwrap();
    }

    if command == "xyzknolls" {
        pullauta::knolls::xyzknolls(&fs, &config, &tmpfolder).unwrap();
    }

    if command == "unzipmtk" {
        pullauta::process::unzipmtk(&fs, &config, &tmpfolder, &args).unwrap();
    }

    if command == "mtkshaperender" {
        pullauta::render::mtkshaperender(&fs, &config, &tmpfolder).unwrap();
    }

    if command == "xyz2contours" {
        let cinterval: f64 = args[0].parse::<f64>().unwrap();
        let xyzfilein = args[1].clone();
        let xyzfileout = args[2].clone();
        let dxffile = args[3].clone();
        let hmap =
            pullauta::contours::xyz2heightmap(&fs, &config, &tmpfolder, cinterval, &xyzfilein)
                .unwrap();

        if xyzfileout != "null" && !xyzfileout.is_empty() {
            hmap.to_file(&fs, xyzfileout).unwrap();
        }

        pullauta::contours::heightmap2contours(&fs, &tmpfolder, cinterval, &hmap, &dxffile)
            .unwrap();
        return;
    }

    if command == "render" {
        let angle: f64 = args[0].parse::<f64>().unwrap();
        let nwidth: usize = args[1].parse::<usize>().unwrap();
        let nodepressions: bool = args.len() > 2 && args[2] == "nodepressions";
        pullauta::render::render(
            &fs,
            &config,
            &thread,
            &tmpfolder,
            angle,
            nwidth,
            nodepressions,
        )
        .unwrap();
        return;
    }

    let proc = config.processes;
    if command.is_empty() && batch && proc > 1 {
        let mut handles: Vec<thread::JoinHandle<()>> = Vec::with_capacity((proc + 1) as usize);
        for i in 0..proc {
            let config = config.clone();
            let fs = fs.clone();
            let handle = thread::spawn(move || {
                info!("Starting thread");
                pullauta::process::batch_process(&config, &fs, &format!("{}", i + 1));
                info!("Thread complete");
            });
            thread::sleep(time::Duration::from_millis(100));
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
        return;
    }

    if (command.is_empty() && batch && proc < 2) || (command == "startthread" && batch) {
        thread = String::from("0");
        if !args.is_empty() {
            thread.clone_from(&args[0]);
        }
        if thread == "0" {
            thread = String::from("");
        }
        pullauta::process::batch_process(&config, &fs, &thread)
    }

    if command_lowercase.ends_with(".zip") {
        let mut zips: Vec<String> = vec![command];
        zips.extend(args);
        pullauta::process::process_zip(&fs, &config, &thread, &tmpfolder, &zips).unwrap();
        return;
    }

    if command_lowercase.ends_with(".las")
        || command_lowercase.ends_with(".laz")
        || command_lowercase.ends_with(".xyz")
        || command_lowercase.ends_with(".xyz.bin")
    {
        let mut norender: bool = false;
        if args.len() > 1 {
            norender = args[1].clone() == "norender";
        }

        // TEMP: use MemoryFileSystem for testing
        let fs = pullauta::io::fs::memory::MemoryFileSystem::new();

        debug!("Copying input file into memory fs: {}", command);
        {
            // copy the input file into the memory file system
            let bytes = std::fs::read(Path::new(&command)).expect("Could not read input file");

            let mut writer = fs
                .create("input.laz")
                .expect("Could not create output file");
            writer
                .write_all(&bytes)
                .expect("Could not write to output file");
        }
        debug!("Done");

        pullauta::process::process_tile(
            &fs,
            &config,
            &thread,
            &tmpfolder,
            // Path::new(&command),
            Path::new("input.laz"),
            norender,
        )
        .unwrap();

        debug!("{:#?}", fs);

        // now write the output files to disk
        if fs.exists("pullauta.png") {
            fn copy(fs: &MemoryFileSystem, name: &str) {
                let mut reader = BufReader::new(
                    fs.open(name)
                        .expect("Could not open output file for reading"),
                );

                let mut writer = BufWriter::new(
                    std::fs::File::create(name).expect("Could not create output file"),
                );

                std::io::copy(&mut reader, &mut writer).expect("Could not copy output file");
            }
            copy(&fs, "pullauta.png");
            copy(&fs, "pullauta_depr.png");
        }
    }
}
