use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
use ratatui::widgets::{Block, Borders, BorderType, Row, Cell, Table, TableState, Paragraph};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::style::{Style, Color, Modifier};
use ratatui::text::{Span, Line, Text};

use crate::graph::DependencyGraph;
use crate::cli::format_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiSortColumn {
    Name,
    Size,
    Transitive,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivePanel {
    Table,
    Details,
}

struct TuiApp<'a> {
    graph: &'a DependencyGraph,
    filtered_indices: Vec<usize>,
    table_state: TableState,
    search_query: String,
    search_mode: bool,
    sort_column: TuiSortColumn,
    sort_direction: SortDirection,
    active_panel: ActivePanel,
    details_scroll: usize,
    selected_dep_idx: usize,
    should_quit: bool,
}

fn filter_and_sort(
    graph: &DependencyGraph,
    search: &str,
    sort_col: TuiSortColumn,
    sort_dir: SortDirection,
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..graph.packages.len()).collect();

    // Filter
    if !search.is_empty() {
        let search_lower = search.to_lowercase();
        indices.retain(|&idx| {
            graph.packages[idx].name.to_lowercase().contains(&search_lower)
        });
    }

    // Sort
    indices.sort_by(|&a, &b| {
        let pkg_a = &graph.packages[a];
        let pkg_b = &graph.packages[b];

        let cmp = match sort_col {
            TuiSortColumn::Name => pkg_a.name.cmp(&pkg_b.name),
            TuiSortColumn::Size => pkg_a.installsize.cmp(&pkg_b.installsize),
            TuiSortColumn::Transitive => pkg_a.transitive_size.cmp(&pkg_b.transitive_size),
            TuiSortColumn::Exclusive => pkg_a.exclusive_size.cmp(&pkg_b.exclusive_size),
        };

        match sort_dir {
            SortDirection::Ascending => cmp,
            SortDirection::Descending => cmp.reverse(),
        }
    });

    indices
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}

impl<'a> TuiApp<'a> {
    fn new(graph: &'a DependencyGraph) -> Self {
        let mut app = Self {
            graph,
            filtered_indices: Vec::new(),
            table_state: TableState::default(),
            search_query: String::new(),
            search_mode: false,
            sort_column: TuiSortColumn::Exclusive,
            sort_direction: SortDirection::Descending,
            active_panel: ActivePanel::Table,
            details_scroll: 0,
            selected_dep_idx: 0,
            should_quit: false,
        };
        app.update_filter();
        app
    }

    fn update_filter(&mut self) {
        let old_selected = self.table_state.selected();
        let selected_pkg_name = old_selected
            .and_then(|idx| self.filtered_indices.get(idx))
            .map(|&idx| &self.graph.packages[idx].name);

        self.filtered_indices = filter_and_sort(
            self.graph,
            &self.search_query,
            self.sort_column,
            self.sort_direction,
        );

        if let Some(name) = selected_pkg_name {
            if let Some(new_idx) = self.filtered_indices.iter().position(|&idx| &self.graph.packages[idx].name == name) {
                self.table_state.select(Some(new_idx));
            } else if !self.filtered_indices.is_empty() {
                self.table_state.select(Some(0));
            } else {
                self.table_state.select(None);
            }
        } else if !self.filtered_indices.is_empty() {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
        self.details_scroll = 0;
        self.selected_dep_idx = 0;
    }

    fn select_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.filtered_indices.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.details_scroll = 0;
        self.selected_dep_idx = 0;
    }

    fn select_prev(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_indices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.details_scroll = 0;
        self.selected_dep_idx = 0;
    }

