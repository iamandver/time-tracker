use crate::io::{ColorType, Out, Vector2};
use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use crossterm::event::{read, Event, KeyCode};
use std::cmp::PartialEq;
use std::env::current_exe;
use std::fmt::{Display, Formatter};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::{cmp, fs};
use crossterm::event;

mod io;

const ANSI_WHITE: u8 = 255;
const ANSI_BLUE: u8 = 19;
const ANSI_CYAN: u8 = 87;
const ANSI_CYAN_DARK: u8 = 73;
const ANSI_YELLOW: u8 = 226;
const ANSI_GRAY: u8 = 248;
const ANSI_BLACK: u8 = 16;
const ANSI_RED_DARK: u8 = 124;
const ANSI_RED: u8 = 160;

static COL_BG_MAIN: u8 = ANSI_BLUE;
static COL_OUTLINE_MAIN: u8 = ANSI_CYAN;
static COL_BG_POPUP: u8 = ANSI_GRAY;
static COL_OUTLINE_POPUP: u8 = ANSI_BLACK;
static COL_TEXT_WHITE: u8 = ANSI_WHITE;
static COL_TEXT_BLACK: u8 = ANSI_BLACK;
static COL_TEXT_HIGHLIGHT: u8 = ANSI_YELLOW;
static COL_TEXT_DIM: u8 = ANSI_CYAN_DARK;
static COL_TEXT_RED_DARK: u8 = ANSI_RED_DARK;
static COL_TEXT_RED: u8 = ANSI_RED;

// graphics
const FRAME_H: char = '═';
const FRAME_V: char = '║';
const CORNER_TL: char = '╔';
const CORNER_TR: char = '╗';
const CORNER_BR: char = '╝';
const CORNER_BL: char = '╚';
const INTERSECT_T: char = '╤';
const INTERSECT_B: char = '╧';
const INTERSECT_L: char = '╟';
const INTERSECT_R: char = '╢';
const DIVIDER_H: char = '─';
const DIVIDER_V: char = '│';

const CURSOR: char = '█';
const ARROW: char = '▶';

const KEY_NEW: KeyCode = KeyCode::Char('n');
const KEY_DELETE: KeyCode = KeyCode::Char('d');
const KEY_END: KeyCode = KeyCode::Char(' ');
const KEY_EDIT: KeyCode = KeyCode::Char('e');
const KEY_QUIT: KeyCode = KeyCode::Char('q');
const KEY_ENTER: KeyCode = KeyCode::Enter;
const KEY_TAB: KeyCode = KeyCode::Tab;
const KEY_YES: KeyCode = KeyCode::Char('y');
const KEY_NO: KeyCode = KeyCode::Char('n');
const KEY_UP: KeyCode = KeyCode::Up;
const KEY_DOWN: KeyCode = KeyCode::Down;
const KEY_BACKSPACE: KeyCode = KeyCode::Backspace;
const KEY_ESCAPE: KeyCode = KeyCode::Esc;

fn key_to_char(key: KeyCode) -> String
{
    let character: String = match key
    {
        KeyCode::Char(c) => match c
        {
            ' ' => "SPACE".to_string(),
            _ => c.to_string(),
        },
        _ =>
        {
            panic!("Unknows Key type.")
        }
    };

    character
}

type Controls = Vec<Control>;

fn get_controls() -> Vec<Control>
{
    vec![
        Control {
            key: KEY_NEW,
            description: "new".to_string(),
        },
        Control {
            key: KEY_DELETE,
            description: "delete".to_string(),
        },
        Control {
            key: KEY_END,
            description: "end".to_string(),
        },
        Control {
            key: KEY_QUIT,
            description: "quit".to_string(),
        },
    ]
}

struct Control
{
    key: KeyCode,
    description: String,
}

#[derive(PartialEq, Copy, Clone)]
enum CommandState
{
    Idle,
    Input(InputField),
    Delete(bool),
    End,
    Quitting,
}

#[derive(PartialEq, Copy, Clone)]
enum InputField
{
    Description(PromptOpen), // close previous inside here
    Tag(EditState),
}

#[derive(PartialEq, Copy, Clone)]
enum PromptOpen
{
    Yes,
    No,
}

#[derive(PartialEq, Copy, Clone)]
enum EditState
{
    Select,
    New,
    Delete(bool),
}

impl Display for CommandState
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result
    {
        match self
        {
            CommandState::Idle =>
            {
                write!(f, "List")
            }
            CommandState::Input(input_field) =>
            {
                write!(f, "Input: {}", input_field)
            }
            CommandState::Delete(_) =>
            {
                write!(f, "Delete")
            }
            CommandState::End =>
            {
                write!(f, "End")
            }
            CommandState::Quitting =>
            {
                write!(f, "Quitting")
            }
        }
    }
}
impl Display for InputField
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            InputField::Description(_) =>
            {
                write!(f, "Description")
            }
            InputField::Tag(tag_edit_state) =>
            {
                write!(f, "Tag: {}", tag_edit_state)
            }
        }
    }
}
impl Display for EditState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            EditState::Select =>
            {
                write!(f, "Select")
            }
            EditState::New =>
            {
                write!(f, "New")
            }
            EditState::Delete(_) =>
            {
                write!(f, "Delete")
            }
        }
    }
}

