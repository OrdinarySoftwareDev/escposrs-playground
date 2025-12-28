Some observations for this aliexpress printer:

**It works with the xp58.ppd driver as well as ESC/POS**
**It shows up as "gxmc micro-printer "**
**It's connected via /dev/usb/lp2**
**It requires 5V 2A minimum**

Demo code:

```rust
use escpos::printer::Printer;
use escpos::printer_options::PrinterOptions;
use escpos::utils::*;
use escpos::{driver::*, errors::Result};
use std::path::Path;

fn main() -> Result<()> {
    // env_logger::init();

    let printer_path = Path::new("/dev/usb/lp2");
    let driver = FileDriver::open(printer_path)?;
    Printer::new(driver, Protocol::default(), Some(PrinterOptions::default()))
        .debug_mode(Some(DebugMode::Dec))
        .init()?
        .smoothing(true)?
        .bold(true)?
        .underline(UnderlineMode::Single)?
        .writeln("Bold underline")?
        .justify(JustifyMode::CENTER)?
        .reverse(true)?
        .bold(false)?
        .writeln("Hello world - Reverse")?
        .feed()?
        .justify(JustifyMode::RIGHT)?
        .reverse(false)?
        .underline(UnderlineMode::None)?
        .size(2, 3)?
        .writeln("Hello world - Normal")?
        .print_cut()?;  // print() or print_cut() is mandatory to send the data to the printer

    Ok(())
}
```

And as for functionality:
``.smooth()`` does nothing
no cutter so ``.print_cut()`` doesn't really work either
``.reverse()`` reverses the foreground and background colors (for some reason it doesn't replace spaces)
