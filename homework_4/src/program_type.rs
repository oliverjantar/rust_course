use std::{
    error::Error,
    io,
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};

use crate::operation::Operation;

///Represents operation and data to process
type OperationData = (Operation, String);

pub enum ProgramType {
    OneShot(OneShot),
    Interactive(Interactive),
}

impl ProgramType {
    pub fn init(args: &[String]) -> Self {
        if args.is_empty() {
            Self::Interactive(Interactive::start())
        } else {
            Self::OneShot(OneShot::start(&args[0]))
        }
    }
    pub fn process(self) {
        match self {
            ProgramType::OneShot(one_shot) => one_shot.process(),
            ProgramType::Interactive(interactive) => interactive.process(),
        }
    }
}

pub struct OneShot {
    result: Result<OperationData, Box<dyn Error>>,
}

impl OneShot {
    fn start(arg: &str) -> Self {
        let result = OneShot::init_one_shot(arg);
        OneShot { result }
    }

    fn process(&self) {
        match &self.result {
            Ok((operation, data)) => format_data(operation, data),
            Err(error) => eprint!("{}", error),
        }
    }

    fn init_one_shot(arg: &str) -> Result<OperationData, Box<dyn Error>> {
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
}

pub struct Interactive {
    receiver: Receiver<OperationData>,
    handle: JoinHandle<Result<(), String>>,
}

impl Interactive {
    fn start() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        let handle: JoinHandle<Result<(), String>> =
            thread::spawn(move || Self::interactive_thread(sender));

        Self { receiver, handle }
    }
    fn process(self) {
        while let Ok((operation, data)) = self.receiver.recv() {
            format_data(&operation, &data)
        }

        if let Err(e) = self.handle.join() {
            eprintln!("Error while reading input data. {:?}", e);
        }
    }

    fn interactive_thread(sender: Sender<(Operation, String)>) -> Result<(), String> {
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
                return Err("Error while sending data for processing. Ending program.".to_string());
            }
        }
        Ok(())
    }

    fn parse_params(args: &str) -> Result<(Operation, String), Box<dyn Error>> {
        let mut parts = args.splitn(2, ' ');
        let first_arg = parts.next().unwrap_or("");
        let second_arg = parts.next().unwrap_or("");

        let operation = Operation::try_from(first_arg)?;

        Ok((operation, second_arg.to_string()))
    }
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_parse_params() {
        let input = "lowercase This is some text";
        let parsed_data = Interactive::parse_params(input);

        assert!(parsed_data.is_ok());
        assert_eq!(
            parsed_data.unwrap(),
            (Operation::Lowercase, "This is some text".to_string())
        );
    }

    #[test]
    fn should_parse_params_for_csv() {
        let input = "csv input.csv";
        let parsed_data = Interactive::parse_params(input);

        assert!(parsed_data.is_ok());
        assert_eq!(
            parsed_data.unwrap(),
            (Operation::Csv, "input.csv".to_string())
        );
    }

    #[test]
    fn should_return_error_for_invalid_operation() {
        let input = "tsdfsd input.csv";
        let parsed_data = Interactive::parse_params(input);

        assert!(parsed_data.is_err());
    }
}
