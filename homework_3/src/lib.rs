use std::{
    error::Error,
    io::{self, Read},
};

use crate::operation::Operation;

mod operation;

pub fn run(arg: &str) -> Result<String, Box<dyn Error>> {
    let operation = Operation::try_from(arg)?;

    let input_data = read_text(&operation)?;

    operation.format(&input_data)
}

fn read_text(operation: &Operation) -> Result<String, Box<dyn Error>> {
    println!("Insert text:");

    let text = match operation {
        Operation::Csv => {
            let mut data: Vec<_> = vec![];
            io::stdin().read_to_end(&mut data)?;
            String::from_utf8(data)?
        }
        _ => {
            let mut data = String::new();
            io::stdin().read_line(&mut data)?;
            data
        }
    };
    Ok(text)
}

/*
pub fn run<R, W>(mut reader: R, mut writer: W, args: &[String]) -> Result<String, Box<dyn Error>>
where
    R: Read,
    W: Write,
{
    let function = Operation::from_array(args)?;

    let mut text = String::new();

    let mut buf_reader = io::BufReader::new(reader);

    writer.write_all(b"Please insert text:")?;
    writer.flush()?;
    buf_reader.read_to_end(&mut text)?;

    function.format(text)
}
*/
