use std::io::{BufReader, Cursor, Write};
use std::path::PathBuf;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder as TarBuilder;

use clap::builder::PossibleValue;
use clap::{Arg, Command};

use hdpictureconverter::Image;

fn var_prefix_str(s: &str) -> Result<String, String> {
    let len = s.chars().count();
    if len != 2 {
        return Err(format!(
            "var_prefix must be exactly two characters, but is {}",
            len
        ));
    }

    for (i, c) in s.chars().enumerate() {
        if !c.is_ascii_alphabetic() {
            return Err(format!(
                "{:?} at var_prefix position {} is not an alphabetic character",
                c, i
            ));
        }
    }

    Ok(s.into())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum QuantizerChoice {
    LibImageQuant,
    NeuQuant,
}

impl clap::ValueEnum for QuantizerChoice {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::LibImageQuant, Self::NeuQuant]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::LibImageQuant => Some(PossibleValue::new("imagequant")),
            Self::NeuQuant => Some(PossibleValue::new("neuquant")),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let m = Command::new("HD picture converter")
        .args([
            Arg::new("image_file")
                .value_parser(clap::value_parser!(PathBuf))
                .required(true),
            Arg::new("var_prefix")
                .value_parser(var_prefix_str)
                .required(true),
            Arg::new("out_dir")
                .short('o')
                .long("outdir")
                .default_value(".")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Write 8xv files to this directory"),
        ])
        .get_matches();

    let image_file = m.get_one::<PathBuf>("image_file").unwrap();
    let var_prefix = m.get_one::<String>("var_prefix").unwrap();
    let out_dir = m.get_one::<PathBuf>("out_dir").unwrap();

    // Produce a single compressed `.8xg` file containing all appvar bytes
    // (tar of individual `.8xv` files, gzipped).
    let out_file_name = image_file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());
    let mut out_path = out_dir.clone();
    out_path.push(out_file_name);
    out_path.set_extension("8xg");

    eprintln!("Opening image file {:?}", &image_file);
    let image = {
        let f = std::fs::File::open(&image_file)?;
        Image::new(
            BufReader::new(f),
            &image_file.file_name().unwrap().to_string_lossy(),
            var_prefix,
        )
    }?;

    eprintln!("Quantizing..");
    let image = image.quantize();

    // Build a tar archive in memory with all `.8xv` appvars, then gzip it to one `.8xg` file.
    eprint!("Packaging appvars into {}..", out_path.display());

    let mut tar_buf = Vec::new();
    let mut tar = TarBuilder::new(&mut tar_buf);

    for tile in image.tiles() {
        eprint!(" {}", tile.appvar_name());
        let mut buf = Cursor::new(Vec::new());
        tile.write_appvar(&mut buf)?;
        let var_data = buf.into_inner();

        let mut header = tar::Header::new_gnu();
        header.set_size(var_data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, format!("{}.8xv", tile.appvar_name()), Cursor::new(var_data))?;
    }

    // Palette
    eprint!(" palette");
    let mut pbuf = Cursor::new(Vec::new());
    image.write_palette_appvar(&mut pbuf)?;
    let palette_data = pbuf.into_inner();
    let mut pheader = tar::Header::new_gnu();
    pheader.set_size(palette_data.len() as u64);
    pheader.set_mode(0o644);
    pheader.set_cksum();
    tar.append_data(&mut pheader, format!("{}.8xv", image.palette_appvar_name()), Cursor::new(palette_data))?;

    tar.finish()?;
    // drop the tar builder to release the mutable borrow of `tar_buf`
    std::mem::drop(tar);

    // Gzip the tar and write to the single .8xg output file
    eprintln!();
    let out_file = std::fs::File::create(&out_path)?;
    let mut encoder = GzEncoder::new(out_file, Compression::default());
    encoder.write_all(&tar_buf)?;
    encoder.finish()?;

    Ok(())
}