struct Session
{
    description: String,
    tag: String,
    start: NaiveDateTime,
    end: Option<NaiveDateTime>,
}

impl Session
{
    fn construct_db_string(&self, separator: char, format: &str) -> String
    {
        let format_split = format.split(' ').collect::<Vec<&str>>();
        let date_format = format_split[0];
        let time_format = format_split[1];

        let end = self.end.expect("Cannot export ongoing session.");
        let duration = end - self.start;

        let secs_per_minute = 60;
        let secs_per_hour = 3600;

        let hours = duration.num_hours();
        let minutes = duration.num_minutes() - hours * secs_per_minute;
        let seconds = duration.num_seconds() - hours * secs_per_hour - minutes * secs_per_minute;

        let date = format!("{}", self.start.format(date_format));
        let description = &self.description;
        let tag = &self.tag;
        let start = format!("{}", self.start.format(time_format));
        let end = format!("{}", end.format(time_format));
        let duration = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        format!("{date}{separator}{description}{separator}{tag}{separator}{start}{separator}{end}{separator}{duration}")
    }

    fn from(description: &str, tag: &str, start: NaiveDateTime, end: Option<NaiveDateTime>) -> Session
    {
        Session {
            description: description.to_string(),
            tag: tag.to_string(),
            start,
            end,
        }
    }

    fn is_running(&self) -> bool
    {
        self.end.is_none()
    }
}

struct DatabaseHandler
{
    database_path: String,
    sessions_file_name: String,
    tags_file_name: String,
    value_separator: char,
}

impl DatabaseHandler
{
    fn new() -> Self
    {
        let current_exe = current_exe().expect("Failed to retrieve executable path.");
        let current_path = current_exe.parent().expect("Failed to retrieve executable parent folder.");
        let database_path = current_path.join("database");

        let handler = DatabaseHandler {
            database_path: database_path.to_str().expect("Failed to parse db path string.").to_string(),
            sessions_file_name: "sessions.txt".to_string(),
            tags_file_name: "tags.txt".to_string(),
            value_separator: ';',
        };

        handler.try_create_data_path_and_files().expect("Error while creating database.");

        handler
    }

    fn try_create_data_path_and_files(&self) -> Result<(), Box<dyn std::error::Error>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);
        let tags_path = database_path.join(&self.tags_file_name);

        if !database_path.exists()
        {
            fs::create_dir(database_path)?;
        }

        if !sessions_path.exists()
        {
            File::create(sessions_path)?;
        }

        if !tags_path.exists()
        {
            File::create(tags_path)?;
        }

        Ok(())
    }

    fn export_session(&self, session_string: &String) -> Result<(), Box<dyn std::error::Error>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        if let Ok(mut sessions) = OpenOptions::new().append(true).open(sessions_path)
        {
            sessions.write_fmt(format_args!("\n{}", session_string))?;
        }

        self.remove_empty_lines(&self.sessions_file_name);

        Ok(())
    }

    fn export_tag(&self, tag: &String) -> Result<(), Box<dyn std::error::Error>>
    {
        let database_path = Path::new(&self.database_path);
        let tags_path = database_path.join(&self.tags_file_name);

        if let Ok(mut tags) = OpenOptions::new().append(true).open(tags_path)
        {
            tags.write_fmt(format_args!("\n{}", tag))?;
        }

        self.remove_empty_lines(&self.tags_file_name);

        Ok(())
    }

    fn import_sessions(&self, format: &str) -> Option<Vec<Session>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        if let Ok(sessions) = OpenOptions::new().read(true).open(sessions_path)
        {
            let lines = BufReader::new(sessions).lines().map_while(Result::ok).filter(|x| !x.is_empty()).collect::<Vec<String>>();

            return self.parse_sessions(lines, format);
        }

        None
    }

    fn parse_sessions(&self, sessions: Vec<String>, format: &str) -> Option<Vec<Session>>
    {
        let mut parsed_sessions = Vec::new();
        for session_string in sessions
        {
            let session_split = session_string.split(self.value_separator).collect::<Vec<&str>>();

            let date = session_split[0];
            let description = session_split[1];
            let tag = session_split[2];
            let start = session_split[3];
            let end = session_split[4];

            let start_string = format!("{date} {start}");
            let end_string = format!("{date} {end}");

            let start_date = NaiveDateTime::parse_from_str(&start_string, format).expect("Error parsing start date.");
            let end_date = NaiveDateTime::parse_from_str(&end_string, format).expect("Error parsing end date.");

            let session = Session::from(description, tag, start_date, Some(end_date));

            parsed_sessions.push(session);
        }

        if parsed_sessions.is_empty()
        {
            return None;
        }

        Some(parsed_sessions)
    }

    fn import_tags(&self) -> Option<Vec<String>>
    {
        let database_path = Path::new(&self.database_path);
        let tags_path = database_path.join(&self.tags_file_name);

        if let Ok(tags) = OpenOptions::new().read(true).open(tags_path)
        {
            let tags = BufReader::new(tags).lines().map_while(Result::ok).filter(|x| !x.is_empty()).collect::<Vec<String>>();

            return Some(tags);
        }

        None
    }

    fn remove_empty_lines(&self, file_name: &String)
    {
        let database_path = Path::new(&self.database_path);
        let file_path = database_path.join(file_name);
        let temp_path = format!("{file_name}.temp");

        if let Ok(file) = OpenOptions::new().read(true).open(file_path.clone())
        {
            let entries = BufReader::new(file).lines().map_while(Result::ok).filter(|x| !x.is_empty()).collect::<Vec<String>>();

            if !entries.is_empty()
            {
                if let Ok(mut temp_file) = OpenOptions::new().truncate(true).write(true).create_new(true).open(temp_path.clone())
                {
                    for entry in entries
                    {
                        temp_file.write_fmt(format_args!("{}\n", entry)).expect("Failed to write to temp file.");
                    }

                    fs::rename(&temp_path, &file_path).expect("Failed renaming after removing empty lines.");
                }
            }
        }
    }

    fn delete_session(&self, session_index: usize)
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        let temp_sessions_path = database_path.join("sessions.txt.temp");

        if let Ok(sessions) = OpenOptions::new().read(true).open(sessions_path.clone())
        {
            let mut session_entries = BufReader::new(sessions).lines().map_while(Result::ok).collect::<Vec<String>>();

            session_entries.remove(session_index);

            if let Ok(mut temp_sessions) =
                OpenOptions::new().truncate(true).write(true).create_new(true).open(temp_sessions_path.clone())
            {
                for entry in session_entries
                {
                    temp_sessions.write_fmt(format_args!("{}\n", entry)).expect("Failed to delete session from database.");
                }

                fs::rename(&temp_sessions_path, &sessions_path).expect("Failed to rename new database.");
            }
        }
    }
}

