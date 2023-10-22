use std::{error::Error, fmt::Display};

use convert_case::{Case, Casing};
use csv::{Reader, StringRecord};
use slug::slugify;

#[derive(Debug, PartialEq)]
pub enum Operation {
    Lowercase,
    Uppercase,
    NoSpace,
    Slugify,
    Random,
    Alternating,
    Csv,
}

impl Operation {
    pub fn format(&self, text: &str) -> Result<String, Box<dyn Error>> {
        match self {
            Self::Lowercase => Self::to_lowercase(text),
            Self::Uppercase => Self::to_uppercase(text),
            Self::NoSpace => Self::replace(text),
            Self::Slugify => Self::slugify(text),
            Self::Random => Self::to_random_case(text),
            Self::Alternating => Self::to_alternating_case(text),
            Self::Csv => Self::to_csv(text),
        }
    }
    // Extrahoval jsem tyto funkce jak bylo v zadani, ale prijde mi ze to je k nicemu.
    // Az na `to_csv()` to nedela nic navic a mohlo to byt v tom matchi nahore ^
    fn to_lowercase(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(text.to_lowercase())
    }

    fn to_uppercase(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(text.to_uppercase())
    }

    fn replace(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(text.replace(' ', ""))
    }
    fn slugify(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(slugify(text))
    }

    fn to_random_case(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(text.to_case(Case::Random))
    }

    fn to_alternating_case(text: &str) -> Result<String, Box<dyn Error>> {
        Ok(text.to_case(Case::Alternating))
    }

    fn to_csv(text: &str) -> Result<String, Box<dyn Error>> {
        let csv = Csv::from_string(text)?;
        Ok(csv.to_string())
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
            "csv"=> Ok(Self::Csv),
            _ => Err("Invalid argument. Please use one of: lowercase, uppercase, no-spaces, slugify, random, alternating, csv".to_string()),
        }
    }
}

struct Csv {
    headers: StringRecord,
    data: Vec<StringRecord>,
}

impl Csv {
    fn from_string(input_str: &str) -> Result<Self, Box<dyn Error>> {
        let mut reader = Reader::from_reader(input_str.as_bytes());
        let headers = reader.headers()?.clone();
        let mut data = vec![];

        for result in reader.records() {
            data.push(result?);
        }

        let csv = Csv { headers, data };

        Ok(csv)
    }
}

impl Display for Csv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.headers.is_empty() {
            return Ok(());
        }

        // Set column width based on the widest cell
        let mut widths = Vec::with_capacity(self.headers.len());
        for header in self.headers.iter() {
            widths.push(header.len());
        }
        for row in &self.data {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.len());
            }
        }

        // Format headers and separator
        let mut separator = "|".to_string();
        let header_str = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, header)| {
                separator.push_str(&"-".repeat(widths[i] + 3));
                format!("| {: <width$} ", header, width = widths[i])
            })
            .collect::<Vec<_>>()
            .join("");
        separator.push('|');
        writeln!(f, "{}", separator)?;
        writeln!(f, "{} |", header_str)?;

        // Format data
        writeln!(f, "{}", separator)?;
        for row in &self.data {
            let row_str = row
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("| {: <width$} ", cell, width = widths[i]))
                .collect::<Vec<_>>()
                .join("");
            writeln!(f, "{} |", row_str)?;
        }
        writeln!(f, "{}", separator)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn valid_csv_should_print_a_table() {
        let data = "\
city,country,pop
Boston,United States,4628910
Prague,Czech Republic,123456
test,some looooooong string,11111111111111111111
,,
,,2929292929292
Paris,,1
,Greenland,";

        let csv = Csv::from_string(data);
        assert!(csv.is_ok());
        println!("{}", csv.unwrap());
    }

    #[test]
    fn valid_csv_should_print_a_table2() {
        let data = "\
Username, Identifier,One-time password,Recovery code,First name,Last name,Department,Location
booker12,,12se74,rb9012,Rachel,Booker,Sales,Manchester
grey07,2070,04ap67,lg2070,Laura,Grey,,London
johnson81,4081,30no86,cj4081,Craig,Johnson,Depot,London
jenkins46,9346,,mj9346,Mary,Jenkins,Engineering,Manchester
smith79,5079,09ja61,js5079,Jamie,Smith,Engineering,";

        let csv = Csv::from_string(data);
        assert!(csv.is_ok());
        println!("{}", csv.unwrap());
    }

    #[test]
    fn invalid_csv_should_return_error() {
        let data = "\
city,country,pop
Boston,";

        let csv = Csv::from_string(data);
        assert!(csv.is_err());
    }

    #[test]
    fn parse_operation_from_str() {
        let operations = vec![
            ("lowercase", Operation::Lowercase),
            ("uppercase", Operation::Uppercase),
            ("no-spaces", Operation::NoSpace),
            ("slugify", Operation::Slugify),
            ("random", Operation::Random),
            ("alternating", Operation::Alternating),
            ("csv", Operation::Csv),
        ];

        for (op_string, expected) in operations {
            let operation_result: Result<Operation, String> = op_string.try_into();
            assert_eq!(operation_result.unwrap(), expected);
        }
    }

    #[test]
    fn invalid_op_should_return_error() {
        let arg: Result<Operation, String> = "sth".try_into();
        assert!(arg.is_err());
        assert_eq!(arg.unwrap_err(), "Invalid argument. Please use one of: lowercase, uppercase, no-spaces, slugify, random, alternating, csv".to_string());
    }

    #[test]
    fn should_return_formatted_text() {
        let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed";

        let test_data = vec![
            (Operation::Lowercase, text.to_lowercase()),
            (Operation::Uppercase, text.to_uppercase()),
            (
                Operation::NoSpace,
                "Loremipsumdolorsitamet,consecteturadipiscingelit,sed".to_string(),
            ),
            (
                Operation::Slugify,
                "lorem-ipsum-dolor-sit-amet-consectetur-adipiscing-elit-sed".to_string(),
            ),
        ];

        for (operation, expected) in test_data {
            let formatted = operation.format(text);
            assert!(formatted.is_ok());
            assert_eq!(formatted.unwrap(), expected);
        }
    }
}
