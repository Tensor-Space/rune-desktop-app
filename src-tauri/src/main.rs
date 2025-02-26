fn main() {
    let _ = fix_path_env::fix();
    if let Err(e) = rune_lib::run() {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
