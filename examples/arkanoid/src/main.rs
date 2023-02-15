#![windows_subsystem = "windows"]

pub mod menu;
pub mod game;

use crate::menu::MenuScene;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    shura::init("menu", MenuScene::new);
}

