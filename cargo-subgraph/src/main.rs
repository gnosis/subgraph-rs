mod api;
mod cmd;
mod manifest;

fn main() {
    if let Err(err) = cmd::run() {
        eprintln!("ERROR: {}", err);
        std::process::exit(1);
    }
}
