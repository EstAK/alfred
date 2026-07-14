use crossterm::event::{self, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::cursor::MoveUp;
use crossterm::terminal::{Clear, ClearType};
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
        viewport: Viewport::Inline(9),
    });

    let app_result = run(&mut terminal, target);

    cleanup()?;
    ratatui::restore();

    app_result
}

fn cleanup() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();

    execute!(
        stdout,
        MoveUp(9),
        Clear(ClearType::FromCursorDown),
    )?;

    Ok(())
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' || c == '├' || c == '─' || c == '\n' || c == '└' {
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

fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut query_chars = query.chars().map(|c| c.to_ascii_lowercase());
    let mut next_query_char = query_chars.next();
    for tc in target.chars().map(|c| c.to_ascii_lowercase()) {
        if let Some(qc) = next_query_char {
            if tc == qc {
                next_query_char = query_chars.next();
            }
        } else {
            return true;
        }
    }
    next_query_char.is_none()
}

fn run(terminal: &mut DefaultTerminal, target: String) -> color_eyre::Result<()> {
    terminal.clear()?;
    let mut list_state = ListState::default().with_selected(Some(0));
    let mut search_query = String::new();

    let output = strip_ansi(&String::from_utf8(
        std::process::Command::new("nix")
            .args(["flake", "show", target.as_str()])
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
        let filtered: Vec<usize> = output
            .iter()
            .enumerate()
            .filter(|(_, (name, _))| fuzzy_match(&search_query, name))
            .map(|(i, _)| i)
            .collect();

        if filtered.is_empty() {
            list_state.select(None);
        } else if let Some(selected) = list_state.selected() {
            if selected >= filtered.len() {
                list_state.select(Some(filtered.len() - 1));
            }
        }

        terminal.draw(|frame| {
            render(frame, &mut list_state, &output, &filtered, &search_query)
        })?;

        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Down => list_state.select_next(),
                KeyCode::Up => list_state.select_previous(),
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    list_state.select_next();
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    list_state.select_previous();
                }
                KeyCode::Enter => {
                    if let Some(selected) = list_state.selected() {
                        if let Some(&idx) = filtered.get(selected) {
                            let (name, _) = &output[idx];
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
                }
                KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(())
                }
                KeyCode::Esc => break Ok(()),
                KeyCode::Backspace => {
                    search_query.pop();
                    list_state.select(Some(0));
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    search_query.push(c);
                    list_state.select(Some(0));
                }
                _ => {}
            }
        }
    }
}

fn render(
    frame: &mut Frame,
    list_state: &mut ListState,
    items: &[(String, String)],
    filtered: &[usize],
    query: &str,
) {
    let constraints = [Constraint::Length(1), Constraint::Fill(1)];
    let layout = Layout::vertical(constraints).spacing(1);
    let [top, list_area] = frame.area().layout(&layout);

    let title = Line::from_iter([
        Span::from("search: ").fg(Color::DarkGray),
        Span::from(format!("{}_", query)),
    ]);
    frame.render_widget(title.centered(), top);

    render_list(frame, list_area, list_state, items, filtered);
}

pub fn render_list(
    frame: &mut Frame,
    area: Rect,
    list_state: &mut ListState,
    list_pairs: &[(String, String)],
    filtered: &[usize],
) {
    let max_len = filtered
        .iter()
        .map(|&i| list_pairs[i].0.len())
        .max()
        .unwrap_or(0);
    let items = filtered
        .iter()
        .map(|&i| {
            let (name, template) = &list_pairs[i];
            let padded_name = format!("{:<width$}", name, width = max_len);

            Line::from_iter([
                Span::from(padded_name).style(Style::new()),
                Span::from(": "),
                Span::from(template).style(Style::new()),
            ])
        })
        .collect::<Vec<Line>>();

    let list = List::new(items)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}
