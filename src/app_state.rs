use chrono::NaiveDateTime;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum CommandState
{
    Idle,
    New(SessionInputState),
    Modify(SessionModifyState),
    End,
    Quitting,
}

#[derive(PartialEq, Copy, Clone)]
pub enum SessionInputState
{
    Description(ConfirmOpen),
    Tag(TagInputState),
}

#[derive(Clone)]
pub enum SessionModifyState
{
    Edit(SessionEditState),
    Continue(ConfirmOpen),
    Delete(ConfirmOpen),
}

#[derive(Clone)]
pub enum SessionEditState
{
    Browse,
    EditFields(SessionFieldEditState),
    Confirm,
}

#[derive(Clone)]
pub enum SessionFieldEditState
{
    Browse,
    Editing,
}

#[derive(Clone)]
pub enum SessionField
{
    Date(NaiveDateTime),
    Description(String),
    Tag(String),
    Start(NaiveDateTime),
    End(Option<NaiveDateTime>),
    None
}

#[derive(PartialEq, Copy, Clone)]
pub enum TagInputState
{
    Select,
    New,
    Delete(ConfirmOpen),
}

#[derive(PartialEq, Copy, Clone)]
pub enum ConfirmOpen
{
    Yes,
    No,
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
            CommandState::New(input_field) =>
            {
                write!(f, "Input: {}", input_field)
            }
            CommandState::Modify(_) =>
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
impl Display for SessionInputState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            SessionInputState::Description(_) =>
            {
                write!(f, "Description")
            }
            SessionInputState::Tag(tag_edit_state) =>
            {
                write!(f, "Tag: {}", tag_edit_state)
            }
        }
    }
}
impl Display for TagInputState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            TagInputState::Select =>
            {
                write!(f, "Select")
            }
            TagInputState::New =>
            {
                write!(f, "New")
            }
            TagInputState::Delete(_) =>
            {
                write!(f, "Delete")
            }
        }
    }
}
