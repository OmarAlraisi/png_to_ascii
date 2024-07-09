use std::io;
use test_proj::Image;

fn main() -> io::Result<()> {
    let image = Image::from("./image.png")?;
    println!("{}", image);
    Ok(())
}
