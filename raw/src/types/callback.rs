use serde::de::{Deserialize, Deserializer};

use types::*;

/// This object represents a message.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CallbackQuery {
    /// Unique identifier for this query
    pub id: String,
    /// Sender
    pub from: User,
    /// Message with the callback button that originated the query. Note that message content and
    /// message date will not be available if the message is too old
    pub message: Option<Box<Message>>,
    /// Identifier of the message sent via the bot in inline mode, that originated the query.
    pub inline_message_id: Option<String>,
    /// Global identifier, uniquely corresponding to the chat to which the message with the callback
    /// button was sent. Useful for high scores in games.
    pub chat_instance: String,
    /// Data associated with the callback button. Be aware that a bad client can send arbitrary
    /// data in this field.
    pub data: Option<String>,
    /// Short name of a Game to be returned, serves as the unique identifier for the game
    pub game_short_name: Option<String>,
}

impl<'de> Deserialize<'de> for CallbackQuery {
    fn deserialize<D>(deserializer: D) -> Result<CallbackQuery, D::Error>
        where D: Deserializer<'de>
    {
        let raw: RawCallback = Deserialize::deserialize(deserializer)?;

        let id = raw.id;
        let from = raw.from.clone();
        let message = raw.message.clone();
        let inline_message_id = raw.inline_message_id;
        let chat_instance = raw.chat_instance;
        let data = raw.data;
        let game_short_name = raw.game_short_name;

        let make_callback = || {
            Ok(
                CallbackQuery {
                    id: id.into(),
                    from,
                    message,
                    inline_message_id,
                    chat_instance,
                    data,
                    game_short_name
                }
            )
        };

        make_callback()
    }
}

/// This object represents a message. Directly mapped.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize)]
pub struct RawCallback {
    /// Unique identifier for this query
    pub id: String,
    /// Sender
    pub from: User,
    /// Message with the callback button that originated the query. Note that message content and
    /// message date will not be available if the message is too old
    pub message: Option<Box<Message>>,
    /// Identifier of the message sent via the bot in inline mode, that originated the query.
    pub inline_message_id: Option<String>,
    /// Global identifier, uniquely corresponding to the chat to which the message with the callback
    /// button was sent. Useful for high scores in games.
    pub chat_instance: String,
    /// Data associated with the callback button. Be aware that a bad client can send arbitrary
    /// data in this field.
    pub data: Option<String>,
    /// Short name of a Game to be returned, serves as the unique identifier for the game
    pub game_short_name: Option<String>
}
