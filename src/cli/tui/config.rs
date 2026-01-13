//! TUI Configuration Interface
//!
//! Provides a menuconfig-style TUI for configuring zigroot projects.
//!
//! **Validates: Requirements 25.1-25.17**

use std::collections::HashSet;
use std::io::{self, Stdout};
use std::path::Path;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::core::config::{
    get_available_packages, get_package_dependencies, get_package_dependents,
    load_manifest_for_config, ConfigCategory,
};
use crate::core::manifest::{Manifest, PackageRef};

/// TUI Application state
pub struct ConfigTui {
    /// Current manifest
    manifest: Manifest,
    /// Project directory
    project_dir: std::path::PathBuf,
    /// Whether changes have been made
    has_changes: bool,
    /// Current view mode
    view_mode: ViewMode,
    /// Selected category (used for state tracking)
    #[allow(dead_code)]
    selected_category: ConfigCategory,
    /// Category list state
    category_state: ListState,
    /// Available packages
    available_packages: Vec<PackageInfo>,
    /// Package list state
    package_state: ListState,
    /// Selected packages (names)
    selected_packages: HashSet<String>,
    /// Build options
    build_options: Vec<BuildOption>,
    /// Build option list state
    build_option_state: ListState,
    /// Currently editing option index
    editing_option: Option<usize>,
    /// Edit buffer for text input
    edit_buffer: String,
    /// Show diff before save
    show_diff: bool,
    /// Pending changes diff
    pending_diff: Vec<String>,
    /// Warning message to display
    warning_message: Option<String>,
    /// Focus area
    focus: FocusArea,
}


/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main menu with categories
    MainMenu,
    /// Board selection
    BoardSelection,
    /// Package selection
    PackageSelection,
    /// Build options
    BuildOptions,
    /// External artifacts
    ExternalArtifacts,
    /// Diff view before save
    DiffView,
}

/// Focus area in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    /// Category menu
    Categories,
    /// Content area
    Content,
    /// Details panel
    Details,
}

/// Package information for display
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Option<String>,
    /// Package description
    pub description: Option<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Is local package
    pub is_local: bool,
}

/// Build option for configuration
#[derive(Debug, Clone)]
pub struct BuildOption {
    /// Option name
    pub name: String,
    /// Option type
    pub option_type: OptionType,
    /// Current value
    pub value: String,
    /// Description
    pub description: String,
}

/// Option type for build options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionType {
    /// Boolean toggle
    Bool,
    /// String input
    String,
    /// Choice from list
    Choice(Vec<String>),
}

impl ConfigTui {
    /// Create a new TUI instance
    pub fn new(project_dir: &Path, board_only: bool, packages_only: bool) -> anyhow::Result<Self> {
        let manifest = load_manifest_for_config(project_dir)
            .unwrap_or_else(|_| Manifest::default());

        // Get available packages
        let package_names = get_available_packages(project_dir);
        let available_packages: Vec<PackageInfo> = package_names
            .iter()
            .map(|name| {
                let deps = get_package_dependencies(project_dir, name);
                PackageInfo {
                    name: name.clone(),
                    version: Some("local".to_string()),
                    description: Some(format!("Local package: {name}")),
                    dependencies: deps,
                    is_local: true,
                }
            })
            .collect();

        // Get currently selected packages
        let selected_packages: HashSet<String> = manifest.packages.keys().cloned().collect();

        // Build options
        let build_options = vec![
            BuildOption {
                name: "compress".to_string(),
                option_type: OptionType::Bool,
                value: manifest.build.compress.to_string(),
                description: "Enable binary compression with UPX".to_string(),
            },
            BuildOption {
                name: "image_format".to_string(),
                option_type: OptionType::Choice(vec![
                    "ext4".to_string(),
                    "squashfs".to_string(),
                    "initramfs".to_string(),
                ]),
                value: manifest.build.image_format.clone(),
                description: "Output image format".to_string(),
            },
            BuildOption {
                name: "rootfs_size".to_string(),
                option_type: OptionType::String,
                value: manifest.build.rootfs_size.clone(),
                description: "Root filesystem size (e.g., 256M, 1G)".to_string(),
            },
            BuildOption {
                name: "hostname".to_string(),
                option_type: OptionType::String,
                value: manifest.build.hostname.clone(),
                description: "Target system hostname".to_string(),
            },
        ];

        // Determine initial view mode
        let view_mode = if board_only {
            ViewMode::BoardSelection
        } else if packages_only {
            ViewMode::PackageSelection
        } else {
            ViewMode::MainMenu
        };

        let mut category_state = ListState::default();
        category_state.select(Some(0));

        let mut package_state = ListState::default();
        if !available_packages.is_empty() {
            package_state.select(Some(0));
        }

        let mut build_option_state = ListState::default();
        if !build_options.is_empty() {
            build_option_state.select(Some(0));
        }

        Ok(Self {
            manifest,
            project_dir: project_dir.to_path_buf(),
            has_changes: false,
            view_mode,
            selected_category: ConfigCategory::Board,
            category_state,
            available_packages,
            package_state,
            selected_packages,
            build_options,
            build_option_state,
            editing_option: None,
            edit_buffer: String::new(),
            show_diff: false,
            pending_diff: Vec::new(),
            warning_message: None,
            focus: FocusArea::Categories,
        })
    }


