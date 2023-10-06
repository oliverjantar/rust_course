use convert_case::{Case, Casing};
use slug::slugify;
use std::{env, io};

/// Run the program with 1 argument: lowercase, uppercase, no-spaces, slugify, random or alternating
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Incorrect number of arguments. Please provide exactly one argument: lowercase, uppercase, no-spaces, slugify, random, alternating.")
    }

    let argument: Argument = args[1].as_str().try_into().unwrap();

    let mut text = String::new();

    println!("Please insert text:");
    io::stdin()
        .read_line(&mut text)
        .expect("Failed to read line");

    let result = argument.format(text);

    println!("{}", result);
}

enum Argument {
    Lowercase,
    Uppercase,
    NoSpace,
    Slugify,
    Random,
    Alternating,
}

impl Argument {
    fn format(&self, text: String) -> String {
        match self {
            Self::Lowercase => text.to_lowercase(),
            Self::Uppercase => text.to_uppercase(),
            Self::NoSpace => text.replace(' ', ""),
            Self::Slugify => slugify(text),
            Self::Random => text.to_case(Case::Random),
            Self::Alternating => text.to_case(Case::Alternating),
        }
    }
}

impl TryFrom<&str> for Argument {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "lowercase" => Ok(Self::Lowercase),
            "uppercase" => Ok(Self::Uppercase),
            "no-spaces" => Ok(Self::NoSpace),
            "slugify" => Ok(Self::Slugify),
            "random" => Ok(Self::Random),
            "alternating" => Ok(Self::Alternating),
            _ => Err("Invalid argument. Please use one of: lowercase, uppercase, no-spaces, slugify, random, alternating".to_string()),
        }
    }
}
