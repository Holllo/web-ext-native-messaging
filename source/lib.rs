#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! # web-ext-native-messaging
//!
//! WebExtension [native messaging] with [`serde_json`] as the (de)serializer.
//!
//! [native messaging]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging#app_side
//!
//! ## Reading
//!
//! In your web extension:
//!
//! ```js
//! const port = browser.runtime.connectNative('native executable');
//!
//! port.postMessage('Hey, there!');
//! ```
//!
//! Then in your native executable:
//!
//! ```rust,no_run
//! use web_ext_native_messaging::read_message;
//!
//! let message = read_message::<String>().unwrap();
//! println!("{}", message);
//! ```
//!
//! ## Writing
//!
//! In your web extension:
//!
//! ```js
//! const port = browser.runtime.connectNative('native executable');
//!
//! port.onMessage.addListener((message) => {
//!   console.log(message);
//! });
//! ```
//!
//! Then in your native executable:
//!
//! ```rust,no_run
//! use web_ext_native_messaging::write_message;
//!
//! let message = "Hey, there!".to_string();
//! write_message(&message).unwrap();
//! ```
//!
//! See the [native messaging documentation] for precise instructions on how to
//! send and receive messages.
//!
//! [native messaging documentation]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging

use std::{
  convert::TryInto,
  io::{Read, Write},
};

use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};

/// All possible errors that can happen with reading or writing messages.
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
  /// Infallible errors.
  #[error(transparent)]
  Infallible(#[from] std::convert::Infallible),
  #[error(transparent)]
  /// IO errors.
  Io(#[from] std::io::Error),
  #[error(transparent)]
  /// JSON (de)serialization errors.
  Json(#[from] serde_json::Error),
  /// Integer parsing errors.
  #[error(transparent)]
  TryFromInt(#[from] std::num::TryFromIntError),
}

/// Read message function with a generic [`Read`]er so that we can test it
/// without having to actually use standard in/out.
pub(crate) fn generic_read_message<D, R>(
  mut reader: R,
) -> Result<D, MessagingError>
where
  D: for<'a> serde::Deserialize<'a>,
  R: Read,
{
  let message_length = reader.read_u32::<NativeEndian>()?.try_into()?;
  let message_bytes = reader.take(message_length);

  serde_json::from_reader(message_bytes).map_err(Into::into)
}

/// Attempts to read a message from the program's stdin in the
/// [native messaging] format.
///
/// [native messaging]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging#app_side
pub fn read_message<D>() -> Result<D, MessagingError>
where
  D: for<'a> serde::Deserialize<'a>,
{
  let stdin = std::io::stdin();
  let stdin = stdin.lock();
  generic_read_message(stdin)
}

/// Write message function with a generic [`Write`]r so that we can test it
/// without having to actually use standard in/out.
pub(crate) fn generic_write_message<S, W>(
  message: &S,
  mut writer: W,
) -> Result<(), MessagingError>
where
  S: serde::Serialize,
  W: Write,
{
  let message_bytes = serde_json::to_vec(message)?;
  let message_length = message_bytes.len().try_into()?;

  writer.write_u32::<NativeEndian>(message_length)?;
  writer.write_all(&message_bytes)?;
  writer.flush().map_err(MessagingError::Io)
}

/// Attempts to write a message to the program's stdout in the
/// [native messaging] format.
///
/// [native messaging]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging#app_side
pub fn write_message<S>(message: &S) -> Result<(), MessagingError>
where
  S: serde::Serialize,
{
  let stdout = std::io::stdout();
  let stdout = stdout.lock();
  generic_write_message(message, stdout)
}

#[cfg(test)]
pub(crate) mod tests {
  use crate::{generic_read_message, generic_write_message, MessagingError};

  #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
  struct Message {
    text: String,
  }

  #[test]
  fn test_messaging() -> Result<(), MessagingError> {
    let test_message = Message {
      text: "This is a test".to_string(),
    };

    // Create a buffer that will act as both the reader and writer
    // (i.e. stdin and stdout).
    let mut buffer: Vec<u8> = vec![];

    // Write the message to the buffer.
    generic_write_message(&test_message, &mut buffer)?;

    // Then read the message, we get `std::io::Read` by dereferencing the
    // `Vec<u8>` to `&[u8]`.
    let message = generic_read_message::<Message, _>(&*buffer)?;

    assert_eq!(message, test_message);
    Ok(())
  }
}
