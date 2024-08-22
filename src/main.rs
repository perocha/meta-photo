use serde::Deserialize;
use std::fs;
use rexif::ExifTag;
use rexif::parse_file;
use glob::glob;
use std::path::Path;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone)]
struct F64(pub f64);

impl PartialEq for F64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for F64 {}

impl Hash for F64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct MetaData {
    f_stop: F64,
    exposure: String,
    iso: u32,
}

const CONFIG_FILE_PATH: &str = "config.yaml"; // or "config.json"

#[derive(Deserialize)]
struct Config {
    filepath: String,
}

// Function to load the config file
fn load_config_from_yaml(path: &str) -> Config {
    let config_data = fs::read_to_string(path).expect("Unable to read config file");
    serde_yaml::from_str(&config_data).expect("Unable to parse YAML config")
}

// Constants for standard shutter speeds
const STANDARD_SHUTTER_SPEEDS: [(f64, &str); 19] = [
    (1.0/8000.0, "1/8000"),
    (1.0/4000.0, "1/4000"),
    (1.0/2000.0, "1/2000"),
    (1.0/1000.0, "1/1000"),
    (1.0/500.0,  "1/500"),
    (1.0/250.0,  "1/250"),
    (1.0/125.0,  "1/125"),
    (1.0/60.0,   "1/60"),
    (1.0/30.0,   "1/30"),
    (1.0/15.0,   "1/15"),
    (1.0/8.0,    "1/8"),
    (1.0/4.0,    "1/4"),
    (1.0/2.0,    "1/2"),
    (1.0,        "1"),
    (2.0,        "2"),
    (4.0,        "4"),
    (8.0,        "8"),
    (15.0,       "15"),
    (30.0,       "30"),
];

// Function to find the closest standard shutter speed
fn to_closest_shutter_speed(value: f64) -> &'static str {
    let mut closest = STANDARD_SHUTTER_SPEEDS[0];
    let mut min_diff = (value - closest.0).abs();

    for &(standard_value, label) in &STANDARD_SHUTTER_SPEEDS {
        let diff = (value - standard_value).abs();
        if diff < min_diff {
            closest = (standard_value, label);
            min_diff = diff;
        }
    }

    closest.1
}

fn main() {
    // Load the config file
    let config = load_config_from_yaml(CONFIG_FILE_PATH);

    // Use the pattern from the config file
    let pattern = &config.filepath;

    let mut metadata_map: HashMap<MetaData, u32> = HashMap::new();

    // Iterate over each file matching the pattern
    for entry in glob(pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if let Some((f_stop, shutter_speed, iso)) = extract_exif_data(&path) {
                    let meta_data = MetaData {
                        f_stop: F64(f_stop),
                        exposure: to_closest_shutter_speed(shutter_speed).to_string(),
                        iso,
                    };

                    *metadata_map.entry(meta_data).or_insert(0) += 1;
                }
            },
            Err(e) => println!("{:?}", e),
        }
    }

    // Print the grouped results
    for (data, count) in &metadata_map {
        println!(
            "f-stop: f/{}, exposure: {}, ISO: {} -> count: {}",
            data.f_stop.0, data.exposure, data.iso, count
        );
    }
}

fn extract_exif_data(path: &Path) -> Option<(f64, f64, u32)> {
    // Parse the EXIF data from the file path
    let exif = parse_file(path).ok()?;

    // Extract f-stop
    let f_stop = exif.entries.iter().find_map(|entry| {
        if entry.tag == ExifTag::FNumber {
            entry.value.to_f64(0) // Get the first element
        } else {
            None
        }
    });

    // Extract shutter speed
    let shutter_speed = exif.entries.iter().find_map(|entry| {
        if entry.tag == ExifTag::ExposureTime {
            entry.value.to_f64(0) // Get the first element
        } else {
            None
        }
    });

    // Extract ISO
    let iso = exif.entries.iter().find_map(|entry| {
        if entry.tag == ExifTag::ISOSpeedRatings {
            match &entry.value {
                rexif::TagValue::U16(values) => values.get(0).cloned().map(|v| v as u32),
                rexif::TagValue::U32(values) => values.get(0).cloned(),
                _ => None,
            }
        } else {
            None
        }
    });

    match (f_stop, shutter_speed, iso) {
        (Some(f), Some(s), Some(i)) => Some((f, s, i)),
        _ => None,
    }
}
