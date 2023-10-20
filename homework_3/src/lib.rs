use std::{
    error::Error,
    io::{self},
};

use crate::operation::Operation;

mod operation;

pub fn run(args: &[String]) -> Result<String, Box<dyn Error>> {
    let function = Operation::from_array(args)?;

    let mut text = String::new();
    println!("Insert text:");
    io::stdin().read_line(&mut text)?;

    function.format(text)
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
