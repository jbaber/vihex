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
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

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

pub fn actual_runtime(filename:&str, color: bool) -> i32 {
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

    let _explanation = "hjkl or ←↓↑→ to navigate | q to quit";

    /* Addresses for left column starting at 0 */
    let addresses = state.addresses(0);
    let addresses_s = addresses.iter().map(|x|
            ec::address_display(*x, state.radix, "", false))
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
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(6),
                        Constraint::Min(2),
                        Constraint::Length(16),
                    ]
                    .as_ref(),
                )
                .split(size);

            let chars = Paragraph::new("pet-CLI 2020 - all rights reserved")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            let addresses = Paragraph::new("Addresses")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            let bytes = Paragraph::new("Bytes")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            rect.render_widget(addresses, chunks[0]);
            rect.render_widget(bytes, chunks[1]);
            rect.render_widget(chars, chunks[2]);
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
