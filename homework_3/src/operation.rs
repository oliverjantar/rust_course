use std::error::Error;

use convert_case::{Case, Casing};
use slug::slugify;

#[derive(Debug, PartialEq)]
pub enum Operation {
    Lowercase,
    Uppercase,
    NoSpace,
    Slugify,
    Random,
    Alternating,
}

impl Operation {
    pub fn format(&self, text: String) -> Result<String, Box<dyn Error>> {
        match self {
            Self::Lowercase => Self::to_lowercase(text),
            Self::Uppercase => Self::to_uppercase(text),
            Self::NoSpace => Self::replace(text),
            Self::Slugify => Self::slugify(text),
            Self::Random => Self::to_random_case(text),
            Self::Alternating => Self::to_alternating_case(text),
        }
    }

    pub fn from_array(args: &[String]) -> Result<Self, String> {
        if args.len() != 2 {
            return Err("Incorrect number of arguments. Please provide exactly one argument: lowercase, uppercase, no-spaces, slugify, random, alternating.".to_string());
        }

        let argument: Operation = args[1].as_str().try_into()?;
        Ok(argument)
    }

    fn to_lowercase(text: String) -> Result<String, Box<dyn Error>> {
        Ok(text.to_lowercase())
    }

    fn to_uppercase(text: String) -> Result<String, Box<dyn Error>> {
        Ok(text.to_uppercase())
    }

    fn replace(text: String) -> Result<String, Box<dyn Error>> {
        Ok(text.replace(' ', ""))
    }
    fn slugify(text: String) -> Result<String, Box<dyn Error>> {
        Ok(slugify(text))
    }

    fn to_random_case(text: String) -> Result<String, Box<dyn Error>> {
        Ok(text.to_case(Case::Random))
    }

    fn to_alternating_case(text: String) -> Result<String, Box<dyn Error>> {
        Ok(text.to_case(Case::Alternating))
    }
}

impl TryFrom<&str> for Operation {
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_operation_from_str() {
        let arg_str = "lowercase";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::Lowercase);

        let arg_str = "uppercase";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::Uppercase);

        let arg_str = "no-spaces";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::NoSpace);

        let arg_str = "slugify";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::Slugify);

        let arg_str = "random";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::Random);

        let arg_str = "alternating";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_ok());
        assert_eq!(arg.unwrap(), Operation::Alternating);

        let arg_str = "sth_else";

        let arg: Result<Operation, String> = arg_str.try_into();
        assert!(arg.is_err());
        assert_eq!(arg.unwrap_err(), "Invalid argument. Please use one of: lowercase, uppercase, no-spaces, slugify, random, alternating".to_string());
    }

    #[test]
    fn should_parse_operation_from_array() {
        let args = ["filename.rs".to_string(), "lowercase".to_string()];
        let operation = Operation::from_array(&args);

        assert!(operation.is_ok());
    }

    #[test]
    fn should_return_err_when_invalid_argument_is_present() {
        let args = ["filename.rs".to_string(), "sth".to_string()];
        let operation = Operation::from_array(&args);

        assert!(operation.is_err());
        assert_eq!(operation.unwrap_err(),"Invalid argument. Please use one of: lowercase, uppercase, no-spaces, slugify, random, alternating".to_string());
    }

    #[test]
    fn should_return_err_when_no_arg_is_present() {
        let args = ["filename.rs".to_string()];
        let operation = Operation::from_array(&args);

        assert!(operation.is_err());
        assert_eq!(operation.unwrap_err(),"Incorrect number of arguments. Please provide exactly one argument: lowercase, uppercase, no-spaces, slugify, random, alternating.".to_string());
    }

    #[test]
    fn should_return_err_when_more_args_are_present() {
        let args = [
            "filename.rs".to_string(),
            "lowercase".to_string(),
            "lowercase".to_string(),
        ];
        let operation = Operation::from_array(&args);

        assert!(operation.is_err());
        assert_eq!(operation.unwrap_err(),"Incorrect number of arguments. Please provide exactly one argument: lowercase, uppercase, no-spaces, slugify, random, alternating.".to_string());
    }

    #[test]
    fn should_return_formatted_text() {
        let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed";

        let result = Operation::Lowercase.format(text.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), text.to_lowercase());

        let result = Operation::Uppercase.format(text.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), text.to_uppercase());

        let result = Operation::NoSpace.format(text.to_string());
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "Loremipsumdolorsitamet,consecteturadipiscingelit,sed".to_string()
        );

        let result = Operation::Slugify.format(text.to_string());
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "lorem-ipsum-dolor-sit-amet-consectetur-adipiscing-elit-sed".to_string()
        );
    }
}
