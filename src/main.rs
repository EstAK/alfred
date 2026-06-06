use std::{io::Read, path::PathBuf, process::exit, str::FromStr};

use crossterm::event::{self, KeyCode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListState},
    DefaultTerminal, Frame, TerminalOptions, Viewport,
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let target = std::env::args()
        .nth(1)
        .expect("Please provide a nix template repo");

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(8),
    });

    let app_result = run(&mut terminal, target);

    ratatui::restore();

    app_result
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' || c == '├' || c == '─' || c == '\n' || c == '└' {
            // skip until 'm'
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn run(terminal: &mut DefaultTerminal, target: String) -> color_eyre::Result<()> {
    terminal.clear()?;
    let mut list_state = ListState::default().with_selected(Some(0));
    // let mut buf = String::new();
    // dbg!(&target);
    let output = strip_ansi(&String::from_utf8(
        std::process::Command::new("nix")
            .args(["flake", "show", target.as_str()]) // REVIEW maybe add refresh
            .output()?
            .stdout,
    )?)
    .split("  ")
    .filter_map(|part| {
        if part.trim().is_empty() {
            None
        } else {
            let mut iter = part.trim().split(": template: ").map(str::to_string);
            Some((iter.next()?, iter.next()?))
        }
    })
    .collect::<Vec<(String, String)>>();

    loop {
        terminal.draw(|frame| render(frame, &mut list_state, &output))?;
        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => list_state.select_previous(),
                KeyCode::Enter => {
                    if let Some(selected) = list_state.selected() {
                        let (name, _) = &output[selected];
                        std::process::Command::new("nix")
                            .args([
                                "flake",
                                "init",
                                "-t",
                                format!("{}#{}", target, name).as_str(),
                            ])
                            .status()?;
                        break Ok(());
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, list_state: &mut ListState, items: &Vec<(String, String)>) {
    let constraints = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];
    let layout = Layout::vertical(constraints).spacing(1);
    let [top, first, second] = frame.area().layout(&layout);

    let title = Line::from_iter([
        Span::from("List Widget").bold(),
        Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
    ]);
    frame.render_widget(title.centered(), top);

    render_list(frame, first, list_state, items);
    // render_bottom_list(frame, second);
}

pub fn render_list(
    frame: &mut Frame,
    area: Rect,
    list_state: &mut ListState,
    list_pairs: &Vec<(String, String)>,
) {
    let max_len = list_pairs
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    let items = list_pairs
        .iter()
        .map(|(name, template)| {
            // 2. Pad the name so they all align perfectly
            let padded_name = format!("{:<width$}", name, width = max_len);

            Line::from_iter([
                Span::from(padded_name).style(Style::new()),
                Span::from(": "),
                Span::from(template).style(Style::new()),
            ])
        })
        .collect::<Vec<Line>>();

    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}
