use std::env;

use homework_4::program_type::ProgramType;

/// Run the program with zero arguments to run in interactive mode or with one argument: lowercase, uppercase, no-spaces, slugify, random, alternating or csv
fn main() {
    let args: Vec<String> = env::args().collect();

    ProgramType::init(&args[1..]).process();
}
