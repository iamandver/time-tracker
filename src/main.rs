use crate::app_state::*;
use crate::database_handler::DatabaseHandler;
use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use colors::*;
use control_keys::*;
use crossterm::event;
use crossterm::event::KeyCode;
use io::{ColorType, Out, Vector2};
use session::*;
use sprites::*;
use std::cmp;
use std::cmp::PartialEq;

mod app_state;
mod colors;
mod control_keys;
mod database_handler;
mod io;
mod session;
mod sprites;

struct AppManager
{
    version: String,
    renderer: Out,
    database_handler: DatabaseHandler,
    value_separator: char,
    date_format: String,
    running: bool,
    tags: Vec<String>,
    temp_tag_index: usize,
    selected_session_index: usize,
    selected_session_field: SessionField,
    selected_datetime_segment: usize,
    selected_tag_index: usize,
    sessions: Vec<Session>,
    state: CommandState,
    description_buffer: String,
    tag_buffer: String,
    session_edit_buffer: Option<Session>,
}

impl AppManager
{
    fn new() -> Self
    {
        let mut manager = AppManager {
            version: "0.3.4".to_string(),
            renderer: Out::new(),
            database_handler: DatabaseHandler::new(),
            value_separator: ';',
            date_format: "%d-%m-%Y %H:%M:%S".to_string(),
            running: true,
            tags: Vec::new(),
            temp_tag_index: 0,
            selected_session_index: 0,
            selected_session_field: SessionField::None,
            selected_datetime_segment: 0,
            selected_tag_index: 0,
            sessions: Vec::new(),
            state: CommandState::Idle,
            description_buffer: String::new(),
            tag_buffer: String::new(),
            session_edit_buffer: None,
        };

        if let Some(sessions) = manager.database_handler.import_sessions(manager.value_separator, &manager.date_format)
        {
            manager.sessions = sessions;

            if let Some(tags) = manager.database_handler.import_tags()
            {
                manager.tags = tags;

                let last_used_tag = &manager.sessions.last().unwrap().tag;
                let tag_index = manager.get_index_of_tag(last_used_tag);

                manager.set_selected_tag_index(tag_index);
            }
        }

        manager
    }

    fn increment_selected_session_field(&mut self)
    {
        if let Some(session_buffer) = &self.session_edit_buffer
        {
            self.selected_session_field = match self.selected_session_field
            {
                SessionField::Date(_) => SessionField::Description(session_buffer.description.clone()),
                SessionField::Description(_) => SessionField::Tag(session_buffer.tag.clone()),
                SessionField::Tag(_) => SessionField::Start(session_buffer.start),
                SessionField::Start(_) | SessionField::End(_) => SessionField::End(session_buffer.end),
                SessionField::None => SessionField::None,
            }
        }
    }

    fn decrement_selected_session_field(&mut self)
    {
        if let Some(session_buffer) = &self.session_edit_buffer
        {
            self.selected_session_field = match self.selected_session_field
            {
                SessionField::Date(_) | SessionField::Description(_) => SessionField::Date(session_buffer.start),
                SessionField::Tag(_) => SessionField::Description(session_buffer.description.clone()),
                SessionField::Start(_) => SessionField::Tag(session_buffer.tag.clone()),
                SessionField::End(_) => SessionField::Start(session_buffer.start),
                SessionField::None => SessionField::None,
            }
        }
    }

    fn selected_session_field_to_index(&self) -> usize
    {
        match self.selected_session_field
        {
            SessionField::None | SessionField::Date(_) => 0,
            SessionField::Description(_) => 1,
            SessionField::Tag(_) => 2,
            SessionField::Start(_) => 3,
            SessionField::End(_) => 4,
        }
    }