    /// Run the TUI
    pub fn run(&mut self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main event loop
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                // Handle quit
                if key.code == KeyCode::Char('q') && !self.is_editing() {
                    if self.has_changes {
                        self.show_diff = true;
                        self.generate_diff();
                        self.view_mode = ViewMode::DiffView;
                    } else {
                        return Ok(());
                    }
                }

                // Handle escape
                if key.code == KeyCode::Esc {
                    if self.is_editing() {
                        self.editing_option = None;
                        self.edit_buffer.clear();
                    } else if self.show_diff {
                        self.show_diff = false;
                        self.view_mode = ViewMode::MainMenu;
                    } else if self.view_mode != ViewMode::MainMenu {
                        self.view_mode = ViewMode::MainMenu;
                        self.focus = FocusArea::Categories;
                    } else {
                        if self.has_changes {
                            self.show_diff = true;
                            self.generate_diff();
                            self.view_mode = ViewMode::DiffView;
                        } else {
                            return Ok(());
                        }
                    }
                    continue;
                }

                // Handle save
                if key.code == KeyCode::Char('s') && !self.is_editing() {
                    if self.has_changes {
                        self.save_changes()?;
                        return Ok(());
                    }
                }

                // Handle Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }

                // Handle view-specific input
                match self.view_mode {
                    ViewMode::MainMenu => self.handle_main_menu_input(key.code),
                    ViewMode::BoardSelection => self.handle_board_input(key.code),
                    ViewMode::PackageSelection => self.handle_package_input(key.code),
                    ViewMode::BuildOptions => self.handle_build_options_input(key.code),
                    ViewMode::ExternalArtifacts => self.handle_external_input(key.code),
                    ViewMode::DiffView => {
                        if key.code == KeyCode::Char('y') || key.code == KeyCode::Enter {
                            self.save_changes()?;
                            return Ok(());
                        } else if key.code == KeyCode::Char('n') {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Check if currently editing a text field
    fn is_editing(&self) -> bool {
        self.editing_option.is_some()
    }

    /// Draw the TUI
    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(10),    // Main content
                Constraint::Length(3),  // Status bar
            ])
            .split(f.area());

        self.draw_title(f, chunks[0]);
        self.draw_main_content(f, chunks[1]);
        self.draw_status_bar(f, chunks[2]);
    }

    /// Draw title bar
    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let title = match self.view_mode {
            ViewMode::MainMenu => "🔧 Zigroot Configuration",
            ViewMode::BoardSelection => "🎯 Board Selection",
            ViewMode::PackageSelection => "📦 Package Selection",
            ViewMode::BuildOptions => "⚙️  Build Options",
            ViewMode::ExternalArtifacts => "📎 External Artifacts",
            ViewMode::DiffView => "📝 Review Changes",
        };

        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let title_text = Paragraph::new(title)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .block(title_block);

        f.render_widget(title_text, area);
    }