    fn select_page_down(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                let next = i + 15;
                if next >= self.filtered_indices.len() {
                    self.filtered_indices.len() - 1
                } else {
                    next
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.details_scroll = 0;
        self.selected_dep_idx = 0;
    }

    fn select_page_up(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i < 15 {
                    0
                } else {
                    i - 15
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.details_scroll = 0;
        self.selected_dep_idx = 0;
    }

    fn select_first(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.table_state.select(Some(0));
            self.details_scroll = 0;
            self.selected_dep_idx = 0;
        }
    }

    fn select_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.table_state.select(Some(self.filtered_indices.len() - 1));
            self.details_scroll = 0;
            self.selected_dep_idx = 0;
        }
    }

    fn set_sort_column(&mut self, col: TuiSortColumn) {
        if self.sort_column == col {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = col;
            self.sort_direction = match col {
                TuiSortColumn::Name => SortDirection::Ascending,
                _ => SortDirection::Descending,
            };
        }
        self.update_filter();
    }
}

fn get_sort_indicator(col: TuiSortColumn, active: TuiSortColumn, dir: SortDirection) -> &'static str {
    if col == active {
        match dir {
            SortDirection::Ascending => "▲",
            SortDirection::Descending => "▼",
        }
    } else {
        ""
    }
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Length(3), // Search Bar
            Constraint::Min(1),    // Main panel
            Constraint::Length(1), // Footer
        ])
        .split(f.size());

    // 1. Draw Header
    let total_size: u64 = app.graph.packages.iter().map(|p| p.installsize).sum();
    let total_pkgs = app.graph.packages.len();
    
    let header_text = format!(
        " pkglist | Packages: {} | Total Installed Size: {}",
        total_pkgs,
        format_bytes(total_size)
    );
    let header = Paragraph::new(Span::raw(header_text))
        .style(Style::default().bg(Color::Rgb(30, 41, 59)).fg(Color::White).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // 2. Draw Search Bar
    let search_border_color = if app.search_mode { Color::Yellow } else { Color::DarkGray };
    let search_title = if app.search_mode { " Search (Type name, Esc to clear & close | Enter/Tab to close) " } else { " Search (Press [f] to search) " };
    
    let search_text = if app.search_query.is_empty() && !app.search_mode {
        Paragraph::new(Span::styled("Type here to filter packages...", Style::default().fg(Color::DarkGray)))
    } else if app.search_mode {
        Paragraph::new(Span::raw(format!("{}|", app.search_query)))
    } else {
        Paragraph::new(Span::raw(app.search_query.clone()))
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(search_border_color))
        .title(search_title);
    
    f.render_widget(search_text.block(search_block), chunks[1]);

    // 3. Draw Main Panel
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(chunks[2]);

    let selected_pkg = app.table_state.selected().map(|idx| app.filtered_indices[idx]);
    
    // Draw Package Table
    let table_border_color = if app.active_panel == ActivePanel::Table { Color::Cyan } else { Color::DarkGray };
    let table_title = format!(" Packages ({}) ", app.filtered_indices.len());
    
    let table_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(table_border_color))
        .title(table_title);

    let header_cells = [
        Cell::from(format!("Package {}", get_sort_indicator(TuiSortColumn::Name, app.sort_column, app.sort_direction))),
        Cell::from(format!("Size {}", get_sort_indicator(TuiSortColumn::Size, app.sort_column, app.sort_direction))),
        Cell::from(format!("Transitive {}", get_sort_indicator(TuiSortColumn::Transitive, app.sort_column, app.sort_direction))),
        Cell::from(format!("Saved {}", get_sort_indicator(TuiSortColumn::Exclusive, app.sort_column, app.sort_direction))),
    ];
    let header_row = Row::new(header_cells)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app.filtered_indices
        .iter()
        .map(|&idx| {
            let pkg = &app.graph.packages[idx];
            let cells = [
                Cell::from(pkg.name.clone()).style(Style::default().fg(Color::White)),
                Cell::from(format_bytes(pkg.installsize)).style(Style::default().fg(Color::Green)),
                Cell::from(format_bytes(pkg.transitive_size)).style(Style::default().fg(Color::Yellow)),
                Cell::from(format_bytes(pkg.exclusive_size)).style(Style::default().fg(Color::Magenta)),
            ];
            Row::new(cells)
        })
        .collect();

    let table = Table::new(rows, [
        Constraint::Percentage(43),
        Constraint::Percentage(19),
        Constraint::Percentage(19),
        Constraint::Percentage(19),
    ])
    .header(header_row)
    .block(table_block)
    .highlight_style(Style::default().bg(Color::Rgb(30, 58, 138)).fg(Color::White).add_modifier(Modifier::BOLD))
    .highlight_symbol("» ");

    f.render_stateful_widget(table, main_chunks[0], &mut app.table_state);

    // Draw Details Panel
    let details_border_color = if app.active_panel == ActivePanel::Details { Color::Cyan } else { Color::DarkGray };
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(details_border_color))
        .title(" Package Details ");

    if let Some(pkg_idx) = selected_pkg {
        let pkg = &app.graph.packages[pkg_idx];
        let content_width = (main_chunks[1].width as usize).saturating_sub(4);
        
        // Clamp selected_dep_idx globally based on total selectable items (dependencies + dependents)
        let num_deps = pkg.dependencies.len();
        let num_reqs = pkg.dependents.len();
        let total_items = num_deps + num_reqs;
        if total_items > 0 {
            app.selected_dep_idx = app.selected_dep_idx.min(total_items - 1);
        } else {
            app.selected_dep_idx = 0;
        }
        
        let mut detail_lines = Vec::new();
        
        detail_lines.push(Line::from(vec![
            Span::raw("Package: "),
            Span::styled(&pkg.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        
        detail_lines.push(Line::from(vec![
            Span::raw("Version: "),
            Span::raw(format!("{}-{} ({})", pkg.version, pkg.release, pkg.arch)),
        ]));
        
        detail_lines.push(Line::from(""));
        
        detail_lines.push(Line::from(vec![
            Span::raw("Installed Size: "),
            Span::styled(format_bytes(pkg.installsize), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        detail_lines.push(Line::from(vec![
            Span::raw("Total Deps Size: "),
            Span::styled(format_bytes(pkg.transitive_size), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        detail_lines.push(Line::from(vec![
            Span::raw("Saved if Removed: "),
            Span::styled(format_bytes(pkg.exclusive_size), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]));
        
        detail_lines.push(Line::from(""));
        
        detail_lines.push(Line::from(Span::styled("Summary:", Style::default().add_modifier(Modifier::UNDERLINED))));
        for wl in wrap_text(&pkg.summary, content_width) {
            detail_lines.push(Line::from(format!("  {}", wl)));
        }
        
        detail_lines.push(Line::from(""));
        
        detail_lines.push(Line::from(Span::styled("Description:", Style::default().add_modifier(Modifier::UNDERLINED))));
        for wl in wrap_text(&pkg.description, content_width) {
            detail_lines.push(Line::from(format!("  {}", wl)));
        }
        
        detail_lines.push(Line::from(""));
        
        detail_lines.push(Line::from(Span::styled(
            format!("Direct Dependencies ({}):", pkg.dependencies.len()),
            Style::default().add_modifier(Modifier::UNDERLINED)
        )));
        if pkg.dependencies.is_empty() {
            detail_lines.push(Line::from("  (none)"));
        } else {
            let mut deps = pkg.dependencies.clone();
            deps.sort_by_key(|&d| std::cmp::Reverse(app.graph.packages[d].installsize));
            
            for (idx, &d) in deps.iter().enumerate() {
                let dep = &app.graph.packages[d];
                let is_selected = app.active_panel == ActivePanel::Details && app.selected_dep_idx == idx;
                
                let prefix = if is_selected { "» " } else { "- " };
                let mut style = Style::default();
                if is_selected {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                } else {
                    style = style.fg(Color::Cyan);
                }
                
                detail_lines.push(Line::from(vec![
                    Span::styled(prefix, if is_selected { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::styled(&dep.name, style),
                    Span::raw(format!(" ({})", format_bytes(dep.installsize))),
                ]));
            }
        }
        
        detail_lines.push(Line::from(""));
        
        detail_lines.push(Line::from(Span::styled(
            format!("Direct Dependents ({}):", pkg.dependents.len()),
            Style::default().add_modifier(Modifier::UNDERLINED)
        )));
        if pkg.dependents.is_empty() {
            detail_lines.push(Line::from("  (none)"));
        } else {
            let mut reqs = pkg.dependents.clone();
            reqs.sort_by_key(|&r| std::cmp::Reverse(app.graph.packages[r].installsize));
            
            for (idx, &r) in reqs.iter().enumerate() {
                let req = &app.graph.packages[r];
                let global_idx = num_deps + idx;
                let is_selected = app.active_panel == ActivePanel::Details && app.selected_dep_idx == global_idx;
                
                let prefix = if is_selected { "» " } else { "- " };
                let mut style = Style::default();
                if is_selected {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                } else {
                    style = style.fg(Color::Cyan);
                }
                
                detail_lines.push(Line::from(vec![
                    Span::styled(prefix, if is_selected { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::styled(&req.name, style),
                    Span::raw(format!(" ({})", format_bytes(req.installsize))),
                ]));
            }
        }

        let total_lines = detail_lines.len();
        let height = main_chunks[1].height as usize;
        let visible_height = height.saturating_sub(2);
        
        app.details_scroll = app.details_scroll.min(total_lines.saturating_sub(visible_height));

        let paragraph = Paragraph::new(Text::from(detail_lines))
            .block(details_block)
            .scroll((app.details_scroll as u16, 0));
        
        f.render_widget(paragraph, main_chunks[1]);
    } else {
        let empty_paragraph = Paragraph::new("No package selected.")
            .block(details_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty_paragraph, main_chunks[1]);
    }

    // 4. Draw Footer
    let help_text = if app.search_mode {
        " [Esc] Cancel & Clear | [Enter/Tab] Close | [Backspace] Delete "
    } else {
        " [q] Quit | [f] Search | [1-4] Sort Columns | [WASD/Arrows] Scroll / Switch Panels "
    };
    let footer = Paragraph::new(Span::raw(help_text))
        .style(Style::default().bg(Color::Rgb(30, 41, 59)).fg(Color::White));
    f.render_widget(footer, chunks[3]);
}

pub fn run_tui(graph: &DependencyGraph) -> Result<(), String> {
    crossterm::terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )
    .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend).map_err(|e| format!("Failed to initialize terminal: {}", e))?;

    let mut app = TuiApp::new(graph);

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    crossterm::terminal::disable_raw_mode().ok();
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )
    .ok();

    res
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut TuiApp,
) -> Result<(), String> {
    while !app.should_quit {
        terminal.draw(|f| draw_ui(f, app)).map_err(|e| format!("Failed to draw TUI: {}", e))?;

        if event::poll(std::time::Duration::from_millis(100)).map_err(|e| format!("Poll error: {}", e))? {
            if let Event::Key(key) = event::read().map_err(|e| format!("Read event error: {}", e))? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                let selected_pkg = app.table_state.selected().and_then(|idx| app.filtered_indices.get(idx).copied());

                if app.search_mode {
                    match key.code {
                        KeyCode::Enter | KeyCode::Tab => {
                            app.search_mode = false;
                        }
                        KeyCode::Esc => {
                            app.search_mode = false;
                            app.search_query.clear();
                            app.update_filter();
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.update_filter();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.update_filter();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('f') => {
                            app.search_mode = true;
                        }
                        KeyCode::Esc => {
                            if !app.search_query.is_empty() {
                                app.search_query.clear();
                                app.update_filter();
                            }
                        }
                        KeyCode::Tab => {
                            app.active_panel = match app.active_panel {
                                ActivePanel::Table => ActivePanel::Details,
                                ActivePanel::Details => ActivePanel::Table,
                            };
                        }
                        KeyCode::Char('1') => app.set_sort_column(TuiSortColumn::Name),
                        KeyCode::Char('2') => app.set_sort_column(TuiSortColumn::Size),
                        KeyCode::Char('3') => app.set_sort_column(TuiSortColumn::Transitive),
                        KeyCode::Char('4') => app.set_sort_column(TuiSortColumn::Exclusive),
                        KeyCode::Enter => {
                            if app.active_panel == ActivePanel::Details {
                                if let Some(pkg_idx) = selected_pkg {
                                    let pkg = &app.graph.packages[pkg_idx];
                                    let mut deps = pkg.dependencies.clone();
                                    deps.sort_by_key(|&d| std::cmp::Reverse(app.graph.packages[d].installsize));
                                    
                                    let mut reqs = pkg.dependents.clone();
                                    reqs.sort_by_key(|&r| std::cmp::Reverse(app.graph.packages[r].installsize));
                                    
                                    let num_deps = deps.len();
                                    let num_reqs = reqs.len();
                                    let total_items = num_deps + num_reqs;
                                    
                                    if total_items > 0 && app.selected_dep_idx < total_items {
                                        let target_idx = if app.selected_dep_idx < num_deps {
                                            deps[app.selected_dep_idx]
                                        } else {
                                            reqs[app.selected_dep_idx - num_deps]
                                        };
                                        
                                        let mut found_pos = app.filtered_indices.iter().position(|&idx| idx == target_idx);
                                        
                                        if found_pos.is_none() {
                                            app.search_query.clear();
                                            app.update_filter();
                                            found_pos = app.filtered_indices.iter().position(|&idx| idx == target_idx);
                                        }
                                        
                                        if let Some(pos) = found_pos {
                                            app.table_state.select(Some(pos));
                                            app.active_panel = ActivePanel::Table;
                                            app.details_scroll = 0;
                                            app.selected_dep_idx = 0;
                                        }
                                    }
                                }
                            }
                        }
                        
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_prev(),
                                ActivePanel::Details => {
                                    if let Some(pkg_idx) = selected_pkg {
                                        let num_deps = app.graph.packages[pkg_idx].dependencies.len();
                                        let num_reqs = app.graph.packages[pkg_idx].dependents.len();
                                        let total_items = num_deps + num_reqs;
                                        if total_items > 0 {
                                            if app.selected_dep_idx == 0 {
                                                app.selected_dep_idx = total_items - 1;
                                            } else {
                                                app.selected_dep_idx -= 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_next(),
                                ActivePanel::Details => {
                                    if let Some(pkg_idx) = selected_pkg {
                                        let num_deps = app.graph.packages[pkg_idx].dependencies.len();
                                        let num_reqs = app.graph.packages[pkg_idx].dependents.len();
                                        let total_items = num_deps + num_reqs;
                                        if total_items > 0 {
                                            if app.selected_dep_idx >= total_items - 1 {
                                                app.selected_dep_idx = 0;
                                            } else {
                                                app.selected_dep_idx += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.active_panel = ActivePanel::Table;
                        }
                        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                            app.active_panel = ActivePanel::Details;
                        }
                        KeyCode::PageUp => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_page_up(),
                                ActivePanel::Details => app.details_scroll = app.details_scroll.saturating_sub(10),
                            }
                        }
                        KeyCode::PageDown => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_page_down(),
                                ActivePanel::Details => app.details_scroll = app.details_scroll.saturating_add(10),
                            }
                        }
                        KeyCode::Home => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_first(),
                                ActivePanel::Details => app.details_scroll = 0,
                            }
                        }
                        KeyCode::End => {
                            match app.active_panel {
                                ActivePanel::Table => app.select_last(),
                                ActivePanel::Details => {
                                    app.details_scroll = usize::MAX;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
