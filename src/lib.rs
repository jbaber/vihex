// TODO This is deprecated and should be
// replaced with
//     ec = {package = "edhex_core", version = "0.1.0}
// in Cargo.toml.  But that's only going to
// work for after Rust 1.26.0  Far enough in the future, use the Cargo.toml way.
extern crate edhex_core as ec;

use std::num::NonZeroUsize;
use std::io::Read;
use cursive::views::{
    LinearLayout,
    TextView,
    ResizedView
};


// pub fn byte_numbers(state:&ec::State) -> String {
//     return "".to_owned();
// }

pub fn cargo_version() -> Result<String, String> {
    if let Some(version) = option_env!("CARGO_PKG_VERSION") {
        return Ok(String::from(version));
    }
    return Err("Version unknown (not compiled with cargo)".to_string());
}

pub fn actual_runtime(filename:&str) -> i32 {
    let file = match ec::filehandle(filename) {
        Ok(Some(filehandle)) => {
            Some(filehandle)
        },
        Ok(None) => None,
        Err(error) => {
            println!("Problem opening '{}'", filename);
            println!("{}'", error);
            return 3;
        }
    };

    let original_num_bytes = match ec::num_bytes_or_die(&file) {
        Ok(num_bytes) => {
            num_bytes
        },
        Err(errcode) => {
            return errcode;
        }
    };

    /* Read all bytes into memory */
    // TODO Buffered reading for files significantly bigger than RAM.
    let mut all_bytes = Vec::new();
    if file.is_some() {
        match file.unwrap().read_to_end(&mut all_bytes) {
            Err(_) => {
                println!("Couldn't read {}", filename);
                return 4;
            },
            Ok(num_bytes_read) => {
                if num_bytes_read != original_num_bytes {
                    println!("Only read {} of {} bytes of {}", num_bytes_read,
                            original_num_bytes, filename);
                    return 5;
                }
            }
        }
    }

    let state = ec::State{
        radix: 16,
        filename: filename.to_owned(),
        before_context: 0,
        after_context: 0,
        show_byte_numbers: true,
        show_prompt: false,
        color: true,
        show_chars: true,
        unsaved_changes: false,
        index: 0,
        width: NonZeroUsize::new(16).unwrap(),
        all_bytes: all_bytes,
        // TODO calculate based on longest possible index
        n_padding: "      ".to_owned(),
        last_search: None,
    };

    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());

    let explanation = TextView::new("hjkl or ←↓↑→ to navigate | q to quit").align(cursive::align::Align::top_left());

    /* Addresses for left column starting at 0 */
    let addresses = state.addresses(0);
    let addresses_s = addresses.iter().map(|x|
            ec::address_display(*x, state.radix, "", false))
            .collect::<Vec<String>>().join("\n");
    let line_numbers = TextView::new(addresses_s)
            .align(cursive::align::Align::top_left());

    /* Rows of bytes for middle column */
    let mut middle_column_string = "".to_owned();
    for address in addresses {
        let cur_row_bytes = state.bytes_from(address);
        let mut bytes_row_string = "  ".to_owned();
        for byte in cur_row_bytes {
            bytes_row_string += &ec::padded_byte(*byte);
            bytes_row_string += " ";
        }
        middle_column_string += &bytes_row_string;
        middle_column_string += "\n";
    }

    /* Rows of chars for rightmost column */
    e

    let bytes_display = TextView::new(format!("{}", middle_column_string)).align(cursive::align::Align::top_left());
    let chars_display = TextView::new("|   chars\n").align(cursive::align::Align::top_left());

    let main_layout = LinearLayout::horizontal().child(line_numbers).child(bytes_display).child(chars_display);

    let total_layout = ResizedView::with_full_width(ResizedView::with_full_height(LinearLayout::vertical().child(explanation).child(main_layout)));

    siv.add_layer(total_layout);

    siv.run();

    0
}
