use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::Path;

fn main() {
    // https://kazlauskas.me/entries/writing-proper-buildrs-scripts
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    match target_os.as_ref().map(|x| &**x) {
        Ok("android") => {
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=c++_shared");
        }
        Ok(_) => {
            // println!("cargo:rerun-if-changed=res/*");

            let build_type = env::var("PROFILE").unwrap();
            let path = env::var("CARGO_TARGET_DIR")
                .map(|dir| Path::new(&dir).to_path_buf())
                .unwrap_or({
                    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
                    Path::new(&manifest_dir_string).join("target")
                })
                .join(build_type);

            let mut copy_options = CopyOptions::new();
            copy_options.overwrite = true;
            let mut paths_to_copy = Vec::new();
            paths_to_copy.push("res/");

            copy_items(&paths_to_copy, path.join("examples"), &copy_options).unwrap();
            copy_items(&paths_to_copy, path, &copy_options).unwrap();
        }
        _ => {}
    }
}
