use crate::app_state::SessionField;
use chrono::NaiveDateTime;
use std::ops::Add;

pub struct Session
{
    pub description: String,
    pub tag: String,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
}

impl Clone for Session
{
    fn clone(&self) -> Self
    {
        Session::from(&self.description, &self.tag, self.start, self.end)
    }
}

impl PartialEq for Session
{
    fn eq(&self, other: &Self) -> bool
    {
        self.description == other.description && self.tag == other.tag && self.start == other.start && self.end == other.end
    }
}

impl Session
{
    pub fn from(description: &str, tag: &str, start: NaiveDateTime, end: Option<NaiveDateTime>) -> Session
    {
        Session {
            description: description.to_string(),
            tag: tag.to_string(),
            start,
            end,
        }
    }

    pub fn is_running(&self) -> bool
    {
        self.end.is_none()
    }
    
    // pub fn get_field(&self, field: &SessionField) 
    
    pub fn get_field_as_string(&self, field: &SessionField) -> String
    {
        match field
        {
            SessionField::Date => self.get_date_string(),
            SessionField::Description => self.description.clone(),
            SessionField::Tag => self.tag.clone(),
            SessionField::Start => self.get_start_time_string(),
            SessionField::End => self.get_end_time_string().unwrap_or_default(),
        }
    }

    pub fn get_date_string(&self) -> String
    {
        format!("{}", self.start.format("%d %b %y"))
    }

    pub fn get_start_time_string(&self) -> String
    {
        format!("{}", self.start.format("%H:%M:%S"))
    }

    pub fn get_end_time_string(&self) -> Option<String>
    {
        if let Some(end) = self.end
        {
            return Some(format!("{}", end.format("%H:%M:%S")));
        }

        None
    }

    pub fn get_duration_string(&self) -> Option<String>
    {
        if let Some(end) = self.end
        {
            let duration = end - self.start;

            let secs_per_minute: i64 = 60;
            let secs_per_hour: i64 = 3600;

            let hours = duration.num_hours();
            let minutes = duration.num_minutes() - hours * secs_per_minute;
            let seconds = duration.num_seconds() - hours * secs_per_hour - minutes * secs_per_minute;

            return Some(format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
        }

        None
    }

    pub fn construct_db_string(&self, separator: char, format: &str) -> String
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

    pub fn set_field_from_string(&mut self, field: &SessionField, field_value: &String)
    {
        const DATE_FORMAT: &str = "%d %b %y";
        const TIME_FORMAT: &str = "%H:%M:%S";
        let datetime_format = format!("{DATE_FORMAT} {TIME_FORMAT}");

        match field
        {
            SessionField::Date =>
            {
                let new_date_string = field_value;
                let start_time_string = self.get_start_time_string();
                let new_datetime_string = format!("{start_time_string} {new_date_string}");

                if let Ok(new_date) = NaiveDateTime::parse_from_str(&new_datetime_string, &datetime_format)
                {
                    let delta = new_date - self.start;
                    self.start = self.start.add(delta);

                    if let Some(end) = self.end
                    {
                        self.end = Some(end.add(delta));
                    }
                }
            }
            SessionField::Description =>
            {
                let description = field_value.trim();

                if !description.is_empty()
                {
                    self.description = String::from(description);
                }
            }
            SessionField::Tag =>
            {
                let tag = field_value.trim();

                if !tag.is_empty()
                {
                    self.tag = String::from(tag);
                }
            }
            SessionField::Start =>
            {
                let new_start_time_string = field_value;
                let date_string = self.get_date_string();
                let new_datetime_string = format!("{new_start_time_string} {date_string}");

                if let Ok(new_start_time) = NaiveDateTime::parse_from_str(&new_datetime_string, &datetime_format)
                {
                    let delta = new_start_time - self.start;
                    self.start = self.start.add(delta);
                }
            }
            SessionField::End =>
            {
                let new_end_time_string = field_value;
                let date_string = self.get_date_string();
                let new_datetime_string = format!("{new_end_time_string} {date_string}");

                if let Ok(new_end_time) = NaiveDateTime::parse_from_str(&new_datetime_string, &datetime_format)
                {
                    if let Some(end) = self.end
                    {
                        let delta = new_end_time - self.start;
                        self.end = Some(end.add(delta));
                    }
                }
            }
        }
    }
}