    fn get_index_of_tag(&self, tag: &String) -> usize
    {
        self.tags.iter().position(|t| t.eq(tag)).expect("Failed to retrieve tag index.")
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

        if self.tag_buffer.is_empty() || self.tags.iter().any(|tag| tag.eq(&self.tag_buffer))
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
            if last_session.is_running()
            {
                last_session.end = Some(end);
                let session_string = last_session.construct_db_string(self.value_separator, &self.date_format);

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

    fn continue_selected_session(&mut self)
    {
        if let Some(session) = self.sessions.get(self.selected_session_index)
        {
            if session.is_running()
            {
                return;
            }

            let description = &session.description;
            let tag_index = self.get_index_of_tag(&session.tag);

            self.description_buffer = description.clone();
            self.set_selected_tag_index(tag_index);

            self.try_start_new_session();
        }
    }

    fn session_buffer_has_pending_changes(&self) -> bool
    {
        if let Some(selected_session) = self.sessions.get(self.selected_session_index)
        {
            if let Some(edited_session) = self.session_edit_buffer.clone()
            {
                !selected_session.eq(&edited_session)
            }
            else
            {
                false
            }
        }
        else
        {
            false
        }
    }

    fn apply_changes_to_session(&mut self)
    {
        if let Some(selected_session) = self.sessions.get_mut(self.selected_session_index)
        {
            if let Some(edited_session) = self.session_edit_buffer.clone()
            {
                selected_session.description = edited_session.description;
                selected_session.tag = edited_session.tag;
                selected_session.start = edited_session.start;
                selected_session.end = edited_session.end;
            }
        }

        self.database_handler
            .export_all_sessions(&self.sessions, self.value_separator, &self.date_format)
            .expect("Failed to export all sessions to db.");
    }

    fn store_modified_field_to_session_buffer(&mut self)
    {
        if let Some(selected_session) = self.session_edit_buffer.as_mut()
        {
            selected_session.set_field(&self.selected_session_field);
        }
    }

    fn copy_selected_session_to_buffer(&mut self)
    {
        if let Some(selected_session) = self.sessions.get(self.selected_session_index)
        {
            self.session_edit_buffer = Some(selected_session.clone());
            self.selected_session_field = SessionField::Date(selected_session.start);
        }
    }

    fn clear_session_edit_buffer(&mut self)
    {
        self.session_edit_buffer = None;
    }
}

fn debug_draw(app_manager: &mut AppManager, message: &str)
{
    let formatted_msg = format!(" {message} ");
    let window_size = app_manager.renderer.get_terminal_size();
    let debug_pos = Vector2::new(window_size.x - formatted_msg.len() as u16 - 2, app_manager.renderer.get_terminal_size().y - 2);

    app_manager.renderer.push_color(ColorType::Foreground, COL_OUTLINE_MAIN);
    app_manager.renderer.draw_at(formatted_msg, &debug_pos);
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
        match app_manager.state.clone()
        {
            CommandState::Idle => match key
            {
                KEY_NEW =>
                {
                    app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::No));
                }
                KEY_EDIT =>
                {
                    app_manager.selected_session_index = app_manager.sessions.len() - 1;
                    app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::Browse));
                }
                KEY_CONTINUE =>
                {
                    app_manager.selected_session_index = app_manager.sessions.len() - 1;
                    app_manager.state = CommandState::Modify(SessionModifyState::Continue(ConfirmOpen::No));
                }
                KEY_DELETE =>
                {
                    app_manager.selected_session_index = app_manager.sessions.len() - 1;
                    app_manager.state = CommandState::Modify(SessionModifyState::Delete(ConfirmOpen::No));
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
            CommandState::New(input_field) => match input_field
            {
                SessionInputState::Description(confirm_end_previous) => match confirm_end_previous
                {
                    ConfirmOpen::Yes =>
                    {
                        if key == KEY_YES
                        {
                            app_manager.end_running_session();
                            app_manager.try_start_new_session();
                            app_manager.state = CommandState::Idle;
                        }
                        else if key == KEY_NO || key == KEY_ESCAPE
                        {
                            app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::No));
                        }
                    }
                    ConfirmOpen::No => match key
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
                                app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::Yes));
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
                            app_manager.state = CommandState::New(SessionInputState::Tag(TagInputState::Select));
                        }
                        KeyCode::Char(character) =>
                        {
                            app_manager.description_buffer.push(character);
                        }
                        _ =>
                        {}
                    },
                },
                SessionInputState::Tag(edit_state) => match edit_state
                {
                    TagInputState::Select => match key
                    {
                        KEY_NEW =>
                        {
                            app_manager.state = CommandState::New(SessionInputState::Tag(TagInputState::New));
                        }
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::No));
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
                            app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::No));
                        }
                        _ =>
                        {}
                    },
                    TagInputState::New => match key
                    {
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::New(SessionInputState::Tag(TagInputState::Select));
                        }
                        KEY_BACKSPACE =>
                        {
                            app_manager.tag_buffer.pop();
                        }
                        KEY_ENTER =>
                        {
                            app_manager.try_store_tag();
                            app_manager.state = CommandState::New(SessionInputState::Description(ConfirmOpen::No));
                        }
                        KeyCode::Char(character) =>
                        {
                            app_manager.tag_buffer.push(character);
                        }
                        _ =>
                        {}
                    },
                    TagInputState::Delete(_) =>
                    {}
                },
            },
            CommandState::Modify(session_modify_state) => match session_modify_state
            {
                SessionModifyState::Edit(edit_state) => match edit_state
                {
                    SessionEditState::Browse => match key
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
                            app_manager.copy_selected_session_to_buffer();
                            app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(
                                SessionFieldEditState::Browse,
                            )));
                        }
                        _ =>
                        {}
                    },
                    SessionEditState::EditFields(state) => match state
                    {
                        SessionFieldEditState::Browse => match key
                        {
                            KEY_ESCAPE =>
                            {
                                if app_manager.session_buffer_has_pending_changes()
                                {
                                    app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::Confirm));
                                }
                                else
                                {
                                    app_manager.clear_session_edit_buffer();
                                    app_manager.selected_session_field = SessionField::None;
                                    app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::Browse));
                                }
                            }
                            KEY_LEFT =>
                            {
                                app_manager.decrement_selected_session_field();

                                if let SessionField::Tag(_) = &app_manager.selected_session_field
                                {
                                    let session_tag = &app_manager.session_edit_buffer.as_ref().unwrap().tag;
                                    app_manager.temp_tag_index = app_manager.get_index_of_tag(session_tag);
                                }
                            }
                            KEY_RIGHT =>
                            {
                                app_manager.increment_selected_session_field();

                                if let SessionField::Tag(_) = &app_manager.selected_session_field
                                {
                                    let session_tag = &app_manager.session_edit_buffer.as_ref().unwrap().tag;
                                    app_manager.temp_tag_index = app_manager.get_index_of_tag(session_tag);
                                }
                            }
                            KEY_ENTER =>
                            {
                                app_manager.selected_datetime_segment = 0;
                                app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(
                                    SessionFieldEditState::Editing,
                                )));
                            }
                            _ =>
                            {}
                        },
                        SessionFieldEditState::Editing =>
                        {
                            match key
                            {
                                KEY_ESCAPE =>
                                {
                                    app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(
                                        SessionFieldEditState::Browse,
                                    )));
                                }
                                KEY_ENTER =>
                                {
                                    app_manager.store_modified_field_to_session_buffer();

                                    app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(
                                        SessionFieldEditState::Browse,
                                    )));
                                }
                                _ =>
                                {}
                            }

                            match &mut app_manager.selected_session_field
                            {
                                SessionField::Date(date_buffer) => match key
                                {
                                    KEY_UP =>
                                    {}
                                    KEY_DOWN =>
                                    {}
                                    KEY_LEFT =>
                                    {}
                                    KEY_RIGHT =>
                                    {}
                                    _ =>
                                    {}
                                },
                                SessionField::Description(description_buffer) => match key
                                {
                                    KEY_BACKSPACE =>
                                    {
                                        description_buffer.pop();
                                    }
                                    KeyCode::Char(character) =>
                                    {
                                        description_buffer.push(character);
                                    }
                                    _ =>
                                    {}
                                },

                                SessionField::Tag(tag_buffer) => match key
                                {
                                    KEY_UP =>
                                    {
                                        if app_manager.temp_tag_index > 0
                                        {
                                            app_manager.temp_tag_index -= 1;
                                        }

                                        tag_buffer.clone_from(&app_manager.tags[app_manager.temp_tag_index]);
                                    }
                                    KEY_DOWN =>
                                    {
                                        if app_manager.temp_tag_index + 1 < app_manager.tags.len()
                                        {
                                            app_manager.temp_tag_index += 1;
                                        }

                                        tag_buffer.clone_from(&app_manager.tags[app_manager.temp_tag_index]);
                                    }
                                    _ =>
                                    {}
                                },
                                SessionField::Start(start_buffer) =>
                                {}
                                SessionField::End(end_buffer) =>
                                {}
                                SessionField::None =>
                                {}
                            }
                        }
                    },
                    SessionEditState::Confirm => match key
                    {
                        KEY_YES =>
                        {
                            app_manager.apply_changes_to_session();
                            app_manager.clear_session_edit_buffer();
                            app_manager.selected_session_field = SessionField::None;
                            app_manager.state = CommandState::Idle;
                        }
                        KEY_NO =>
                        {
                            app_manager.clear_session_edit_buffer();
                            app_manager.selected_session_field = SessionField::None;
                            app_manager.state = CommandState::Idle;
                        }
                        KEY_ESCAPE =>
                        {
                            app_manager.state = CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(
                                SessionFieldEditState::Browse,
                            )));
                        }
                        _ =>
                        {}
                    },
                },
                SessionModifyState::Continue(confirm_open) => match confirm_open
                {
                    ConfirmOpen::Yes =>
                    {
                        if key == KEY_YES
                        {
                            app_manager.continue_selected_session();
                            app_manager.state = CommandState::Idle;
                        }
                        else if key == KEY_NO || key == KEY_ESCAPE
                        {
                            app_manager.state = CommandState::Idle;
                        }
                    }
                    ConfirmOpen::No => match key
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
                            app_manager.state = CommandState::Modify(SessionModifyState::Continue(ConfirmOpen::Yes));
                        }
                        _ =>
                        {}
                    },
                },
                SessionModifyState::Delete(confirm_open) => match confirm_open
                {
                    ConfirmOpen::Yes =>
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
                    ConfirmOpen::No => match key
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
                            app_manager.state = CommandState::Modify(SessionModifyState::Delete(ConfirmOpen::Yes));
                        }
                        _ =>
                        {}
                    },
                },
            },
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
    let title_pos = Vector2::new(window_pos.x + OFFSET, window_pos.y);
    renderer.draw_at(format!(" {} ", title), &title_pos);
}

