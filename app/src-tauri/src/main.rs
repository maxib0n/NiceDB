#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "run") {
        std::process::exit(nicedb_app::cli_main(&args[1..]));
    }
    nicedb_app::run();
}
