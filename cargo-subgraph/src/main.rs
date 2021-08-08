mod api;
mod cmd;
mod linker;
mod manifest;
mod mappings;

fn main() {
    if let Err(err) = cmd::run() {
        eprintln!("ERROR: {}", err);
        std::process::exit(1);
    }
}
