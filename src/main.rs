#![allow(dead_code)]
use std::fmt;
//use std::from;
use std::path::{Path, PathBuf};
use std::process::Command;

use escpos::printer::{Printer};
use escpos::printer_options::PrinterOptions;
use escpos::ui::line::LineBuilder;
use escpos::utils::*;
use escpos::{driver::*};

use bmp::{Image, Pixel};
use anyhow::Result;

use simple_logger::SimpleLogger;
use log::LevelFilter;

const CURRENCY: &'static str = "zl";

fn center_string(s: String, width: usize) {
    let length = s.chars().count() as usize;
    let pad = (width - length) as f32 / 2.0f32;
    format!("{}{}{}", " ".repeat(pad.floor() as usize), s, " ".repeat(pad.ceil() as usize));
}

// rounding mode enum
enum RoundingMode {
    Never, // never round the price
    IfNameTooLong, // round price only if the name doesn't fit
    Always // round the price whenever possible
}

// name shortening mode enum
enum ItemNameShorteningMode {
    Trim, // trim the name
    TrimDot, // trim the name but replace last character with dot
    SymmetricalDot // shorten all words equally if possible (most human-readable)
}

// receipt options struct
struct ReceiptOptions {
    width: u8, // max text width
    item_name_shortening: ItemNameShorteningMode,
    left_leaning_price: bool, // price placed to the left of the currency symbol (eg. 12.99$)
    rounding: RoundingMode,
    show_quantities: bool, // whether to show item quantities
    show_single_item_quantity: bool,
    currency_symbol: &'static str,
    logo_path: Option<String>,
    logo_bitimageoption: BitImageOption,
    barcode: Option<Barcode>
}

impl Default for ReceiptOptions {
    fn default() -> Self {
        Self {
            width: 32,
            item_name_shortening: ItemNameShorteningMode::Trim,
            left_leaning_price: true,
            rounding: RoundingMode::IfNameTooLong,
            show_quantities: true,
            show_single_item_quantity: true,
            currency_symbol: CURRENCY,
            logo_path: None,
            logo_bitimageoption: BitImageOption::default(),
            barcode: None
        }
    }
}

impl ReceiptOptions {
    fn from_printer(printer: &mut Printer<FileDriver>) -> Self { // get width from printer
        Self {
            width: printer.options().get_characters_per_line(),
            ..Self::default()
        }
    }
}

// receipt struct
struct Receipt<'a> {
    business_name: &'a str,
    address: &'a str,
    contact_info: &'a str,
    items: Vec<Item<'a>>,
    tax_percent: u8,
    footer: &'a str,
    options: &'a ReceiptOptions
}

impl Receipt<'_> {
    fn write(&self, printer: &mut Printer<FileDriver>) -> Result<()> {
        let default_line = LineBuilder::new()
            .width(self.options.width)
            .build();

        // first some basic info
        printer
        .justify(JustifyMode::CENTER)?;
        if let Some(logo_path) = &self.options.logo_path {
            if let Err(e) = printer.bit_image_option(&logo_path, self.options.logo_bitimageoption) {
                eprintln!("Failed to print logo: {:?}", e);
            }
        }

        printer
            .write(self.business_name)?
            .write(self.address)?
            .write(self.contact_info)?
            .draw_line(default_line)?;

        // now the items

        for _ in &self.items {
            println!("");
        }

        Ok(())
    }
}

/*impl fmt::Display for Receipt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        writeln!(self.business_name);
    }
}*/

// item struct
struct Item<'a> {
    name: String,
    price: u32, // in cents
    quantity: Option<u32>,
    quantity_grams: Option<u32>,
    options: &'a ReceiptOptions
}

