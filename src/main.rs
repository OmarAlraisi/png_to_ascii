use png_to_ascii::Img;
use std::{env, io};

fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    let file = args
        .next()
        .expect("ERR: Usage: png_to_ascii <path/to/image>");

    let image = Img::new(&file)?;
    println!(
        "<html>
    <body>
        <div style=\"line-height: 10px; font-size: 14px\">
            <pre>"
    );
    image.display();
    println!(
        "</pre>
        </div>
    </body>
</html>"
    );
    Ok(())
}