struct AppManager
{
    version: String,
    renderer: Out,
    database_handler: DatabaseHandler,
    date_format: String,
    running: bool,
    tags: Vec<String>,
    temp_tag_index: usize,
    selected_session_index: usize,
    selected_tag_index: usize,
    sessions: Vec<Session>,
    state: CommandState,
    description_buffer: String,
    tag_buffer: String,
}

impl AppManager
{
    fn new() -> Self
    {
        let mut manager = AppManager {
            version: "0.2.2".to_string(),
            renderer: Out::new(),
            database_handler: DatabaseHandler::new(),
            date_format: "%d-%m-%Y %H:%M:%S".to_string(),
            running: true,
            tags: Vec::new(),
            temp_tag_index: 0,
            selected_session_index: 0,
            selected_tag_index: 0,
            sessions: Vec::new(),
            state: CommandState::Idle,
            description_buffer: String::new(),
            tag_buffer: String::new(),
        };

        if let Some(sessions) = manager.database_handler.import_sessions(&manager.date_format)
        {
            manager.sessions = sessions;

            if let Some(tags) = manager.database_handler.import_tags()
            {
                manager.tags = tags;

                let last_used_tag = &manager.sessions.last().unwrap().tag;
                let tag_index = manager.tags.iter().position(|t| t.eq(last_used_tag)).expect("Failed to retrieve tag index.");

                manager.set_selected_tag_index(tag_index);
            }
        }

        manager
    }

    fn try_start_new_session(&mut self)
    {
        self.description_buffer = self.description_buffer.trim().to_string();

        if let Some(selected_tag) = self.tags.get(self.get_selected_tag_index())
        {
            if !self.description_buffer.is_empty()
            {
                let start = self.get_current_time();

                self.sessions.push(Session::from(&self.description_buffer, selected_tag, start, None));

                self.description_buffer.clear();
            }
        }
    }

    fn get_current_time(&self) -> NaiveDateTime
    {
        let now = Local::now();
        let date = now.date_naive();
        let time = now.time();

        let year = date.year();
        let month = date.month();
        let day = date.day();

        let hour = time.hour();
        let minute = time.minute();
        let second = time.second();

        let formatted_start = format!("{day}-{month}-{year} {hour}:{minute}:{second}");

        NaiveDateTime::parse_from_str(&formatted_start, &self.date_format).expect("Failed to construct time.")
    }

    fn try_store_tag(&mut self)
    {
        self.tag_buffer = self.tag_buffer.trim().to_string();

        if self.tag_buffer.is_empty()
        {
            return;
        }

        self.tags.push(self.tag_buffer.clone());
        self.database_handler.export_tag(&self.tag_buffer).expect("Failed to export tag.");
        self.set_selected_tag_index(self.tags.len() - 1);
        self.tag_buffer.clear();
    }

