use crossterm::event::KeyCode;

pub const KEY_NEW: KeyCode = KeyCode::Char('n');
pub const KEY_DELETE: KeyCode = KeyCode::Char('d');
pub const KEY_END: KeyCode = KeyCode::Char(' ');
pub const KEY_EDIT: KeyCode = KeyCode::Char('e');
pub const KEY_COPY: KeyCode = KeyCode::Char('c');
pub const KEY_QUIT: KeyCode = KeyCode::Char('q');
pub const KEY_ENTER: KeyCode = KeyCode::Enter;
pub const KEY_TAB: KeyCode = KeyCode::Tab;
pub const KEY_YES: KeyCode = KeyCode::Char('y');
pub const KEY_NO: KeyCode = KeyCode::Char('n');
pub const KEY_UP: KeyCode = KeyCode::Up;
pub const KEY_DOWN: KeyCode = KeyCode::Down;
pub const KEY_LEFT: KeyCode = KeyCode::Left;
pub const KEY_RIGHT: KeyCode = KeyCode::Right;
pub const KEY_BACKSPACE: KeyCode = KeyCode::Backspace;
pub const KEY_ESCAPE: KeyCode = KeyCode::Esc;

pub type Controls = Vec<Control>;

pub fn key_to_char(key: KeyCode) -> String
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
pub fn get_controls() -> Vec<Control>
{
    vec![
        Control {
            key: KEY_NEW,
            description: "new".to_string(),
        },
        Control {
            key: KEY_EDIT,
            description: "edit".to_string(),
        },
        Control {
            key: KEY_DELETE,
            description: "delete".to_string(),
        },
        Control {
            key: KEY_COPY,
            description: "copy".to_string(),
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

pub struct Control
{
    pub key: KeyCode,
    pub description: String,
}
