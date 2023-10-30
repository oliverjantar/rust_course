use std::{
    error::Error,
    io,
    sync::mpsc::Receiver,
    thread::{self, JoinHandle},
};

use crate::operation::Operation;

///Represents operation and data to process
type OperationData = (Operation, String);

pub enum ProgramType {
    OneShot(Result<OperationData, Box<dyn Error>>),
    Interactive(Receiver<OperationData>, JoinHandle<Result<(), String>>),
}

//Pouzil jsem tady ten "builder pattern" ale moc se mi to nelibi.
//Bohuzel ten interaktivni mod byl dost jiny na zpracovani od toho druheho modu s parametrem, tak se mi to nepodarilo moc dobre rozdelit.
//Zkousel jsem jeste udelat to jednoduche zpracovani pomoci one shot channelu, abych vracel stejny typ, ale prislo mi pak zbytecny kvuli tomu spoustet thread.
impl ProgramType {
    pub fn init(args: &[String]) -> Self {
        if args.is_empty() {
            let (receiver, handle) = Self::start_interactive();
            return Self::Interactive(receiver, handle);
        }

        if args.len() != 1 {
            eprintln!("Incorrect number of arguments. Please provide zero arguments or one of: lowercase, uppercase, no-spaces, slugify, random, alternating, csv.");
        }

        let one_shot = Self::start_oneshot(&args[0]);

        Self::OneShot(one_shot)
    }
    pub fn process(self) {
        match self {
            ProgramType::OneShot(value) => match &value {
                Ok((operation, data)) => Self::format_data(operation, data),
                Err(error) => eprint!("{}", error),
            },
            ProgramType::Interactive(receiver, handle) => {
                while let Ok((operation, data)) = receiver.recv() {
                    Self::format_data(&operation, &data)
                }

                if let Err(e) = handle.join() {
                    eprintln!("Error while reading input data. {:?}", e);
                }
            }
        }
    }

    fn start_oneshot(arg: &str) -> Result<OperationData, Box<dyn Error>> {
        let operation = Operation::try_from(arg)?;
        match &operation {
            Operation::Csv => println!("Insert path to a csv file:"),
            _ => println!("Insert text:"),
        }
        let mut data = String::new();
        io::stdin().read_line(&mut data)?;

        let data = data.trim().to_string();

        Ok((operation, data))
    }

    fn start_interactive() -> (Receiver<OperationData>, JoinHandle<Result<(), String>>) {
        let (sender, receiver) = std::sync::mpsc::channel();

        let handle: JoinHandle<Result<(), String>> = thread::spawn(move || {
            println!("Enter <command> <text> to format the data or 'q' to quit the program.");
            loop {
                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_err() {
                    eprintln!("Error while reading from console. Please try again or enter a different input.");
                    continue;
                }

                let input = input.trim();

                if input == "q" {
                    break;
                }

                let (operation, input_text) = match Self::parse_params(input) {
                    Ok(result) => result,
                    Err(error) => {
                        eprintln!("{}", error);
                        continue;
                    }
                };

                if sender.send((operation, input_text)).is_err() {
                    return Err(
                        "Error while sending data for processing. Ending program.".to_string()
                    );
                }
            }
            Ok(())
        });

        (receiver, handle)
    }

    fn format_data(operation: &Operation, data: &str) {
        let result = operation.format(data);
        match result {
            Ok(value) => println!("{value}"),
            Err(error) => eprintln!(
                "Error while using operation: {:?}. Error: {}",
                operation, error
            ),
        }
    }

    fn parse_params(args: &str) -> Result<(Operation, String), Box<dyn Error>> {
        let mut parts = args.splitn(2, ' ');
        let first_arg = parts.next().unwrap_or("");
        let second_arg = parts.next().unwrap_or("");

        let operation = Operation::try_from(first_arg)?;

        Ok((operation, second_arg.to_string()))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_parse_params() {
        let input = "lowercase This is some text";
        let parsed_data = ProgramType::parse_params(input);

        assert!(parsed_data.is_ok());
        assert_eq!(
            parsed_data.unwrap(),
            (Operation::Lowercase, "This is some text".to_string())
        );
    }

    #[test]
    fn should_parse_params_for_csv() {
        let input = "csv input.csv";
        let parsed_data = ProgramType::parse_params(input);

        assert!(parsed_data.is_ok());
        assert_eq!(
            parsed_data.unwrap(),
            (Operation::Csv, "input.csv".to_string())
        );
    }

    #[test]
    fn should_return_error_for_invalid_operation() {
        let input = "tsdfsd input.csv";
        let parsed_data = ProgramType::parse_params(input);

        assert!(parsed_data.is_err());
    }
}