    fn set_selected_tag_index(&mut self, index: usize)
    {
        self.selected_tag_index = index;
    }

    fn get_selected_tag_index(&self) -> usize
    {
        self.selected_tag_index
    }

    fn is_last_session_still_running(&self) -> bool
    {
        if let Some(last_session) = self.sessions.last()
        {
            return last_session.is_running();
        }

        false
    }

    fn end_running_session(&mut self)
    {
        let end = self.get_current_time();
        if let Some(last_session) = self.sessions.last_mut()
        {
            if last_session.end.is_none()
            {
                last_session.end = Some(end);
                let session_string = last_session.construct_db_string(self.database_handler.value_separator, &self.date_format);

                self.database_handler.export_session(&session_string).expect("Error exporting session.");
            }
        }
    }

    fn delete_selected_session(&mut self)
    {
        if self.sessions.is_empty()
        {
            return;
        }

        if let Some(session) = self.sessions.get(self.selected_session_index)
        {
            if !session.is_running()
            {
                self.database_handler.delete_session(self.selected_session_index);
            }
        }

        self.sessions.remove(self.selected_session_index);
    }
}

fn debug_draw(app_manager: &mut AppManager, message: &str)
{
    let window_size = app_manager.renderer.get_terminal_size();
    let debug_pos = Vector2::new(window_size.x - message.len() as u16 - 4, 0);

    app_manager.renderer.push_color(ColorType::Foreground, COL_OUTLINE_MAIN);
    app_manager.renderer.draw_at(format!(" v{} ", &app_manager.version), &debug_pos);
    app_manager.renderer.pop_color(ColorType::Foreground);
}

fn main()
{
    let mut app_manager = AppManager::new();
    app_manager.renderer.clear_screen();

    while app_manager.running
    {
        render(&mut app_manager);

        app_manager.renderer.check_color_stacks();

        update(&mut app_manager);
    }
}

#[allow(clippy::too_many_lines)]
fn update(app_manager: &mut AppManager)
{
    if let Some(key) = get_user_key()
    {
        match app_manager.state
        {
            CommandState::Idle => match key
            {
                KEY_NEW =>
                {
                    app_manager.state = CommandState::Input(InputField::Description(PromptOpen::No));
                }
                KEY_DELETE =>
                {
                    app_manager.selected_session_index = app_manager.sessions.len() - 1;
                    app_manager.state = CommandState::Delete(false);
                }
                KEY_END =>
                {
                    if app_manager.is_last_session_still_running()
                    {
                        app_manager.state = CommandState::End;
                    }
                }
                KEY_QUIT =>
                {
                    app_manager.state = CommandState::Quitting;
                }
                _ =>
                {}
            },
            CommandState::Input(input_field) => match input_field
            {
                InputField::Description(confirm_end_previous) => match confirm_end_previous
                {
                    PromptOpen::Yes =>
                    {
                        if key == KEY_YES
                        {
                            app_manager.end_running_session();
                            app_manager.try_start_new_session();
                            app_manager.state = CommandState::Idle;
                        }
                        else if key == KEY_NO || key == KEY_ESCAPE
                        {
                            app_manager.state = CommandState::Input(InputField::Description(PromptOpen::No));
                        }
                    }
                    PromptOpen::No => match key
                    {
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::Idle;
                        }
                        KEY_BACKSPACE =>
                        {
                            app_manager.description_buffer.pop();
                        }
                        KEY_ENTER =>
                        {
                            if app_manager.is_last_session_still_running()
                            {
                                app_manager.state = CommandState::Input(InputField::Description(PromptOpen::Yes));
                            }
                            else
                            {
                                app_manager.try_start_new_session();
                                app_manager.state = CommandState::Idle;
                            }
                        }
                        KEY_TAB =>
                        {
                            app_manager.temp_tag_index = app_manager.get_selected_tag_index();
                            app_manager.state = CommandState::Input(InputField::Tag(EditState::Select));
                        }
                        KeyCode::Char(character) =>
                        {
                            app_manager.description_buffer.push(character);
                        }
                        _ =>
                        {}
                    },
                },
                InputField::Tag(edit_state) => match edit_state
                {
                    EditState::Select => match key
                    {
                        KEY_NEW =>
                        {
                            app_manager.state = CommandState::Input(InputField::Tag(EditState::New));
                        }
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::Input(InputField::Description(PromptOpen::No));
                        }
                        KEY_UP =>
                        {
                            if app_manager.temp_tag_index > 0
                            {
                                app_manager.temp_tag_index -= 1;
                            }
                        }
                        KEY_DOWN =>
                        {
                            if app_manager.temp_tag_index + 1 < app_manager.tags.len()
                            {
                                app_manager.temp_tag_index += 1;
                            }
                        }
                        KEY_ENTER =>
                        {
                            app_manager.set_selected_tag_index(app_manager.temp_tag_index);
                            app_manager.state = CommandState::Input(InputField::Description(PromptOpen::No));
                        }
                        _ =>
                        {}
                    },
                    EditState::New => match key
                    {
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::Input(InputField::Tag(EditState::Select));
                        }
                        KEY_BACKSPACE =>
                        {
                            app_manager.tag_buffer.pop();
                        }
                        KEY_ENTER =>
                        {
                            app_manager.try_store_tag();
                            app_manager.state = CommandState::Input(InputField::Description(PromptOpen::No));
                        }
                        KeyCode::Char(character) =>
                        {
                            app_manager.tag_buffer.push(character);
                        }
                        _ =>
                        {}
                    },
                    EditState::Delete(_) =>
                    {}
                },
            },
            CommandState::Delete(is_confirm_open) =>
            {
                if is_confirm_open
                {
                    if key == KEY_YES
                    {
                        app_manager.delete_selected_session();
                        app_manager.state = CommandState::Idle;
                    }
                    else if key == KEY_NO || key == KEY_ESCAPE
                    {
                        app_manager.state = CommandState::Idle;
                    }
                }
                else
                {
                    match key
                    {
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::Idle;
                        }
                        KEY_UP =>
                        {
                            if app_manager.selected_session_index + 1 < app_manager.sessions.len()
                            {
                                app_manager.selected_session_index += 1;
                            }
                        }
                        KEY_DOWN =>
                        {
                            if app_manager.selected_session_index > 0
                            {
                                app_manager.selected_session_index -= 1;
                            }
                        }
                        KEY_ENTER =>
                        {
                            app_manager.state = CommandState::Delete(true);
                        }
                        KeyCode::Char(character) =>
                        {
                            app_manager.tag_buffer.push(character);
                        }
                        _ =>
                        {}
                    }
                }
            }
            CommandState::End =>
            {
                if key == KEY_YES
                {
                    app_manager.end_running_session();
                    app_manager.state = CommandState::Idle;
                }
                else if key == KEY_NO || key == KEY_ESCAPE
                {
                    app_manager.state = CommandState::Idle;
                }
            }
            CommandState::Quitting =>
            {
                if key == KEY_YES
                {
                    if app_manager.is_last_session_still_running()
                    {
                        app_manager.end_running_session();
                    }

                    app_manager.running = false;
                }
                else if key == KEY_NO || key == KEY_ESCAPE
                {
                    app_manager.state = CommandState::Idle;
                }
            }
        }
    }
}

