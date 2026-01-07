use crate::error::{Error, Result};
use crate::parse::parse_sse_event;
use crate::types::SearchEvent;
use bytes::Bytes;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

const EVENT_MESSAGE_PREFIX: &str = "event: message\r\n";
const EVENT_END_OF_STREAM_PREFIX: &str = "event: end_of_stream\r\n";
const DATA_PREFIX: &str = "data: ";

pin_project_lite::pin_project! {
    pub struct SseStream<S> {
        #[pin]
        inner: S,
        buffer: String,
        finished: bool,
    }
}

impl<S> SseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, rquest::Error>>,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
            finished: false,
        }
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
                    if let Ok(text) = std::str::from_utf8(&chunk) {
                        this.buffer.push_str(text);
                    }
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
fn try_parse_event(buffer: &mut String, finished: &mut bool) -> Option<Result<SearchEvent>> {
    let delimiter = "\r\n\r\n";

    if let Some(pos) = buffer.find(delimiter) {
        let event_str = buffer[..pos].to_string();
        buffer.drain(..pos + delimiter.len());

        if event_str.starts_with(EVENT_END_OF_STREAM_PREFIX) {
            *finished = true;
            return None;
        }

        if let Some(after_event) = event_str.strip_prefix(EVENT_MESSAGE_PREFIX) {
            if let Some(data_start) = after_event.find(DATA_PREFIX) {
                let json_str = &after_event[data_start + DATA_PREFIX.len()..];
                return Some(parse_sse_event(json_str));
            }
        }
    }

    None
}
