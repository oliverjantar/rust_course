use std::env;

use homework_4::process;

/// Run the program with zero arguments to run in interactive mode or with one argument: lowercase, uppercase, no-spaces, slugify, random, alternating or csv
fn main() {
    let args: Vec<String> = env::args().collect();

    process(&args[1..]);
}