fn draw_window_title(renderer: &mut Out, title: &str, window_pos: &Vector2)
{
    const OFFSET: u16 = 2;
    // let title_pos = Vector2::new(window_pos.x + ((confirm_popup_size.x - title.len() as u16) / 2) - 1, window_pos.y);
    let title_pos = Vector2::new(window_pos.x + OFFSET, window_pos.y);

    renderer.draw_at(format!(" {} ", title), &title_pos);
}

fn draw_window_shadow(renderer: &mut Out, window_size: &Vector2, window_pos: &Vector2)
{
    renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
    let shadow_bottom = " ".repeat(window_size.x as usize);
    renderer.draw_at(shadow_bottom, &Vector2::new(window_pos.x + 1, window_pos.y + window_size.y));

    for y in 1..=window_size.y
    {
        renderer.draw_at("  ", &Vector2::new(window_pos.x + window_size.x, window_pos.y + y));
    }
    renderer.pop_color(ColorType::Background);
}

fn draw_yes_no_popup(app_manager: &mut AppManager, title: &str)
{
    let confirm_popup_size = Vector2::new(40, 5);
    let window_size = app_manager.renderer.get_terminal_size();
    let confirm_popup_pos = Vector2::new((window_size.x - confirm_popup_size.x) / 2, (window_size.y - confirm_popup_size.y) / 2);
    app_manager.renderer.push_color(ColorType::Background, COL_BG_POPUP);
    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_BLACK);

    draw_window(&mut app_manager.renderer, &confirm_popup_size, &confirm_popup_pos);
    draw_window_shadow(&mut app_manager.renderer, &confirm_popup_size, &confirm_popup_pos);

    app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
    app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
    draw_window_title(&mut app_manager.renderer, title, &confirm_popup_pos);
    app_manager.renderer.pop_color(ColorType::Background);
    app_manager.renderer.pop_color(ColorType::Foreground);

    let text_pos_y = confirm_popup_pos.y + confirm_popup_size.y / 2;
    let yes_pos = Vector2::new(confirm_popup_pos.x + confirm_popup_size.x / 4 - 2, text_pos_y);
    let no_pos = Vector2::new(confirm_popup_pos.x + (confirm_popup_size.x / 4) * 3 - 2, text_pos_y);

    app_manager.renderer.draw_at('[', &yes_pos);
    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED_DARK);
    app_manager.renderer.draw('y');
    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.draw("]es");
    app_manager.renderer.draw_at('[', &no_pos);
    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED_DARK);
    app_manager.renderer.draw('n');
    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.draw("]o");

    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);
}

