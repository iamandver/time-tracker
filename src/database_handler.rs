use crate::session::Session;
use chrono::NaiveDateTime;
use std::env::current_exe;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub struct DatabaseHandler
{
    database_path: String,
    sessions_file_name: String,
    tags_file_name: String,
}

impl DatabaseHandler
{
    pub fn new() -> Self
    {
        let current_exe = current_exe().expect("Failed to retrieve executable path.");
        let current_path = current_exe.parent().expect("Failed to retrieve executable parent folder.");
        let database_path = current_path.join("database");

        let handler = DatabaseHandler {
            database_path: String::from(database_path.to_str().expect("Failed to parse db path string.")),
            sessions_file_name: String::from("sessions.txt"),
            tags_file_name: String::from("tags.txt"),
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

    pub fn export_session(&self, session_string: &String) -> Result<(), Box<dyn std::error::Error>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        if let Ok(mut sessions_db) = OpenOptions::new().append(true).open(sessions_path)
        {
            sessions_db.write_fmt(format_args!("\n{}", session_string))?;
        }

        self.remove_empty_lines(&self.sessions_file_name);

        Ok(())
    }

    pub fn export_all_sessions(
        &self,
        sessions: &Vec<Session>,
        value_separator: char,
        date_format: &str,
    ) -> Result<(), Box<dyn std::error::Error>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        if let Ok(mut sessions_db) = OpenOptions::new().write(true).truncate(true).open(sessions_path)
        {
            for session in sessions
            {
                let session_string = session.construct_db_string(value_separator, date_format);
                sessions_db.write_fmt(format_args!("\n{}", session_string))?;
            }
        }

        self.remove_empty_lines(&self.sessions_file_name);

        Ok(())
    }

    pub fn export_tag(&self, tag: &String) -> Result<(), Box<dyn std::error::Error>>
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

    pub fn import_sessions(&self, value_separator: char, format: &str) -> Option<Vec<Session>>
    {
        let database_path = Path::new(&self.database_path);
        let sessions_path = database_path.join(&self.sessions_file_name);

        if let Ok(sessions) = OpenOptions::new().read(true).open(sessions_path)
        {
            let lines = BufReader::new(sessions).lines().map_while(Result::ok).filter(|x| !x.is_empty()).collect::<Vec<String>>();

            return self.parse_sessions(lines, value_separator, format);
        }

        None
    }

    pub fn parse_sessions(&self, sessions: Vec<String>, value_separator: char, format: &str) -> Option<Vec<Session>>
    {
        let mut parsed_sessions = Vec::new();
        for session_string in sessions
        {
            let session_split = session_string.split(value_separator).collect::<Vec<&str>>();

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

    pub fn import_tags(&self) -> Option<Vec<String>>
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
                && let Ok(mut temp_file) = OpenOptions::new().truncate(true).write(true).create_new(true).open(temp_path.clone())
            {
                for entry in entries
                {
                    temp_file.write_fmt(format_args!("{}\n", entry)).expect("Failed to write to temp file.");
                }

                fs::rename(&temp_path, &file_path).expect("Failed renaming after removing empty lines.");
            }
        }
    }

    pub fn delete_session(&self, session_index: usize)
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
