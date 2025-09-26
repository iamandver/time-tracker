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

    // pub fn get_field_as_string(&self, field: &SessionField) -> String
    // {
    //     match field
    //     {
    //         SessionField::Date(_) => self.get_date_string(),
    //         SessionField::Description(_) => self.description.clone(),
    //         SessionField::Tag(_) => self.tag.clone(),
    //         SessionField::Start(_) => self.get_start_time_string(),
    //         SessionField::End(_) => self.get_end_time_string().unwrap_or_default(),
    //         SessionField::None =>
    //         {
    //             panic!("Never call with variant 'None'");
    //         }
    //     }
    // }

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

        let date = format!("{}", self.start.format(date_format));
        let description = &self.description;
        let tag = &self.tag;
        let start = format!("{}", self.start.format(time_format));

        let end = self.end.expect("Cannot export ongoing session.");
        let end = format!("{}", end.format(time_format));

        format!("{date}{separator}{description}{separator}{tag}{separator}{start}{separator}{end}{separator}")
    }

    pub fn set_field(&mut self, field: &SessionField)
    {
        match field
        {
            SessionField::Date(new_date) =>
            {
                let delta = *new_date - self.start;
                self.start = self.start.add(delta);

                if let Some(end) = self.end
                {
                    self.end = Some(end.add(delta));
                }
            }
            SessionField::Description(new_description) =>
            {
                let new_description = new_description.trim();

                if !new_description.is_empty()
                {
                    self.description = String::from(new_description);
                }
            }
            SessionField::Tag(new_tag) =>
            {
                let new_tag = new_tag.trim();

                if !new_tag.is_empty()
                {
                    self.tag = String::from(new_tag);
                }
            }
            SessionField::Start(new_start) =>
            {
                self.start = *new_start;
            }
            SessionField::End(new_end) =>
            {
                self.end = *new_end;
            }
            SessionField::None =>
            {}
        }
    }
}
