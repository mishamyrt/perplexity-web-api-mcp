use crate::error::{Error, Result};
use crate::parse::parse_sse_event;
use crate::types::SearchEvent;
use bytes::{Bytes, BytesMut};
use futures_util::Stream;
use memchr::memmem;
use std::pin::Pin;
use std::task::{Context, Poll};

const EVENT_MESSAGE_PREFIX: &[u8] = b"event: message\r\n";
const EVENT_END_OF_STREAM_PREFIX: &[u8] = b"event: end_of_stream\r\n";
const DATA_PREFIX: &[u8] = b"data: ";
const DELIMITER: &[u8] = b"\r\n\r\n";

pin_project_lite::pin_project! {
    pub struct SseStream<S> {
        #[pin]
        inner: S,
        buffer: BytesMut,
        finished: bool,
    }
}

impl<S> SseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, rquest::Error>>,
{
    pub fn new(inner: S) -> Self {
        Self { inner, buffer: BytesMut::new(), finished: false }
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, rquest::Error>>,
{
    type Item = Result<SearchEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.finished {
            return Poll::Ready(None);
        }

        loop {
            if let Some(event) = try_parse_event(this.buffer, this.finished) {
                return Poll::Ready(Some(event));
            }

            if *this.finished {
                return Poll::Ready(None);
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    this.buffer.extend_from_slice(&chunk);
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(Error::Http(e))));
                }
                Poll::Ready(None) => {
                    *this.finished = true;
                    if this.buffer.is_empty() {
                        return Poll::Ready(None);
                    }
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

#[allow(clippy::collapsible_if)]
fn try_parse_event(buffer: &mut BytesMut, finished: &mut bool) -> Option<Result<SearchEvent>> {
    let finder = memmem::Finder::new(DELIMITER);

    if let Some(pos) = finder.find(buffer) {
        let event_bytes = buffer.split_to(pos + DELIMITER.len());
        let event_data = &event_bytes[..pos];

        // Check for end of stream event
        if event_data.starts_with(EVENT_END_OF_STREAM_PREFIX) {
            *finished = true;
            return None;
        }

        // Check for message event
        if event_data.starts_with(EVENT_MESSAGE_PREFIX) {
            let after_event = &event_data[EVENT_MESSAGE_PREFIX.len()..];
            if let Some(data_start) = memmem::find(after_event, DATA_PREFIX) {
                let json_bytes = &after_event[data_start + DATA_PREFIX.len()..];
                // Validate UTF-8 and parse
                return match std::str::from_utf8(json_bytes) {
                    Ok(json_str) => Some(parse_sse_event(json_str)),
                    Err(_) => Some(Err(Error::InvalidUtf8)),
                };
            }
        }
    }

    None
}