#[allow(clippy::too_many_lines)]
fn render(app_manager: &mut AppManager)
{
    let terminal_size = app_manager.renderer.get_terminal_size();
    let main_window_size = Vector2::new(terminal_size.x, terminal_size.y - 1);

    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_WHITE);
    app_manager.renderer.push_color(ColorType::Background, COL_BG_MAIN);

    app_manager.renderer.push_color(ColorType::Foreground, COL_OUTLINE_MAIN);
    draw_window(&mut app_manager.renderer, &main_window_size, &Vector2::new(0, 0));

    let left_offset = 2;
    let top_offset = 1;

    let command_column_width = 6;
    let date_column_width = 12;
    let timestamp_column_width = 10;

    let tag_column_width = (app_manager.sessions.iter().map(|s| &s.tag).map(String::len).max().unwrap_or(10) + 2) as u16;

    let command_column_pos = 0;
    let date_column_pos = command_column_width;
    let description_column_pos = date_column_pos + date_column_width;
    let timestamp_column_3_pos = main_window_size.x - timestamp_column_width - 2;
    let timestamp_column_2_pos = timestamp_column_3_pos - timestamp_column_width - 1;
    let timestamp_column_1_pos = timestamp_column_2_pos - timestamp_column_width - 1;
    let tag_column_pos = timestamp_column_1_pos - tag_column_width - 1;

    let dividers = [
        (command_column_pos, "Cmd"),
        (date_column_pos, "Date"),
        (description_column_pos, "Description"),
        (timestamp_column_3_pos, "Duration"),
        (timestamp_column_2_pos, "End"),
        (timestamp_column_1_pos, "Start"),
        (tag_column_pos, "Tag"),
    ];

    for (index, (column_pos, section_title)) in dividers.iter().enumerate()
    {
        app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_HIGHLIGHT);
        app_manager.renderer.draw_at(section_title, &Vector2::new(*column_pos + left_offset, top_offset));
        app_manager.renderer.pop_color(ColorType::Foreground);

        if index == 0
        {
            continue;
        }

        app_manager.renderer.draw_at(INTERSECT_T, &Vector2::new(*column_pos, 0));

        for row_index in 1..main_window_size.y - 1
        {
            app_manager.renderer.draw_at(DIVIDER_V, &Vector2::new(*column_pos, row_index));
        }

        app_manager.renderer.draw_at(INTERSECT_B, &Vector2::new(*column_pos, main_window_size.y - 1));
    }

    app_manager.renderer.pop_color(ColorType::Foreground);

    app_manager.renderer.push_color(ColorType::Foreground, COL_BG_MAIN);
    app_manager.renderer.push_color(ColorType::Background, COL_OUTLINE_MAIN);
    draw_window_title(&mut app_manager.renderer, "SESSIONS", &Vector2::new(0, 0));
    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);

    for (session_index, session) in app_manager.sessions.iter().rev().enumerate()
    {
        let entry_pos_y = top_offset + 1 + session_index as u16;

        let selected_row = if let CommandState::Delete(_) = app_manager.state
        {
            app_manager.sessions.len() - 1 - app_manager.selected_session_index == session_index
        }
        else
        {
            false
        };

        if selected_row
        {
            app_manager.renderer.push_color(ColorType::Background, COL_TEXT_DIM);

            let bg = " ".repeat(main_window_size.x as usize - 3);
            app_manager.renderer.draw_at(bg, &Vector2::new(left_offset, entry_pos_y));
        }

        let start_date = format!("{}", session.start.format("%d %b %y"));
        let start_time = format!("{}", session.start.format("%H:%M:%S"));
        let (end_time, duration) = match session.end
        {
            None => ("-".to_string(), "Running".to_string()),
            Some(end) => (end.format("%H:%M:%S").to_string(), {
                let duration = end - session.start;

                let secs_per_minute: i64 = 60;
                let secs_per_hour: i64 = 3600;

                let hours = duration.num_hours();
                let minutes = duration.num_minutes() - hours * secs_per_minute;
                let seconds = duration.num_seconds() - hours * secs_per_hour - minutes * secs_per_minute;

                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }),
        };

        app_manager.renderer.draw_at(start_date, &Vector2::new(date_column_pos + left_offset, entry_pos_y));
        app_manager.renderer.draw_at(&session.description, &Vector2::new(description_column_pos + left_offset, entry_pos_y));
        app_manager.renderer.draw_at(&session.tag, &Vector2::new(tag_column_pos + left_offset, entry_pos_y));
        app_manager.renderer.draw_at(start_time, &Vector2::new(timestamp_column_1_pos + left_offset, entry_pos_y));
        app_manager.renderer.draw_at(end_time, &Vector2::new(timestamp_column_2_pos + left_offset, entry_pos_y));

        if session.is_running()
        {
            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED);
        }
        app_manager.renderer.draw_at(duration, &Vector2::new(timestamp_column_3_pos + left_offset, entry_pos_y));
        if session.is_running()
        {
            app_manager.renderer.pop_color(ColorType::Foreground);
        }

        if selected_row
        {
            app_manager.renderer.pop_color(ColorType::Background);
        }
    }

    match app_manager.state
    {
        CommandState::Idle =>
        {}
        CommandState::Input(input_field) =>
        {
            let input_field_size = Vector2::new(terminal_size.x - 32, 3);
            let input_field_pos = Vector2::new((terminal_size.x - input_field_size.x) / 2, (terminal_size.y - input_field_size.y) / 2);

            app_manager.renderer.push_color(ColorType::Background, COL_BG_POPUP);
            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_BLACK);

            draw_window(&mut app_manager.renderer, &input_field_size, &input_field_pos);
            draw_window_shadow(&mut app_manager.renderer, &input_field_size, &input_field_pos);

            let input_field_half = input_field_pos.x + input_field_size.x / 2;
            let title = "NEW SESSION";

            app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
            app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
            draw_window_title(&mut app_manager.renderer, title, &input_field_pos);
            app_manager.renderer.pop_color(ColorType::Background);
            app_manager.renderer.pop_color(ColorType::Foreground);

            app_manager.renderer.draw_at(INTERSECT_T, &Vector2::new(input_field_half, input_field_pos.y));
            app_manager.renderer.draw_at(DIVIDER_V, &Vector2::new(input_field_half, input_field_pos.y + 1));
            app_manager.renderer.draw_at(INTERSECT_B, &Vector2::new(input_field_half, input_field_pos.y + 2));

            let text_pos_y = input_field_pos.y + 1;
            let description_input_pos = Vector2::new(input_field_pos.x + 2, text_pos_y);
            let tag_input_pos = Vector2::new(input_field_pos.x + input_field_size.x / 2 + 2, text_pos_y);

            let description_input_label = "DESCRIPTION ";
            let tag_input_label = "TAG ";
            let no_tags_msg = "- empty -".to_string();

            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED_DARK);
            app_manager.renderer.draw_at(description_input_label, &description_input_pos);
            app_manager.renderer.pop_color(ColorType::Foreground);

            app_manager.renderer.draw(&app_manager.description_buffer);

            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED_DARK);
            app_manager.renderer.draw_at(tag_input_label, &tag_input_pos);
            app_manager.renderer.pop_color(ColorType::Foreground);

            let selected_tag = app_manager.tags.get(app_manager.get_selected_tag_index()).unwrap_or(&no_tags_msg);

            app_manager.renderer.draw(selected_tag);

            match input_field
            {
                InputField::Description(confirm_end_previous) => match confirm_end_previous
                {
                    PromptOpen::Yes =>
                    {
                        draw_yes_no_popup(app_manager, "END RUNNING SESSION?");
                    }
                    PromptOpen::No =>
                    {
                        let cursor_pos_x =
                            description_input_pos.x + (description_input_label.len() + app_manager.description_buffer.len()) as u16;

                        app_manager.renderer.draw_at(CURSOR, &Vector2::new(cursor_pos_x, text_pos_y));
                    }
                },
                InputField::Tag(edit_state) =>
                {
                    let dropdown_title = "TAG";
                    let tag_dropdown_pos = &tag_input_pos;
                    let tag_dropdown_text_pos = Vector2::new(tag_dropdown_pos.x + 2, tag_dropdown_pos.y + 1);

                    if let Some(longest_tag_str) = app_manager.tags.iter().map(String::len).max()
                    {
                        let longest_tag_str = cmp::max(longest_tag_str, dropdown_title.len() + 2) as u16;
                        let tag_dropdown_size = Vector2::new(longest_tag_str + 8, app_manager.tags.len() as u16 + 2);

                        draw_window(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);
                        draw_window_shadow(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);

                        app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
                        app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
                        draw_window_title(&mut app_manager.renderer, dropdown_title, tag_dropdown_pos);
                        app_manager.renderer.pop_color(ColorType::Background);
                        app_manager.renderer.pop_color(ColorType::Foreground);

                        for (index, tag) in app_manager.tags.iter().enumerate()
                        {
                            let selected_row = index == app_manager.temp_tag_index;

                            let arrow = if selected_row
                            {
                                ARROW
                            }
                            else
                            {
                                ' '
                            };

                            if selected_row
                            {
                                app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
                                app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
                            }

                            let right_pad = longest_tag_str as usize + 1;
                            app_manager.renderer.draw_at(
                                format!(" {} {:<pad$}", arrow, tag, pad = right_pad),
                                &Vector2::new(tag_dropdown_text_pos.x, tag_dropdown_text_pos.y + index as u16),
                            );

                            if selected_row
                            {
                                app_manager.renderer.pop_color(ColorType::Background);
                                app_manager.renderer.pop_color(ColorType::Foreground);
                            }
                        }
                    }
                    else
                    {
                        let tag_dropdown_size = Vector2::new(no_tags_msg.len() as u16 + 4, 3);
                        draw_window(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);
                        draw_window_shadow(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);

                        app_manager.renderer.draw_at(&no_tags_msg, &tag_dropdown_text_pos);
                    };

                    match edit_state
                    {
                        EditState::Select =>
                        {}
                        EditState::New =>
                        {
                            let new_tag_title = "NEW TAG";
                            let new_tag_window_pos = &tag_dropdown_text_pos;
                            let new_tag_window_size = Vector2::new(32, 3);

                            draw_window(&mut app_manager.renderer, &new_tag_window_size, new_tag_window_pos);
                            draw_window_shadow(&mut app_manager.renderer, &new_tag_window_size, new_tag_window_pos);

                            app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
                            app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
                            draw_window_title(&mut app_manager.renderer, new_tag_title, new_tag_window_pos);
                            app_manager.renderer.pop_color(ColorType::Background);
                            app_manager.renderer.pop_color(ColorType::Foreground);

                            let new_tag_text_pos = Vector2::new(new_tag_window_pos.x + 2, new_tag_window_pos.y + 1);
                            app_manager.renderer.draw_at(format!("{}{}", &app_manager.tag_buffer, CURSOR), &new_tag_text_pos);
                        }
                        EditState::Delete(_) =>
                        {}
                    }
                }
            }

            app_manager.renderer.pop_color(ColorType::Background);
            app_manager.renderer.pop_color(ColorType::Foreground);
        }
        CommandState::Delete(is_confirm_open) =>
        {
            let row = (app_manager.sessions.len() - app_manager.selected_session_index - top_offset as usize) as u16;

            app_manager.renderer.push_color(ColorType::Background, COL_TEXT_DIM);
            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_HIGHLIGHT);
            app_manager.renderer.draw_at(" DEL", &Vector2::new(left_offset - 1, 2 + row));
            app_manager.renderer.pop_color(ColorType::Foreground);
            app_manager.renderer.pop_color(ColorType::Background);

            if is_confirm_open
            {
                draw_yes_no_popup(app_manager, "CONFIRM DELETE");
            }
        }
        CommandState::End =>
        {
            draw_yes_no_popup(app_manager, "END SESSION?");
        }
        CommandState::Quitting =>
        {
            draw_yes_no_popup(app_manager, "REALLY QUIT?");
        }
    }

    let version = app_manager.version.clone();
    debug_draw(app_manager, &version);

    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);

    draw_control_panel(app_manager);


    app_manager.renderer.render();
}

