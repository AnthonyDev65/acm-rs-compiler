// Sin consola extra en Windows en las compilaciones release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    acm_ide_lib::run()
}
