use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use colored_text::Colorize;
use chrono::{DateTime,Utc};
use clap::Parser;
use exif::{In, Tag};
use walkdir::WalkDir;

/// Sort image files into folders by EXIF creation date (YYYY/MM/DD)
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Source folder to search recursively for images
    #[arg(short, long)]
    input: PathBuf,

    /// Output folder where dated subfolders will be created
    #[arg(short, long)]
    output: PathBuf,

    /// Copy files instead of moving them
    #[arg(short, long, default_value_t = false)]
    copy: bool,

    /// Dry run: print what would happen without doing it
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
}

/// Extensions we consider image files
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "tiff", "tif", "heic", "heif", "webp", "png", "cr2", "cr3",
    "nef", "arw", "orf", "rw2", "dng", "raf",
];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Try to extract (year, month, day) from EXIF DateTimeOriginal or DateTime.
fn exif_date(path: &Path) -> Option<(String, String, String)> {
    let file = fs::File::open(path).ok()?;
    let mut bufreader = std::io::BufReader::new(file);
    let exif: exif::Exif = exif::Reader::new().read_from_container(&mut bufreader).ok()?;
    
    // Prefer DateTimeOriginal, fall back to DateTimeDigitized, then DateTime
    let tags = [
        Tag::DateTimeOriginal,
        Tag::DateTimeDigitized,
        Tag::DateTime,
    ];

    for tag in &tags {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY) {
            let raw: String = field.display_value().to_string();
            // Format: "2023:07:15 14:32:01"
            let parts: Vec<&str> = raw.split(' ').collect();
            if let Some(date_part) = parts.first() {
                let date_components: Vec<&str> = date_part.split('-').collect();
                if date_components.len() == 3 {
                    let year = date_components[0].to_string();
                    let month = date_components[1].to_string();
                    let day = date_components[2].to_string();
                    if year.len() == 4 && month.len() == 2 && day.len() == 2 {
                        return Some((year, month, day));
                    }
                }
            }
        }
    }
    None
}

fn file_date(path: &Path) -> Option<(String, String, String)> {
    let file_metadata = fs::metadata(path).ok()?;
    let created = file_metadata.created().ok()?;
    let datetime = DateTime::<Utc>::from(created).to_string();
    let date = &datetime[0..10];
    let date_components: Vec<&str> = date.split('-').collect();
    if date_components.len() == 3 {
        let year = date_components[0].to_string();
        let month = date_components[1].to_string();
        let day = date_components[2].to_string();
        if year.len() == 4 && month.len() == 2 && day.len() == 2 {
            return Some((year, month, day));
        }
    }
    None
}


/// Resolve destination path, appending a counter suffix if a file already exists.
fn resolve_dest(dest_dir: &Path, filename: &str) -> PathBuf {
    let mut dest = dest_dir.join(filename);
    if !dest.exists() {
        return dest;
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let mut counter = 1u32;
    loop {
        dest = dest_dir.join(format!("{}_{}{}", stem, counter, ext));
        if !dest.exists() {
            return dest;
        }
        counter += 1;
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if !args.input.is_dir() {
        anyhow::bail!("Input path {:?} is not a directory", args.input);
    }

    let mut moved = 0usize;
    let mut skipped = 0usize;
    let mut no_exif = 0usize;
    let mut errors = 0usize;

    let mut created_dirs: HashMap<PathBuf, bool> = HashMap::new();

    for entry in WalkDir::new(&args.input)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let src = entry.path();

        if !is_image(src) {
            continue;
        }

        let filename = match src.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => {
                eprintln!("⚠  Skipping file with non-UTF8 name: {:?}", src);
                skipped += 1;
                continue;
            }
        };

        match exif_date(src) {
            Some((year, month, day)) => {
                let dest_dir = args.output.join(&year).join(&month).join(&day);

                if !args.dry_run {
                    if !created_dirs.contains_key(&dest_dir) {
                        fs::create_dir_all(&dest_dir)?;
                        created_dirs.insert(dest_dir.clone(), true);
                    }
                }

                let dest = resolve_dest(&dest_dir, &filename);

                if args.dry_run {
                    println!("[DRY RUN] {} - {} {} → {}", "[EXIF]".green(), if args.copy { "COPY" } else { "MOVE" }, src.display(), dest.display());
                    moved += 1;
                } else {
                    let result = if args.copy {
                        fs::copy(src, &dest).map(|_| ())
                    } else {
                        fs::rename(src, &dest).or_else(|_| {
                            // rename fails across filesystems; fall back to copy+delete
                            fs::copy(src, &dest).map(|_| ())?;
                            fs::remove_file(src)
                        })
                    };

                    match result {
                        Ok(_) => {
                            println!("✓ {} - {} → {}", "[EXIF]".green(), src.display(), dest.display());
                            //println!("✓ {}/{}/{} ← {}", year, month, day, filename);
                            moved += 1;
                        }
                        Err(e) => {
                            eprintln!("✗ Error processing {:?}: {}", src, e);
                            errors += 1;
                        }
                    }
                }
            }
            None => {
                let (year, month, day) = file_date(src).unwrap_or_else(|| ("_no_date".into(), "_no_date".into(), "_no_date".into()));
                let dest_dir = args.output.join(&year).join(&month).join(&day);
                no_exif += 1;
                
                if !args.dry_run {
                    if !created_dirs.contains_key(&dest_dir) {
                        fs::create_dir_all(&dest_dir)?;
                        created_dirs.insert(dest_dir.clone(), true);
                    }
                }

                let dest = resolve_dest(&dest_dir, &filename);

                if args.dry_run {
                    println!("[DRY RUN] {} - {} {} → {}", "[FILE]".yellow(), if args.copy { "COPY" } else { "MOVE" }, src.display(), dest.display());
                    moved += 1;
                } else {
                    let result = if args.copy {
                        fs::copy(src, &dest).map(|_| ())
                    } else {
                        fs::rename(src, &dest).or_else(|_| {
                            // rename fails across filesystems; fall back to copy+delete
                            fs::copy(src, &dest).map(|_| ())?;
                            fs::remove_file(src)
                        })
                    };
                    match result {
                        Ok(_) => {
                            println!("✓ {} - {} → {}", "[FILE]".yellow(), src.display(), dest.display());
                            moved += 1;
                        }
                        Err(e) => {
                            eprintln!("✗ Error processing {:?}: {}", src, e);
                            errors += 1;
                        }
                    }
              }
            
            }
        }
    }

    println!("\n── Summary ──────────────────────────────");
    println!("  {} {} in total", moved, if args.copy { "copied" } else { "moved" });
    println!("  {} {} using file date", no_exif, if args.copy { "copied" } else { "moved" });
    println!("  {} errors", errors);
    if skipped > 0 {
        println!("  {} skipped (non-UTF8 names)", skipped);
    }

    Ok(())
}
