use std::env;

use homework_3::run;

/// Run the program with 1 argument: lowercase, uppercase, no-spaces, slugify, random, alternating or csv
/// Then insert one line to std input. In case of csv you can pass multiple lines
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Incorrect number of arguments. Please provide exactly one argument: lowercase, uppercase, no-spaces, slugify, random, alternating, csv.");
    }
    let arg = &args[1];
    let result = run(arg);

    match result {
        Ok(value) => println!("{value}"),
        Err(error) => eprintln!("Error while using operation: {arg}. Error: {error}"),
    }
}
