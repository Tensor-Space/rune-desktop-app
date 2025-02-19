fn main() {
    if let Err(e) = rune_lib::run() {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
