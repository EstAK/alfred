use clap::{CommandFactory, Parser};
use crossterm::cursor::MoveUp;
use crossterm::event::{self, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use ratatui::{
    DefaultTerminal, Frame, TerminalOptions, Viewport,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListState},
};

#[derive(Parser, Debug)]
#[command(
    name = "alfred",
    about = "A cli for selecting and initializing nix templates",
    version
)]
struct Cli {
    /// Source of the templates (e.g. flake reference like github:nixos/templates or .)
    #[arg(value_name = "SOURCE", conflicts_with = "source_flag")]
    source: Option<String>,

    /// Name of the template to initialize (enables non-interactive mode)
    #[arg(value_name = "TEMPLATE", conflicts_with = "template_flag")]
    template: Option<String>,

    /// Source of the templates (flake reference)
    #[arg(short = 's', long = "source", value_name = "SOURCE")]
    source_flag: Option<String>,

    /// Name of the template to initialize
    #[arg(
        short = 't',
        long = "template",
        visible_alias = "name",
        value_name = "TEMPLATE"
    )]
    template_flag: Option<String>,

    /// Run in non-interactive mode
    #[arg(short = 'n', long = "non-interactive")]
    non_interactive: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();

    let source = match args.source.or(args.source_flag) {
        Some(s) => s,
        None => {
            let mut cmd = Cli::command();
            cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "the following required arguments were not provided:\n  <SOURCE> or --source <SOURCE>",
            )
            .exit();
        }
    };

    let template = args.template.or(args.template_flag);

    if args.non_interactive && template.is_none() {
        let mut cmd = Cli::command();
        cmd.error(
            clap::error::ErrorKind::MissingRequiredArgument,
            "template name must be provided in non-interactive mode (specify <TEMPLATE> or --template <TEMPLATE>)",
        )
        .exit();
    }

    if let Some(template_name) = template {
        init_template(&source, &template_name)?;
    } else {
        let mut terminal = ratatui::init_with_options(TerminalOptions {
            viewport: Viewport::Inline(9),
        });

        let app_result = run(&mut terminal, &source);

        cleanup()?;
        ratatui::restore();

        if let Some(template_name) = app_result? {
            init_template(&source, &template_name)?;
        }
    }

    Ok(())
}

fn init_template(target: &str, name: &str) -> color_eyre::Result<()> {
    let status = std::process::Command::new("nix")
        .args([
            "flake",
            "init",
            "-t",
            format!("{}#{}", target, name).as_str(),
        ])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn cleanup() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();

    execute!(stdout, MoveUp(9), Clear(ClearType::FromCursorDown),)?;

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

fn run(terminal: &mut DefaultTerminal, target: &str) -> color_eyre::Result<Option<String>> {
    terminal.clear()?;
    let mut list_state = ListState::default().with_selected(Some(0));
    let mut search_query = String::new();

    let output = strip_ansi(&String::from_utf8(
        std::process::Command::new("nix")
            .args(["flake", "show", target])
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

        terminal.draw(|frame| render(frame, &mut list_state, &output, &filtered, &search_query))?;

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
                            break Ok(Some(name.clone()));
                        }
                    }
                }
                KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Esc => break Ok(None),
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
    let area = frame.area();
    let constraints = [Constraint::Fill(1), Constraint::Length(1)];
    let layout = Layout::vertical(constraints);
    let [list_area, input_area] = area.layout(&layout);

    let prompt = Line::from_iter([
        Span::from("> ").fg(Color::Green),
        Span::from(format!("{}_", query)),
    ]);
    frame.render_widget(prompt, input_area);

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
