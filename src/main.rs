use png_to_ascii::Image;
use std::{env, io};

fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    let file = args
        .next()
        .expect("ERR: Usage: png_to_ascii <path/to/image>");
    let image = Image::from(&file)?;
    println!("{}", image);
    Ok(())
}