impl Item<'_> {
    fn format_price(&self, round_price: &mut bool) -> String {
        let mut precision: usize = 2;

        if *round_price && self.price % 100 == 0 {
            precision = 0;
        }

        let price_string: String = format!("{:.1$}", self.price as f64 / 100.0, precision);

        if self.options.left_leaning_price {
            format!("{}{}", price_string, self.options.currency_symbol)
        } else {
            format!("{}{}", self.options.currency_symbol, price_string)
        }
    }

    fn max_name_length(&self, price: &String, quantity_string: &String) -> u8 {
        self.options.width
            - price.chars().count() as u8
            - quantity_string.chars().count() as u8
    }

    fn pad_name(&self, n: usize) -> String {
        let mut name = self.name.clone();
        while name.chars().count() < n {
            name.push(' ');
        }
        name
    }

    fn format(&self) -> String { // formatter for the 'Item' struct, fits an item into the max width of the printer
        let mut round_price = matches!(self.options.rounding, RoundingMode::Always);
        let mut price = self.format_price(&mut round_price);

        let show_single = self.options.show_single_item_quantity;
        let quantity_string: String = match (self.quantity_grams, self.quantity.filter(|&q| show_single || q != 1)) {
            (Some(grams), _) => format!(" {grams}g "),
            (None, Some(q)) => format!(" {q} "),
            _ => " ".to_string()
        };

        let mut name = self.name.clone();
        let initial_name_length = name.chars().count() as u8;

        // first check if everything fits
        let mut max_name_length = self.max_name_length(&price, &quantity_string);

        // if the name is too long, we haven't tried rounding the price yet, and we can round at all
        if initial_name_length > max_name_length && !round_price && !matches!(self.options.rounding, RoundingMode::Never) {
            round_price = true;
            price = self.format_price(&mut round_price);

            // check again
            max_name_length = self.max_name_length(&price, &quantity_string);
        }

        // if the name is still too long, shorten
        if name.chars().count() as u8 > max_name_length {
            name = match self.options.item_name_shortening { // shortening always makes the name fit perfectly
                ItemNameShorteningMode::Trim => name.chars().take(max_name_length as usize).collect(),
                ItemNameShorteningMode::TrimDot => {
                    let mut s: String = name.chars().take(max_name_length as usize - 1).collect();
                    s.push('.');
                    s
                }
                ItemNameShorteningMode::SymmetricalDot => String::from("") // work in progress
            }
        } else {
            // fill the rest of the string with whitespace
            name = self.pad_name(max_name_length as usize)
        }
        /*loop {
            price = self.format_price(&mut round_price);
            name_segment_size = self.options.width
                - price.chars().count() as u8
                - quantity_string.chars().count() as u8;

            if initial_name_length > name_segment_size { // name is too long
                if !(matches!(self.options.round_price, RoundingMode::Never)) && !round_price{
                    // if we can round (and haven't already), try
                    round_price = true;
                } else {
                    // in any other case just shorten
                    name = self.shorten_name();
                    break;
                }
            } else {
                break;
            }
        }*/

        format!("{name}{quantity_string}{price}")
    }
}

// struct for multiple items
struct Items {

}

fn main() -> Result<()> { // print a receipt
    let receipt_options = ReceiptOptions {
        item_name_shortening: ItemNameShorteningMode::TrimDot,
        ..Default::default()
    };

    let printer_path = Path::new("/dev/usb/lp2");
    let driver = FileDriver::open(printer_path)?;

    let mut printer = Printer::new(driver, Protocol::default(), Some(PrinterOptions::default()));

    printer
        .debug_mode(Some(DebugMode::Hex))
        .init()?;

    let items = vec![
        Item { name: "Konstantynopolitanczykowianeczka".to_string(), price: 2500000, quantity: Some(1), quantity_grams: None, options: &receipt_options},
        Item { name: "Karta PaySafeCard 50".to_string(), price: 5000, quantity: Some(1), quantity_grams: None, options: &receipt_options},
        Item { name: "Rupurix 10".to_string(), price: 1600, quantity: Some(2), quantity_grams: None, options: &receipt_options},
        Item { name: "Kremowka".to_string(), price: 2137, quantity: None, quantity_grams: Some(670), options: &receipt_options},
        Item { name: "Dlugopis".to_string(), price: 1205, quantity: Some(20), quantity_grams: None, options: &receipt_options},
    ];

    let receipt = Receipt {
        business_name: &"",
        address: &"",
        contact_info: &"",
        items: items,
        tax_percent: 23,
        footer: &"",
        options: &receipt_options
    };

    //print!("{}", receipt);
    Ok(())
}

