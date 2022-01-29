// TODO This is deprecated and should be
// replaced with
//     ec = {package = "edhex_core", version = "0.1.0}
// in Cargo.toml.  But that's only going to
// work for after Rust 1.26.0  Far enough in the future, use the Cargo.toml way.
extern crate edhex_core as ec;

use std::num::NonZeroUsize;
use std::io::Read;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
// use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::backend::CrosstermBackend;
use tui::layout::Alignment;
use tui::layout::Constraint;
use tui::layout::Direction;
use tui::layout:: Layout;
use tui::style::Color;
use tui::style::Style;
use tui::Terminal;
use tui::widgets::Block;
use tui::widgets::Borders;
use tui::widgets::BorderType;
use tui::widgets::Paragraph;

enum Event<I> {
    Input(I),
    Tick,
}


pub fn cargo_version() -> Result<String, String> {
    if let Some(version) = option_env!("CARGO_PKG_VERSION") {
        return Ok(String::from(version));
    }
    return Err("Version unknown (not compiled with cargo)".to_string());
}

pub fn actual_runtime(filename:&str, color: bool, readonly:bool,
        prefs_path: PathBuf, state_path: PathBuf) -> i32 {
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

    let default_prefs = ec::Preferences {
        color: color,
        ..ec::Preferences::default()
    };


    /* Use a state file if one is present */
    let maybe_state = ec::State::read_from_path(&state_path);
    let mut state = if maybe_state.is_ok() {
        maybe_state.unwrap()
    }
    else {
        ec::State {
            prefs: default_prefs,
            unsaved_changes: (filename == ""),
            filename: filename.to_owned(),
            readonly: readonly,
            index: 0,
            all_bytes: if filename == "" {
                Vec::new()
            }
            else {
                let maybe_all_bytes = ec::all_bytes_from_filename(filename);
                if maybe_all_bytes.is_ok() {
                    maybe_all_bytes.unwrap()
                }
                else {
                    match maybe_all_bytes {
                        Err(ec::AllBytesFromFilenameError::NotARegularFile) => {
                            println!("{} is not a regular file", filename);
                            return 1;
                        },
                        Err(ec::AllBytesFromFilenameError::FileDoesNotExist) => {
                            Vec::new()
                        },
                        _ => {
                            println!("Cannot read {}", filename);
                            return 1;
                        }
                    }
                }
            },
            last_search: None,
        }
    };

    let _explanation = "hjkl or ←↓↑→ to navigate | q to quit";

    /* Addresses for left column starting at 0 */
    let addresses = state.addresses(0);
    let addresses_s = addresses.iter().map(|x|
            ec::address_display(*x, state.prefs.radix, "", false))
            .collect::<Vec<String>>().join("\n");

    /* Rows of bytes for middle column */
    let mut middle_column_string = "".to_owned();

    /* Rows of chars for last column */
    let mut last_column_string = "".to_owned();

    for address in addresses {
        let cur_row_bytes = state.bytes_from(address);
        let mut bytes_row_string = "  ".to_owned();
        let mut chars_row_string = "  ".to_owned();
        for byte in cur_row_bytes {
            bytes_row_string += &ec::padded_byte(*byte);
            bytes_row_string += " ";
            chars_row_string += &String::from(ec::chared_byte(*byte));
        }
        middle_column_string += &bytes_row_string;
        middle_column_string += "\n";
        last_column_string   += &chars_row_string;
        last_column_string   += "\n";
    }

    /* Beginning of tui-crossterm boilerplate */
    enable_raw_mode().expect("Cannot run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);

    /* input loop */
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events"
) {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend);
    if terminal.is_err() {
        println!("Could not render terminal");
        return 1;
    }
    let mut terminal = terminal.unwrap();
    if terminal.clear().is_err() {
        println!("Could not clear terminal");
        return 0;
    }

    /* End of tui-crossterm boilerplate */

    /* render loop */
    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(100),
                    ]
                    .as_ref(),
                )
                .split(size);

            let everything = Paragraph::new(format!("{:?}", state.bytes_from(0)))
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                .title(filename)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            rect.render_widget(everything, chunks[0]);
        });

        let received = rx.recv();
        if received.is_err() {
            println!("Failed to parse input");
            return 3;
        }
        let received = received.unwrap();
        match received {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    if disable_raw_mode().is_err() || terminal.show_cursor().is_err() {
                        println!("Couldn't clean up terminal before quitting.");
                    }
                    break;
                }
                KeyCode::Char('h') => {},
                KeyCode::Char('p') => {},
                _ => {}
            },
            Event::Tick => {}
        }
    }

    0
}
