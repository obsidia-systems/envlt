//! Interactive, read-only terminal adapter for an envlt Vault.
//!
//! This crate deliberately delegates Vault access to [`envlt_core::AppService`]
//! and keeps only presentation state. It never persists or renders Variable
//! values in the initial TUI surface.

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use envlt_core::{AppService, VarType, VariableView};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Styled, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// The Project and Environment selected by the CLI before the TUI starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiContext {
    project_name: Option<String>,
    environment_name: Option<String>,
}

impl TuiContext {
    /// Creates a context that opens a specific Project Environment.
    pub fn project(project_name: String, environment_name: String) -> Self {
        Self {
            project_name: Some(project_name),
            environment_name: Some(environment_name),
        }
    }

    /// Creates the context that opens the Vault's Project list.
    pub fn project_list() -> Self {
        Self {
            project_name: None,
            environment_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VariableRow {
    key: String,
    var_type: VarType,
    updated_at: String,
}

impl VariableRow {
    fn type_label(&self) -> &'static str {
        match self.var_type {
            VarType::Secret => "Secret",
            VarType::Plain => "Plain",
        }
    }
}

impl From<VariableView> for VariableRow {
    fn from(variable: VariableView) -> Self {
        Self {
            key: variable.key,
            var_type: variable.var_type,
            updated_at: variable.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct App {
    context: TuiContext,
    projects: Vec<String>,
    rows: Vec<VariableRow>,
    selected: usize,
    screen: Screen,
    should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Projects,
    Variables,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppEvent {
    None,
    OpenProject(String),
}

impl App {
    fn projects(projects: Vec<String>) -> Self {
        Self {
            context: TuiContext::project_list(),
            projects,
            rows: Vec::new(),
            selected: 0,
            screen: Screen::Projects,
            should_quit: false,
        }
    }

    fn variables(context: TuiContext, rows: Vec<VariableRow>) -> Self {
        Self {
            context,
            projects: Vec::new(),
            rows,
            selected: 0,
            screen: Screen::Variables,
            should_quit: false,
        }
    }

    fn selection_len(&self) -> usize {
        match self.screen {
            Screen::Projects => self.projects.len(),
            Screen::Variables => self.rows.len(),
        }
    }

    fn select_next(&mut self) {
        let selection_len = self.selection_len();
        if selection_len != 0 {
            self.selected = (self.selected + 1) % selection_len;
        }
    }

    fn select_previous(&mut self) {
        let selection_len = self.selection_len();
        if selection_len != 0 {
            self.selected = self.selected.checked_sub(1).unwrap_or(selection_len - 1);
        }
    }

    fn handle_event(&mut self, event: Event) -> AppEvent {
        let Event::Key(key) = event else {
            return AppEvent::None;
        };

        if key.kind != KeyEventKind::Press {
            return AppEvent::None;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if self.screen == Screen::Projects => self.should_quit = true,
            KeyCode::Esc => {
                self.screen = Screen::Projects;
                self.selected = 0;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Enter if self.screen == Screen::Projects => {
                return self
                    .projects
                    .get(self.selected)
                    .cloned()
                    .map_or(AppEvent::None, AppEvent::OpenProject);
            }
            _ => {}
        }

        AppEvent::None
    }
}

/// Runs the read-only TUI for the selected Project and Environment.
///
/// # Errors
///
/// Returns an error when the Vault cannot be read or the terminal cannot be
/// initialized, drawn, or restored.
pub fn run(service: &AppService, context: TuiContext, passphrase: &str) -> Result<()> {
    let TuiContext {
        project_name,
        environment_name,
    } = context;
    let mut app = match (project_name, environment_name) {
        (Some(project_name), Some(environment_name)) => {
            let rows = load_rows(service, &project_name, &environment_name, passphrase)?;
            App::variables(TuiContext::project(project_name, environment_name), rows)
        }
        (None, None) => App::projects(
            service
                .list_projects(passphrase)?
                .into_iter()
                .map(|project| project.name)
                .collect(),
        ),
        _ => {
            return Err(anyhow!(
                "TUI context must include both a Project and an Environment"
            ))
        }
    };

    ratatui::run(|terminal| run_app(terminal, &mut app, service, passphrase))?;
    Ok(())
}

fn load_rows(
    service: &AppService,
    project_name: &str,
    environment_name: &str,
    passphrase: &str,
) -> Result<Vec<VariableRow>> {
    Ok(service
        .project_variable_views(project_name, environment_name, passphrase)?
        .into_iter()
        .map(VariableRow::from)
        .collect())
}

fn open_project(
    service: &AppService,
    app: &mut App,
    project_name: String,
    passphrase: &str,
) -> Result<()> {
    let environment_name = service
        .list_environments(&project_name, passphrase)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("project '{project_name}' has no environments"))?;
    let rows = load_rows(service, &project_name, &environment_name, passphrase)?;

    app.context = TuiContext::project(project_name, environment_name);
    app.rows = rows;
    app.selected = 0;
    app.screen = Screen::Variables;
    Ok(())
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    service: &AppService,
    passphrase: &str,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| render(frame, app))?;
        if let AppEvent::OpenProject(project_name) = app.handle_event(event::read()?) {
            open_project(service, app, project_name, passphrase)?;
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Projects => render_projects(frame, app),
        Screen::Variables => render_variables(frame, app),
    }
}

fn render_projects(frame: &mut Frame, app: &App) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let header = Paragraph::new(vec![
        Line::from("envlt vault").bold(),
        Line::from("Projects"),
    ])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, header_area);

    if app.projects.is_empty() {
        let empty =
            Paragraph::new("No projects in this Vault. Use `envlt add <project>` to import one.")
                .block(Block::default().borders(Borders::ALL).title("Projects"));
        frame.render_widget(empty, list_area);
    } else {
        let items = app
            .projects
            .iter()
            .map(|project| ListItem::new(project.as_str()));
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Projects"))
            .highlight_style(Style::default().bg(Color::DarkGray));
        let mut state = ListState::default();
        state.select(Some(app.selected));
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    frame.render_widget(
        Paragraph::new("↑/k ↓/j navigate   Enter open project   q/Esc quit"),
        footer_area,
    );
}

fn render_variables(frame: &mut Frame, app: &App) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let project_name = app.context.project_name.as_deref().unwrap_or_default();
    let environment_name = app.context.environment_name.as_deref().unwrap_or_default();
    let header = Paragraph::new(vec![
        Line::from("envlt vault").bold(),
        Line::from(format!(
            "Project: {project_name}   Environment: {environment_name}"
        )),
    ])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, header_area);

    if app.rows.is_empty() {
        let empty = Paragraph::new("No variables in this environment.")
            .block(Block::default().borders(Borders::ALL).title("Variables"));
        frame.render_widget(empty, list_area);
    } else {
        let items = app.rows.iter().map(|row| {
            let type_style = match row.var_type {
                VarType::Secret => Style::default().fg(Color::Yellow),
                VarType::Plain => Style::default().fg(Color::Cyan),
            };
            ListItem::new(Line::from(vec![
                row.key.clone().bold(),
                format!("  {:<6}", row.type_label()).set_style(type_style),
                format!("  {}", row.updated_at).into(),
            ]))
        });
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Variables"))
            .highlight_style(Style::default().bg(Color::DarkGray));
        let mut state = ListState::default();
        state.select(Some(app.selected));
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    frame.render_widget(
        Paragraph::new("↑/k ↓/j navigate   Esc projects   q quit   Secret values are never shown"),
        footer_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn app_with_rows(row_count: usize) -> App {
        let rows = (0..row_count)
            .map(|index| VariableRow {
                key: format!("KEY_{index}"),
                var_type: VarType::Secret,
                updated_at: "2026-08-04 00:00 UTC".to_owned(),
            })
            .collect();
        App::variables(
            TuiContext::project("api".to_owned(), "local".to_owned()),
            rows,
        )
    }

    #[test]
    fn navigation_wraps_without_rows() {
        let mut app = app_with_rows(0);

        app.select_next();
        app.select_previous();

        assert_eq!(app.selected, 0);
    }

    #[test]
    fn navigation_wraps_at_list_boundaries() {
        let mut app = app_with_rows(2);

        app.select_previous();
        app.select_next();

        assert_eq!(app.selected, 0);
    }

    #[test]
    fn project_list_opens_the_selected_project() {
        let mut app = App::projects(vec!["api".to_owned(), "web".to_owned()]);
        app.select_next();

        let event = app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));

        assert_eq!(event, AppEvent::OpenProject("web".to_owned()));
    }

    #[test]
    fn render_does_not_contain_variable_values() {
        let row = VariableRow::from(VariableView {
            key: "DATABASE_URL".to_owned(),
            value: "example-secret".to_owned(),
            var_type: VarType::Secret,
            updated_at: chrono::Utc::now(),
        });
        let app = App::variables(
            TuiContext::project("api".to_owned(), "local".to_owned()),
            vec![row],
        );
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("test backend initializes");

        terminal
            .draw(|frame| render(frame, &app))
            .expect("render succeeds");

        let buffer = terminal.backend().buffer();
        let output = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!output.contains("example-secret"));
    }
}