    /// Draw main content area
    fn draw_main_content(&mut self, f: &mut Frame, area: Rect) {
        match self.view_mode {
            ViewMode::MainMenu => self.draw_main_menu(f, area),
            ViewMode::BoardSelection => self.draw_board_selection(f, area),
            ViewMode::PackageSelection => self.draw_package_selection(f, area),
            ViewMode::BuildOptions => self.draw_build_options(f, area),
            ViewMode::ExternalArtifacts => self.draw_external_artifacts(f, area),
            ViewMode::DiffView => self.draw_diff_view(f, area),
        }
    }

    /// Draw main menu with categories
    fn draw_main_menu(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        // Categories list
        let categories = vec![
            ListItem::new("  Board"),
            ListItem::new("  Packages"),
            ListItem::new("  Build Options"),
            ListItem::new("  External Artifacts"),
        ];

        let categories_list = List::new(categories)
            .block(Block::default().borders(Borders::ALL).title("Categories"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(categories_list, chunks[0], &mut self.category_state);

        // Details panel
        let details = self.get_category_details();
        let details_block = Block::default()
            .borders(Borders::ALL)
            .title("Details");

        let details_text = Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .block(details_block);

        f.render_widget(details_text, chunks[1]);
    }

    /// Get details for selected category
    fn get_category_details(&self) -> String {
        match self.category_state.selected() {
            Some(0) => {
                let board_name = self.manifest.board.name.as_deref().unwrap_or("Not set");
                format!(
                    "Board Configuration\n\n\
                     Current board: {board_name}\n\n\
                     Press Enter to select a target board.\n\
                     The board determines the target architecture and default settings."
                )
            }
            Some(1) => {
                let pkg_count = self.selected_packages.len();
                let available = self.available_packages.len();
                format!(
                    "Package Selection\n\n\
                     Selected: {pkg_count} packages\n\
                     Available: {available} local packages\n\n\
                     Press Enter to browse and select packages.\n\
                     Dependencies are automatically selected."
                )
            }
            Some(2) => {
                format!(
                    "Build Options\n\n\
                     Compression: {}\n\
                     Image format: {}\n\
                     Rootfs size: {}\n\
                     Hostname: {}\n\n\
                     Press Enter to configure build options.",
                    if self.manifest.build.compress { "enabled" } else { "disabled" },
                    self.manifest.build.image_format,
                    self.manifest.build.rootfs_size,
                    self.manifest.build.hostname
                )
            }
            Some(3) => {
                let ext_count = self.manifest.external.len();
                format!(
                    "External Artifacts\n\n\
                     Configured: {ext_count} artifacts\n\n\
                     Press Enter to manage external artifacts\n\
                     (bootloaders, kernels, DTBs, etc.)"
                )
            }
            _ => String::new(),
        }
    }

    /// Draw board selection view
    fn draw_board_selection(&mut self, f: &mut Frame, area: Rect) {
        let current_board = self.manifest.board.name.as_deref().unwrap_or("Not set");
        let text = format!(
            "Current board: {current_board}\n\n\
             Board selection from registry is not yet implemented.\n\
             You can set the board manually in zigroot.toml.\n\n\
             Press Esc to go back."
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Board Selection");

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(block);

        f.render_widget(paragraph, area);
    }

    /// Draw package selection view
    fn draw_package_selection(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Package list
        let items: Vec<ListItem> = self
            .available_packages
            .iter()
            .map(|pkg| {
                let selected = self.selected_packages.contains(&pkg.name);
                let marker = if selected { "[✓]" } else { "[ ]" };
                let style = if selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{marker} {}", pkg.name)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Packages"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[0], &mut self.package_state);

        // Package details
        let details = if let Some(idx) = self.package_state.selected() {
            if let Some(pkg) = self.available_packages.get(idx) {
                let deps = if pkg.dependencies.is_empty() {
                    "None".to_string()
                } else {
                    pkg.dependencies.join(", ")
                };
                let dependents = get_package_dependents(&self.project_dir, &pkg.name);
                let dependents_str = if dependents.is_empty() {
                    "None".to_string()
                } else {
                    dependents.join(", ")
                };
                format!(
                    "Package: {}\n\
                     Version: {}\n\
                     Description: {}\n\n\
                     Dependencies: {deps}\n\
                     Depended by: {dependents_str}\n\n\
                     Press Space to toggle selection.",
                    pkg.name,
                    pkg.version.as_deref().unwrap_or("unknown"),
                    pkg.description.as_deref().unwrap_or("No description")
                )
            } else {
                "No package selected".to_string()
            }
        } else {
            "No packages available".to_string()
        };

        let details_block = Block::default()
            .borders(Borders::ALL)
            .title("Package Details");

        let details_text = Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .block(details_block);

        f.render_widget(details_text, chunks[1]);

        // Show warning if any
        if let Some(ref warning) = self.warning_message {
            let warning_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 3,
                width: area.width - 4,
                height: 2,
            };
            let warning_text = Paragraph::new(warning.as_str())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(warning_text, warning_area);
        }
    }


    /// Draw build options view
    fn draw_build_options(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Options list
        let items: Vec<ListItem> = self
            .build_options
            .iter()
            .enumerate()
            .map(|(idx, opt)| {
                let value_display = match &opt.option_type {
                    OptionType::Bool => {
                        if opt.value == "true" { "[✓]" } else { "[ ]" }
                    }
                    _ => &opt.value,
                };
                let editing = self.editing_option == Some(idx);
                let display = if editing {
                    format!("{}: {}_", opt.name, self.edit_buffer)
                } else {
                    format!("{}: {value_display}", opt.name)
                };
                let style = if editing {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                ListItem::new(display).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Build Options"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[0], &mut self.build_option_state);

        // Option details
        let details = if let Some(idx) = self.build_option_state.selected() {
            if let Some(opt) = self.build_options.get(idx) {
                let type_str = match &opt.option_type {
                    OptionType::Bool => "Boolean (toggle with Space)".to_string(),
                    OptionType::String => "Text (press Enter to edit)".to_string(),
                    OptionType::Choice(choices) => format!("Choice: {}", choices.join(", ")),
                };
                format!(
                    "Option: {}\n\
                     Type: {type_str}\n\
                     Current: {}\n\n\
                     {}",
                    opt.name, opt.value, opt.description
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let details_block = Block::default()
            .borders(Borders::ALL)
            .title("Option Details");

        let details_text = Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .block(details_block);

        f.render_widget(details_text, chunks[1]);
    }

    /// Draw external artifacts view
    fn draw_external_artifacts(&mut self, f: &mut Frame, area: Rect) {
        let text = format!(
            "External Artifacts: {} configured\n\n\
             External artifact management is not yet fully implemented.\n\
             You can configure artifacts manually in zigroot.toml.\n\n\
             Press Esc to go back.",
            self.manifest.external.len()
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title("External Artifacts");

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(block);

        f.render_widget(paragraph, area);
    }

    /// Draw diff view before saving
    fn draw_diff_view(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                "Changes to be saved:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for diff_line in &self.pending_diff {
            let style = if diff_line.starts_with('+') {
                Style::default().fg(Color::Green)
            } else if diff_line.starts_with('-') {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(diff_line.clone(), style)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press 'y' or Enter to save, 'n' or Esc to discard",
            Style::default().fg(Color::Yellow),
        )));

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Review Changes");

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(block);

        f.render_widget(paragraph, area);
    }

    /// Draw status bar
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = if self.has_changes {
            "Modified • "
        } else {
            ""
        };

        let help = match self.view_mode {
            ViewMode::MainMenu => "↑↓/jk: Navigate • Enter: Select • s: Save • q: Quit",
            ViewMode::PackageSelection => "↑↓/jk: Navigate • Space: Toggle • Enter: Details • Esc: Back",
            ViewMode::BuildOptions => "↑↓/jk: Navigate • Space/Enter: Edit • Esc: Back",
            ViewMode::DiffView => "y/Enter: Save • n/Esc: Discard",
            _ => "↑↓/jk: Navigate • Esc: Back",
        };

        let status_text = format!("{status}{help}");
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray));

        let paragraph = Paragraph::new(status_text).block(block);

        f.render_widget(paragraph, area);
    }


    /// Handle main menu input
    fn handle_main_menu_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.category_state.selected().unwrap_or(0);
                let new_i = if i == 0 { 3 } else { i - 1 };
                self.category_state.select(Some(new_i));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.category_state.selected().unwrap_or(0);
                let new_i = if i >= 3 { 0 } else { i + 1 };
                self.category_state.select(Some(new_i));
            }
            KeyCode::Enter => {
                match self.category_state.selected() {
                    Some(0) => self.view_mode = ViewMode::BoardSelection,
                    Some(1) => self.view_mode = ViewMode::PackageSelection,
                    Some(2) => self.view_mode = ViewMode::BuildOptions,
                    Some(3) => self.view_mode = ViewMode::ExternalArtifacts,
                    _ => {}
                }
                self.focus = FocusArea::Content;
            }
            _ => {}
        }
    }

    /// Handle board selection input
    fn handle_board_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.view_mode = ViewMode::MainMenu;
                self.focus = FocusArea::Categories;
            }
            _ => {}
        }
    }

    /// Handle package selection input
    fn handle_package_input(&mut self, key: KeyCode) {
        self.warning_message = None;

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.available_packages.is_empty() {
                    let i = self.package_state.selected().unwrap_or(0);
                    let new_i = if i == 0 {
                        self.available_packages.len() - 1
                    } else {
                        i - 1
                    };
                    self.package_state.select(Some(new_i));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.available_packages.is_empty() {
                    let i = self.package_state.selected().unwrap_or(0);
                    let new_i = if i >= self.available_packages.len() - 1 {
                        0
                    } else {
                        i + 1
                    };
                    self.package_state.select(Some(new_i));
                }
            }
            KeyCode::Char(' ') => {
                if let Some(idx) = self.package_state.selected() {
                    if let Some(pkg) = self.available_packages.get(idx) {
                        let pkg_name = pkg.name.clone();
                        if self.selected_packages.contains(&pkg_name) {
                            // Deselecting - check for dependents
                            let dependents = get_package_dependents(&self.project_dir, &pkg_name);
                            let selected_dependents: Vec<_> = dependents
                                .iter()
                                .filter(|d| self.selected_packages.contains(*d))
                                .cloned()
                                .collect();

                            if !selected_dependents.is_empty() {
                                self.warning_message = Some(format!(
                                    "⚠️  Warning: {} depends on this package",
                                    selected_dependents.join(", ")
                                ));
                            }
                            self.selected_packages.remove(&pkg_name);
                            self.has_changes = true;
                        } else {
                            // Selecting - auto-select dependencies
                            self.selected_packages.insert(pkg_name.clone());
                            let deps = get_package_dependencies(&self.project_dir, &pkg_name);
                            for dep in deps {
                                if !self.selected_packages.contains(&dep) {
                                    self.selected_packages.insert(dep);
                                }
                            }
                            self.has_changes = true;
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::MainMenu;
                self.focus = FocusArea::Categories;
            }
            _ => {}
        }
    }

    /// Handle build options input
    fn handle_build_options_input(&mut self, key: KeyCode) {
        if self.is_editing() {
            // Handle text editing
            match key {
                KeyCode::Enter => {
                    if let Some(idx) = self.editing_option {
                        if let Some(opt) = self.build_options.get_mut(idx) {
                            opt.value = self.edit_buffer.clone();
                            self.has_changes = true;
                        }
                    }
                    self.editing_option = None;
                    self.edit_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.build_options.is_empty() {
                    let i = self.build_option_state.selected().unwrap_or(0);
                    let new_i = if i == 0 {
                        self.build_options.len() - 1
                    } else {
                        i - 1
                    };
                    self.build_option_state.select(Some(new_i));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.build_options.is_empty() {
                    let i = self.build_option_state.selected().unwrap_or(0);
                    let new_i = if i >= self.build_options.len() - 1 {
                        0
                    } else {
                        i + 1
                    };
                    self.build_option_state.select(Some(new_i));
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(idx) = self.build_option_state.selected() {
                    if let Some(opt) = self.build_options.get_mut(idx) {
                        match &opt.option_type {
                            OptionType::Bool => {
                                opt.value = if opt.value == "true" {
                                    "false".to_string()
                                } else {
                                    "true".to_string()
                                };
                                self.has_changes = true;
                            }
                            OptionType::Choice(choices) => {
                                let current_idx = choices
                                    .iter()
                                    .position(|c| c == &opt.value)
                                    .unwrap_or(0);
                                let next_idx = (current_idx + 1) % choices.len();
                                opt.value = choices[next_idx].clone();
                                self.has_changes = true;
                            }
                            OptionType::String => {
                                self.editing_option = Some(idx);
                                self.edit_buffer = opt.value.clone();
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::MainMenu;
                self.focus = FocusArea::Categories;
            }
            _ => {}
        }
    }

    /// Handle external artifacts input
    fn handle_external_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.view_mode = ViewMode::MainMenu;
                self.focus = FocusArea::Categories;
            }
            _ => {}
        }
    }


    /// Generate diff of changes
    fn generate_diff(&mut self) {
        self.pending_diff.clear();

        // Board changes
        let old_board = self.manifest.board.name.as_deref().unwrap_or("(none)");
        self.pending_diff.push(format!("Board: {old_board}"));

        // Package changes
        let old_packages: HashSet<_> = self.manifest.packages.keys().cloned().collect();
        let added: Vec<_> = self
            .selected_packages
            .difference(&old_packages)
            .cloned()
            .collect();
        let removed: Vec<_> = old_packages
            .difference(&self.selected_packages)
            .cloned()
            .collect();

        if !added.is_empty() {
            self.pending_diff.push(String::new());
            self.pending_diff.push("Packages:".to_string());
            for pkg in &added {
                self.pending_diff.push(format!("+ {pkg}"));
            }
        }
        if !removed.is_empty() {
            if added.is_empty() {
                self.pending_diff.push(String::new());
                self.pending_diff.push("Packages:".to_string());
            }
            for pkg in &removed {
                self.pending_diff.push(format!("- {pkg}"));
            }
        }

        // Build option changes
        let mut build_changes = Vec::new();
        for opt in &self.build_options {
            let old_value = match opt.name.as_str() {
                "compress" => self.manifest.build.compress.to_string(),
                "image_format" => self.manifest.build.image_format.clone(),
                "rootfs_size" => self.manifest.build.rootfs_size.clone(),
                "hostname" => self.manifest.build.hostname.clone(),
                _ => continue,
            };
            if old_value != opt.value {
                build_changes.push(format!("  {}: {} → {}", opt.name, old_value, opt.value));
            }
        }

        if !build_changes.is_empty() {
            self.pending_diff.push(String::new());
            self.pending_diff.push("Build Options:".to_string());
            for change in build_changes {
                self.pending_diff.push(change);
            }
        }

        if self.pending_diff.len() <= 1 {
            self.pending_diff.push(String::new());
            self.pending_diff.push("No changes to save.".to_string());
        }
    }

    /// Save changes to manifest
    fn save_changes(&mut self) -> anyhow::Result<()> {
        // Update manifest with selected packages
        self.manifest.packages.clear();
        for pkg_name in &self.selected_packages {
            self.manifest.packages.insert(
                pkg_name.clone(),
                PackageRef {
                    version: None,
                    git: None,
                    ref_: None,
                    registry: None,
                    options: std::collections::HashMap::new(),
                },
            );
        }

        // Update build options
        for opt in &self.build_options {
            match opt.name.as_str() {
                "compress" => {
                    self.manifest.build.compress = opt.value == "true";
                }
                "image_format" => {
                    self.manifest.build.image_format = opt.value.clone();
                }
                "rootfs_size" => {
                    self.manifest.build.rootfs_size = opt.value.clone();
                }
                "hostname" => {
                    self.manifest.build.hostname = opt.value.clone();
                }
                _ => {}
            }
        }

        // Write manifest to file
        let manifest_path = self.project_dir.join("zigroot.toml");
        let toml_content = self.manifest.to_toml()?;
        std::fs::write(&manifest_path, toml_content)?;

        println!("✓ Configuration saved to zigroot.toml");
        self.has_changes = false;

        Ok(())
    }
}
