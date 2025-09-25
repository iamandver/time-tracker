use crate::app_state::{CommandState, SessionField};
use crate::database_handler::DatabaseHandler;
use crate::io::Out;
use crate::session::Session;
use chrono::{Datelike, Timelike};
use chrono::{Local, NaiveDateTime};

pub struct AppManager
{
    pub version: String,
    pub renderer: Out,
    database_handler: DatabaseHandler,
    value_separator: char,
    date_format: String,
    pub running: bool,
    pub tags: Vec<String>,
    pub temp_tag_index: usize,
    pub selected_session_index: usize,
    pub selected_session_field: SessionField,
    pub selected_datetime_segment: usize,
    selected_tag_index: usize,
    pub sessions: Vec<Session>,
    pub state: CommandState,
    pub description_buffer: String,
    pub tag_buffer: String,
    pub session_edit_buffer: Option<Session>,
}

impl AppManager
{
    pub fn new() -> Self
    {
        let mut manager = AppManager {
            version: "0.4.6".to_string(),
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

    pub fn increment_selected_session_field(&mut self)
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

    pub fn decrement_selected_session_field(&mut self)
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

    pub fn get_selected_session_field_index(&self) -> usize
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

    pub fn get_index_of_tag(&self, tag: &String) -> usize
    {
        self.tags.iter().position(|t| t.eq(tag)).expect("Failed to retrieve tag index.")
    }

    pub fn try_start_new_session(&mut self)
    {
        self.description_buffer = self.description_buffer.trim().to_string();

        if let Some(selected_tag) = self.tags.get(self.get_selected_tag_index())
            && !self.description_buffer.is_empty()
        {
            let start = self.get_current_time();

            self.sessions.push(Session::from(&self.description_buffer, selected_tag, start, None));

            self.description_buffer.clear();
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

    pub fn try_store_tag(&mut self)
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

    pub fn set_selected_tag_index(&mut self, index: usize)
    {
        self.selected_tag_index = index;
    }

    pub fn get_selected_tag_index(&self) -> usize
    {
        self.selected_tag_index
    }

    pub fn is_last_session_still_running(&self) -> bool
    {
        if let Some(last_session) = self.sessions.last()
        {
            return last_session.is_running();
        }

        false
    }

    pub fn end_running_session(&mut self)
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

    pub fn delete_selected_session(&mut self)
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

    pub fn start_new_session_based_on_selected(&mut self)
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

    pub fn session_buffer_has_pending_changes(&self) -> bool
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

    pub fn apply_changes_to_session(&mut self)
    {
        if let Some(selected_session) = self.sessions.get_mut(self.selected_session_index)
            && let Some(edited_session) = self.session_edit_buffer.clone()
        {
            selected_session.description = edited_session.description;
            selected_session.tag = edited_session.tag;
            selected_session.start = edited_session.start;
            selected_session.end = edited_session.end;

            if !selected_session.is_running()
            {
                self.database_handler
                    .export_all_sessions(&self.sessions, self.value_separator, &self.date_format)
                    .expect("Failed to export all sessions to db.");
            }
        }
    }

    pub fn store_modified_field_to_session_buffer(&mut self)
    {
        if let Some(selected_session) = self.session_edit_buffer.as_mut()
        {
            selected_session.set_field(&self.selected_session_field);
        }
    }

    pub fn copy_selected_session_to_buffer(&mut self)
    {
        if let Some(selected_session) = self.sessions.get(self.selected_session_index)
        {
            self.session_edit_buffer = Some(selected_session.clone());
            self.selected_session_field = SessionField::Date(selected_session.start);

            self.temp_tag_index = self.get_index_of_tag(&selected_session.tag);
        }
    }

    pub fn clear_session_edit_buffer(&mut self)
    {
        self.session_edit_buffer = None;
    }
}
