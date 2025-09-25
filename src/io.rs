use crossterm::cursor;
use crossterm::style;
use crossterm::style::{Color, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{terminal, QueueableCommand};
use std::fmt::{Display, Formatter};
use std::io::{stdout, Stdout, Write};

pub enum ColorType
{
    Foreground,
    Background,
}
#[derive(Debug)]
pub struct Vector2
{
    pub x: u16,
    pub y: u16,
}

impl Vector2
{
    pub fn new(x: u16, y: u16) -> Self
    {
        Vector2 {
            x,
            y,
        }
    }
}

impl From<(u16, u16)> for Vector2
{
    fn from(value: (u16, u16)) -> Self
    {
        Self::new(value.0, value.1)
    }
}
impl Display for Vector2
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "( {}, {} )", self.x, self.y)
    }
}

pub struct Out
{
    stdout: Stdout,
    foreground_color_stack: Vec<u8>,
    background_color_stack: Vec<u8>,
}

impl Out
{
    pub fn new() -> Out
    {
        let out = Out {
            stdout: stdout(),
            foreground_color_stack: vec![],
            background_color_stack: vec![],
        };

        enable_raw_mode().expect("enable_raw_mode() failed.");

        out
    }

    pub fn clear_screen(&mut self)
    {
        self.stdout
            .queue(terminal::Clear(terminal::ClearType::All))
            .expect("Clear all failed.")
            .queue(cursor::Hide)
            .expect("Hiding cursor failed.")
            .queue(terminal::DisableLineWrap)
            .expect("Disable line wrap failed.");

        self.render();
    }

    pub fn get_terminal_size(&self) -> Vector2
    {
        Vector2::from(terminal::size().expect("get_terminal_size() failed."))
    }

    pub fn render(&mut self)
    {
        self.stdout.flush().unwrap();
    }

    pub fn push_color(&mut self, color_type: ColorType, ansi_value: u8)
    {
        match color_type
        {
            ColorType::Foreground =>
            {
                self.foreground_color_stack.push(ansi_value);
                self.set_foreground_color(Color::AnsiValue(ansi_value));
            }
            ColorType::Background =>
            {
                self.background_color_stack.push(ansi_value);
                self.set_background_color(Color::AnsiValue(ansi_value));
            }
        }
    }

    pub fn pop_color(&mut self, color_type: ColorType)
    {
        match color_type
        {
            ColorType::Foreground =>
            {
                assert!(!self.foreground_color_stack.is_empty());
                self.foreground_color_stack.pop();

                let color = if let Some(color) = self.foreground_color_stack.last()
                {
                    Color::AnsiValue(*color)
                }
                else
                {
                    Color::Reset
                };

                self.set_foreground_color(color);
            }
            ColorType::Background =>
            {
                assert!(!self.background_color_stack.is_empty());
                self.background_color_stack.pop();

                let color = if let Some(color) = self.background_color_stack.last()
                {
                    Color::AnsiValue(*color)
                }
                else
                {
                    Color::Reset
                };

                self.set_background_color(color);
            }
        }
    }

    fn set_foreground_color(&mut self, color: Color) -> &mut Self
    {
        self.stdout.queue(SetForegroundColor(color)).expect("set_foreground_color() failed.");

        self
    }

    fn set_background_color(&mut self, color: Color) -> &mut Self
    {
        self.stdout.queue(SetBackgroundColor(color)).expect("set_background_color() failed.");

        self
    }

    pub fn check_color_stacks(&self)
    {
        assert!(self.foreground_color_stack.is_empty() && self.background_color_stack.is_empty());
    }

    pub fn go_to_position(&mut self, position: &Vector2) -> &mut Self
    {
        self.stdout.queue(cursor::MoveTo(position.x, position.y)).expect("go_to_position() failed.");

        self
    }

    pub fn draw<T: Display>(&mut self, sprite: T) -> &mut Self
    {
        self.stdout.queue(style::Print(sprite)).expect("draw() failed.");

        self
    }

    pub fn draw_at<T: Display>(&mut self, sprite: T, position: &Vector2) -> &mut Self
    {
        self.go_to_position(position).draw(sprite);

        self
    }

    fn clean_up(&mut self)
    {
        self.set_foreground_color(Color::Reset)
            .set_background_color(Color::Reset)
            .stdout
            .queue(cursor::Show)
            .expect("clean_up() failed.")
            .queue(terminal::Clear(terminal::ClearType::All))
            .expect("Clear all failed.")
            .queue(terminal::EnableLineWrap)
            .expect("Disable line wrap failed.")
            .queue(cursor::MoveTo(0, 0))
            .expect("Cursor move failed.");

        disable_raw_mode().expect("Disable raw mode failed.");

        self.render();
    }
}

impl Drop for Out
{
    fn drop(&mut self)
    {
        self.clean_up();
    }
}
