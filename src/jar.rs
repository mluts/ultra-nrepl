use failure::Error;
use std::fs::File;
use std::io::Read;

pub fn read_jar_file(jar_path: String, file: String) -> Result<String, Error> {
    let mut out = String::new();

    let f = File::open(jar_path)?;

    let mut zip = zip::ZipArchive::new(f)?;

    let mut zip_file = zip.by_name(&file)?;

    zip_file.read_to_string(&mut out)?;

    Ok(out)
}
