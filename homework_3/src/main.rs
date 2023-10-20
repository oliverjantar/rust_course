use std::env;

use homework_3::run;

/// Run the program with 1 argument: lowercase, uppercase, no-spaces, slugify, random or alternating
fn main() {
    let args: Vec<String> = env::args().collect();
    let result = run(&args);

    match result {
        Ok(value) => println!("{value}"),
        Err(error) => eprintln!("{error}"),
    }
}