fn draw_window_shadow(renderer: &mut Out, window_size: &Vector2, window_pos: &Vector2)
{
    renderer.push_color(ColorType::Background, COL_WINDOW_SHADOW);
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
    app_manager.renderer.push_color(ColorType::Foreground, COL_OUTLINE_POPUP);

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

    let content_offset = Vector2::new(2, 1);

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
        app_manager.renderer.draw_at(section_title, &Vector2::new(*column_pos + content_offset.x, content_offset.y));
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
    app_manager.renderer.draw_at(" ".repeat(app_manager.renderer.get_terminal_size().x as usize), &Vector2::new(0, 0));
    draw_window_title(&mut app_manager.renderer, "SESSIONS", &Vector2::new(0, 0));
    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);

    for (session_index, session) in app_manager.sessions.iter().rev().enumerate()
    {
        let entry_pos_y = content_offset.y + 1 + session_index as u16;

        let selected_row = if let CommandState::Modify(_) = app_manager.state
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
            app_manager.renderer.draw_at(bg, &Vector2::new(content_offset.x, entry_pos_y));
        }

        let session = if selected_row && app_manager.session_edit_buffer.is_some()
        {
            app_manager.session_edit_buffer.as_ref().unwrap()
        }
        else
        {
            session
        };

        let start_date = session.get_date_string();
        let start_time = session.get_start_time_string();
        let end_time = session.get_end_time_string().unwrap_or(String::from("-"));
        let duration = session.get_duration_string().unwrap_or(String::from("Running"));

        let session_fields = [
            (&start_date, date_column_pos),
            (&session.description, description_column_pos),
            (&session.tag, tag_column_pos),
            (&start_time, timestamp_column_1_pos),
            (&end_time, timestamp_column_2_pos),
            // (&duration, timestamp_column_3_pos),
        ];

        for (session_field_index, (field, position)) in session_fields.iter().enumerate()
        {
            let field_pos = Vector2::new(position + content_offset.x, entry_pos_y);

            if !selected_row || session_field_index != app_manager.selected_session_field_to_index()
            {
                app_manager.renderer.draw_at(field, &field_pos);
                continue;
            }

            if let CommandState::Modify(SessionModifyState::Edit(SessionEditState::EditFields(edit_field_state))) =
                app_manager.state.clone()
            {
                match edit_field_state
                {
                    SessionFieldEditState::Browse =>
                    {
                        app_manager.renderer.push_color(ColorType::Background, COL_TEXT_HIGHLIGHT);
                        app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_BLACK);

                        app_manager.renderer.draw_at(field, &field_pos);

                        app_manager.renderer.pop_color(ColorType::Background);
                        app_manager.renderer.pop_color(ColorType::Foreground);
                    }
                    SessionFieldEditState::Editing =>
                    {
                        app_manager.renderer.push_color(ColorType::Background, COL_TEXT_RED);
                        app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_WHITE);

                        match &app_manager.selected_session_field
                        {
                            SessionField::Date(_) =>
                            {}
                            SessionField::Description(description_buffer) =>
                            {
                                app_manager.renderer.draw_at(description_buffer, &field_pos);

                                let cursor_pos_x = field_pos.x + description_buffer.len() as u16;

                                app_manager.renderer.draw_at(CURSOR, &Vector2::new(cursor_pos_x, entry_pos_y));
                            }
                            SessionField::Tag(tag_buffer) =>
                            {
                                // app_manager.renderer.draw_at(tag_buffer, &field_pos);

                                ////////

                                let dropdown_title = "EDIT TAG";
                                let tag_dropdown_pos = &field_pos;
                                let tag_dropdown_text_pos = Vector2::new(tag_dropdown_pos.x + 2, tag_dropdown_pos.y + 1);

                                if let Some(longest_tag_str) = app_manager.tags.iter().map(String::len).max()
                                {
                                    let longest_tag_str = cmp::max(longest_tag_str, dropdown_title.len() + 2) as u16;
                                    let tag_dropdown_size = Vector2::new(longest_tag_str + 8, app_manager.tags.len() as u16 + 2);

                                    draw_window(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);
                                    draw_window_shadow(&mut app_manager.renderer, &tag_dropdown_size, tag_dropdown_pos);

                                    // app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
                                    // app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
                                    draw_window_title(&mut app_manager.renderer, dropdown_title, tag_dropdown_pos);
                                    // app_manager.renderer.pop_color(ColorType::Background);
                                    // app_manager.renderer.pop_color(ColorType::Foreground);

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
                                            // app_manager.renderer.push_color(ColorType::Background, COL_TEXT_BLACK);
                                            // app_manager.renderer.push_color(ColorType::Foreground, COL_BG_POPUP);
                                        }

                                        let right_pad = longest_tag_str as usize + 1;
                                        app_manager.renderer.draw_at(
                                            format!(" {} {:<pad$}", arrow, tag, pad = right_pad),
                                            &Vector2::new(tag_dropdown_text_pos.x, tag_dropdown_text_pos.y + index as u16),
                                        );

                                        if selected_row
                                        {
                                            // app_manager.renderer.pop_color(ColorType::Background);
                                            // app_manager.renderer.pop_color(ColorType::Foreground);
                                        }
                                    }
                                }

                                ////////


                            }
                            SessionField::Start(_) =>
                            {}
                            SessionField::End(_) =>
                            {}
                            SessionField::None =>
                            {}
                        }

                        app_manager.renderer.pop_color(ColorType::Foreground);
                        app_manager.renderer.pop_color(ColorType::Background);
                    }
                }
            }
            else
            {
                app_manager.renderer.draw_at(field, &Vector2::new(position + content_offset.x, entry_pos_y));
            }
        }

        if session.is_running()
        {
            app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_RED);
        }
        app_manager.renderer.draw_at(duration, &Vector2::new(timestamp_column_3_pos + content_offset.x, entry_pos_y));
        if session.is_running()
        {
            app_manager.renderer.pop_color(ColorType::Foreground);
        }

        if selected_row
        {
            app_manager.renderer.pop_color(ColorType::Background);
        }
    }


    // draw selected row
    let selected_session_index = app_manager.sessions.len() - 1 - app_manager.selected_session_index;
    let selected_session_pos_y = content_offset.y + 1 + selected_session_index as u16;



    ////////

    match app_manager.state.clone()
    {
        CommandState::Idle =>
        {}
        CommandState::New(input_field) =>
        {
            let input_field_size = Vector2::new(terminal_size.x - 32, 3);
            let input_field_pos = Vector2::new((terminal_size.x - input_field_size.x) / 2, (terminal_size.y - input_field_size.y) / 2);

            app_manager.renderer.push_color(ColorType::Background, COL_BG_POPUP);
            app_manager.renderer.push_color(ColorType::Foreground, COL_OUTLINE_POPUP);

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
                SessionInputState::Description(confirm_end_previous) => match confirm_end_previous
                {
                    ConfirmOpen::Yes =>
                    {
                        draw_yes_no_popup(app_manager, "END RUNNING SESSION?");
                    }
                    ConfirmOpen::No =>
                    {
                        let cursor_pos_x =
                            description_input_pos.x + (description_input_label.len() + app_manager.description_buffer.len()) as u16;

                        app_manager.renderer.draw_at(CURSOR, &Vector2::new(cursor_pos_x, text_pos_y));
                    }
                },
                SessionInputState::Tag(edit_state) =>
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
                        TagInputState::Select =>
                        {}
                        TagInputState::New =>
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
                        TagInputState::Delete(_) =>
                        {}
                    }
                }
            }

            app_manager.renderer.pop_color(ColorType::Background);
            app_manager.renderer.pop_color(ColorType::Foreground);
        }
        CommandState::Modify(session_edit_state) => match session_edit_state
        {
            SessionModifyState::Edit(edit_state) =>
            {
                draw_session_selection_line(app_manager, &content_offset, "EDT");

                match edit_state
                {
                    SessionEditState::Browse =>
                    {}
                    SessionEditState::EditFields(field_state) => match field_state
                    {
                        SessionFieldEditState::Browse =>
                        {}
                        SessionFieldEditState::Editing =>
                        {}
                    },
                    SessionEditState::Confirm =>
                    {
                        draw_yes_no_popup(app_manager, "ACCEPT CHANGES?");
                    }
                }
            }
            SessionModifyState::Continue(confirm_open) =>
            {
                draw_session_selection_line(app_manager, &content_offset, "CPY");

                match confirm_open
                {
                    ConfirmOpen::Yes =>
                    {
                        draw_yes_no_popup(app_manager, "COPY AND START SESSION?");
                    }
                    ConfirmOpen::No =>
                    {}
                }
            }
            SessionModifyState::Delete(confirm_open) =>
            {
                draw_session_selection_line(app_manager, &content_offset, "DEL");

                match confirm_open
                {
                    ConfirmOpen::Yes =>
                    {
                        draw_yes_no_popup(app_manager, "CONFIRM DELETE");
                    }
                    ConfirmOpen::No =>
                    {}
                }
            }
        },
        CommandState::End =>
        {
            draw_yes_no_popup(app_manager, "END SESSION?");
        }
        CommandState::Quitting =>
        {
            draw_yes_no_popup(app_manager, "REALLY QUIT?");
        }
    }

    let version = format!("Version {}", &app_manager.version);
    debug_draw(app_manager, &version);

    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);

    draw_control_panel(app_manager);

    app_manager.renderer.render();
}

fn draw_session_selection_line(app_manager: &mut AppManager, content_offset: &Vector2, command_label: &str)
{
    let row = (app_manager.sessions.len() - app_manager.selected_session_index - content_offset.y as usize) as u16;

    app_manager.renderer.push_color(ColorType::Background, COL_TEXT_DIM);
    app_manager.renderer.push_color(ColorType::Foreground, COL_TEXT_HIGHLIGHT);
    app_manager.renderer.draw_at(format!(" {}", command_label), &Vector2::new(content_offset.x - 1, 2 + row));
    app_manager.renderer.pop_color(ColorType::Foreground);
    app_manager.renderer.pop_color(ColorType::Background);
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