fn draw_control_panel(app_manager: &mut AppManager)
{
    let controls: Controls = get_controls();
    let control_columns = controls.len() as u16;

    let window_size = app_manager.renderer.get_terminal_size();
    let start_position = Vector2::new(0, window_size.y - 1);
    let control_section_width = window_size.x / control_columns;

    let bg = " ".repeat(window_size.x as usize);
    app_manager.renderer.push_color(ColorType::Background, COL_BG_POPUP);
    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_BLACK);
    app_manager.renderer.draw_at(bg, &start_position);

    for label_index in 0..control_columns
    {
        if let Some(control_label) = controls.get(label_index as usize)
        {
            let position = Vector2::new(start_position.x + (control_section_width * label_index), start_position.y);
            app_manager.renderer.draw_at('[', &position);
            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED_DARK);
            app_manager.renderer.draw(key_to_char(control_label.key));
            app_manager.renderer.pop_color(ColorType::Foreground);
            app_manager.renderer.draw(format!("] {}", &control_label.description));
        }
    }

    app_manager.renderer.pop_color(ColorType::Background);
    app_manager.renderer.pop_color(ColorType::Foreground);
}

fn get_user_key() -> Option<KeyCode>
{
    let event = event::read().expect("Input Error");

    if let Some(key_event) = event.as_key_press_event()
    {
        return Some(key_event.code);
    }

    None
}

fn draw_window(renderer: &mut Out, size: &Vector2, position: &Vector2)
{
    renderer.draw_at(CORNER_TL, position);

    for _ in 0..size.x - 2
    {
        renderer.draw(FRAME_H);
    }
    renderer.draw(CORNER_TR);

    for y in 1..size.y - 1
    {
        renderer.draw_at(FRAME_V, &Vector2::new(position.x, position.y + y));
        for _ in 0..size.x - 2
        {
            renderer.draw(' ');
        }
        renderer.draw(FRAME_V);
    }

    renderer.draw_at(CORNER_BL, &Vector2::new(position.x, position.y + size.y - 1));
    for _ in 0..size.x - 2
    {
        renderer.draw(FRAME_H);
    }
    renderer.draw(CORNER_BR);
}
