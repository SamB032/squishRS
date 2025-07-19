use squishrs::run;

use colored::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}: {e}", "Error".red());
        std::process::exit(1);
    }
}
