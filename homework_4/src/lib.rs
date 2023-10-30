use program_type::{Interactive, OneShot};

pub mod operation;
pub mod program_type;

pub fn process(args: &[String]) {
    if args.is_empty() {
        Interactive::start()
    } else {
        OneShot::start(&args[0])
    }
}
