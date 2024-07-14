use png_to_ascii::Img;
use std::{env, io};

fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    let file = args
        .next()
        .expect("ERR: Usage: png_to_ascii <path/to/image>");

    let image = Img::new(&file)?;
    image.display();
    Ok(())
}