/*fn main() -> Result<()> {
    println!("Initializing printer...");
    SimpleLogger::new().with_level(LevelFilter::Debug).init().unwrap();

    // Initialize printer
    let printer_path = Path::new("/dev/usb/lp2");
    let driver = FileDriver::open(printer_path)?;

    let mut printer = Printer::new(driver, Protocol::default(), Some(PrinterOptions::default()));

    printer
        .debug_mode(Some(DebugMode::Hex))
        .init()?;

    //image_with_dither(&mut printer, "/media/user/MISC/Documents/Coding/Rust/thermal-printer-rust/assets/LeeroyChicken.png")?;
    printer
        .feed()?
        .print_cut()?;

    //let _ = image_with_dither(&mut printer, "/media/user/MISC/Documents/Coding/Rust/thermal-printer-rust/assets/fop.png")?;
    //print_test_pattern_line_by_line(&mut printer)?;
    //let _ = move_back(&mut printer, 0xA0);
    //
    //image_with_dither(&mut printer, "/media/user/MISC/Documents/Coding/Rust/thermal-printer-rust/assets/yael.png")?;
    Ok(())

    /* manual test pattern
    driver.write(&[0x1B, b'@'])?;

    let width_bytes: u16 = 384 / 8;  // 48 bytes
    let height: u16 = 24;            // 24 dots high

    let xL = (width_bytes & 0xFF) as u8;
    let xH = (width_bytes >> 8) as u8;
    let yL = (height & 0xFF) as u8;
    let yH = (height >> 8) as u8;

    let m = 0; // normal mode

    // Build the raster command prefix
    let mut cmd = vec![0x1D, 0x76, 0x30, m, xL, xH, yL, yH];

    // Generate a simple test pattern: vertical stripes
    for row in 0..height {
        for byte in 0..width_bytes {
            // Alternate black/white vertical stripes
            let val = if byte % 2 == 0 { 0xFF } else { 0x00 };
            cmd.push(val);
        }
    }

    // Send command to printer
    driver.write(&cmd)?;
    println!("Wrote {:#X?}",&cmd);
    driver.flush()?;


    Ok(())*/
}*/

fn image_with_dither(printer: &mut Printer<FileDriver>, img_path: &str) -> Result<()> {
    let tmp_path = "/tmp/dithered.bmp";

    println!("Running ImageMagick preprocessing...");

    let status = Command::new("convert")
        .args(&[
            img_path,
            "-background", "white", // make transparent pixels white
            "-alpha", "remove",
            "-resize", "384x",            // resize to printer width
            "-modulate", "115",           // brightness
            "-level", "5%,95%",           // gentle contrast
            "-dither", "FloydSteinberg",  // dithering
            "-remap", "pattern:gray50",   // 50% gray pattern
            "-depth", "1",                // 1-bit BMP
            &tmp_path,
        ])
        .status()
        .expect("Failed to execute ImageMagick");

    if !status.success() {
        panic!("ImageMagick convert failed with status: {}", status);
    }

    let option = BitImageOption::new(None, Some(128000), BitImageSize::Normal)?;
    printer.bit_image_option(tmp_path, option)?;
    printer.feed()?;

    Ok(())
}

pub fn print_test_pattern_line_by_line(printer: &mut Printer<FileDriver>) -> Result<()> {
    const WIDTH: u32 = 384;
    const HEIGHT: u32 = 24; // each slice
    const TOTAL_HEIGHT: u32 = 12; // total test pattern height

    for y_offset in (0..TOTAL_HEIGHT).step_by(HEIGHT as usize) {
        let slice_h = HEIGHT.min(TOTAL_HEIGHT - y_offset);

        // Create a new BMP slice
        let mut img = Image::new(WIDTH, slice_h);

        for y in 0..slice_h {
            for x in 0..WIDTH {
                // Alternate black/white vertical stripes for test pattern
                let pixel = if (x / 8) % 2 == 0 {
                    Pixel::new(0, 0, 0) // black
                } else {
                    Pixel::new(255, 255, 255) // white
                };
                img.set_pixel(x, y, pixel);
            }
        }

        // Convert the image to BMP bytes in memory
        let mut bmp_bytes = Vec::new();
        img.to_writer(&mut bmp_bytes)?;

        // Send slice to printer
        printer.bit_image_from_bytes(&bmp_bytes)?;
    }

    printer.custom(&[0x18])?;
    Ok(())
}

fn all_barcodes_test(printer: &mut Printer<FileDriver>) -> Result<()> {
    // smallest possible
    let default_opt = BarcodeOption::new(BarcodeWidth::S, BarcodeHeight::XS, BarcodeFont::default(), BarcodePosition::Above);

    printer
    .ean13_option("1234567890265", default_opt.clone())? //  does work
    .ean8_option("01234565", default_opt.clone())? //  does work
    .upca_option("012345678905", default_opt.clone())? //  does work
    .upce("01234565")? // doesn't work
    .itf_option("30712345000010", default_opt.clone())? //  does work
    .pdf417("Hello World!")? //  doesn't work
    .code39_option("CODE39-123", default_opt.clone())?
    .codabar_option("A1234B", default_opt.clone())?
    .maxi_code_option("MAXICODE123", MaxiCodeMode::default())? //  doesn't work
    .gs1_databar_2d("123456780123")? // needs valid data (whatever that is)
    .qrcode_option(
        "QR123",
        QRCodeOption::new(QRCodeModel::Model1, 1, QRCodeCorrectionLevel::L)
    )?;


    Ok(())
}

fn move_back(printer: &mut Printer<FileDriver>, n: u8) -> Result<()> {
    printer.custom(&[0x1B, 0x6A, n])?;
    Ok(())
}
