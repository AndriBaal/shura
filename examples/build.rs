use fs_extra::{copy_items, dir::CopyOptions};
use std::env;
use std::path::Path;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    match target_os.as_ref().map(|x| &**x) {
        Ok("android") => (),
        Ok(_) => {
            println!("cargo:rerun-if-changed=res/*");

            let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            let root = Path::new(&cargo_dir);
            let res = root.join("res");
            if res.exists() {
                let mut copy_options = CopyOptions::new();
                copy_options.overwrite = true;
                let target_dir = env::var("CARGO_TARGET_DIR")
                    .map(|dir| Path::new(&dir).to_path_buf())
                    .unwrap_or(root.join("target"));

                let build_type = env::var("PROFILE").unwrap();
                let dest = target_dir.join(build_type);

                if dest.exists() {
                    copy_items(&[res.clone()], dest.clone(), &copy_options).unwrap();
                    let examples = dest.join("examples");
                    if examples.exists() {
                        copy_items(&[res], examples, &copy_options).unwrap();
                    }
                }
            }
        }
        _ => (),
    }
}
