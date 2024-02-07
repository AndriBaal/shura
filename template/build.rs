use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn copy_directory(src: &Path, dest: &Path) -> io::Result<()> {
    const EXCLUDE_EXTENSIONS: &'static [&'static str] = &["blend", "blend1", "aseprite", "ase", "aup3-shm", "aup3", "aup3-wal"];
    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest)?;

    // Iterate over entries in the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry_path.file_name().unwrap());

        if entry_path.is_dir() {
            // Recursively copy subdirectories
            copy_directory(&entry_path, &dest_path)?;
        } else {
            // Check if the file has an excluded extension
            if !EXCLUDE_EXTENSIONS
                .iter()
                .any(|ext| entry_path.extension().map_or(false, |e| e == *ext))
            {
                // Copy the file to the destination
                fs::copy(&entry_path, &dest_path)?;
            }
        }
    }

    Ok(())
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    match target_os.as_ref().map(|x| &**x) {
        Ok("android") => (),
        Ok(_) => {
            println!("cargo:rerun-if-changed=assets/*");

            let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            let root = Path::new(&cargo_dir);
            let assets = root.join("assets");
            if assets.exists() {
                let target_dir = env::var("CARGO_TARGET_DIR")
                    .map(|dir| Path::new(&dir).to_path_buf())
                    .unwrap_or(root.join("target"));

                let build_type = env::var("PROFILE").unwrap();
                let dest = target_dir.join(build_type);

                if dest.exists() {
                    copy_directory(&assets, &dest.join("assets")).unwrap();
                    let examples = dest.join("examples");
                    if examples.exists() {
                        copy_directory(&assets, &examples.join("assets")).unwrap();
                    }
                }
            }
        }
        _ => (),
    }
}
