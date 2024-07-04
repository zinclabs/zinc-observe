// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// refer: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.FormatEvent.html#examples

use std::io;

use chrono::{Local, Utc};
use opentelemetry::trace::SpanBuilder;
use opentelemetry_api::trace::{SpanId, TraceContextExt, TraceId};
use serde::ser::{SerializeMap, Serializer as _};
use tracing::{Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_serde::{fields::AsMap, AsSerde};
use tracing_subscriber::{
    fmt::{
        format::{self, Writer},
        time::FormatTime,
        FmtContext, FormatEvent, FormatFields,
    },
    registry::{LookupSpan, SpanRef},
};

pub struct CustomTimeFormat;

impl FormatTime for CustomTimeFormat {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let cfg = crate::get_config();
        if cfg.log.local_time_format.is_empty() {
            write!(w, "{}", Utc::now().to_rfc3339())
        } else {
            write!(w, "{}", Local::now().format(&cfg.log.local_time_format))
        }
    }
}

pub struct O2Formatter {
    timer: CustomTimeFormat,
}

impl O2Formatter {
    pub fn new() -> Self {
        Self {
            timer: CustomTimeFormat,
        }
    }
}

impl Default for O2Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, N> FormatEvent<S, N> for O2Formatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        // Format values from the event's's metadata:
        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        self.timer.format_time(&mut writer)?;
        write!(&mut writer, " {} {}: ", meta.level(), meta.target())?;

        // Write fields on the event
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

pub struct TraceIdFormat;

impl<S, N> FormatEvent<S, N> for TraceIdFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        let mut visit = || {
            let mut serializer = serde_json::Serializer::new(WriteAdaptor::new(&mut writer));
            let mut serializer = serializer.serialize_map(None)?;
            serializer.serialize_entry("timestamp", &Utc::now().to_rfc3339())?;
            serializer.serialize_entry("level", &meta.level().as_serde())?;
            serializer.serialize_entry("fields", &event.field_map())?;
            serializer.serialize_entry("target", meta.target())?;

            if let Some(mut span_ref) = ctx.lookup_current() {
                loop {
                    if let Some(builder) = span_ref.extensions().get::<SpanBuilder>() {
                        if let Some(trace_id) = builder.trace_id {
                            serializer.serialize_entry("trace_id", &trace_id.to_bytes())?;
                            break;
                        }
                    }
                    if let Some(parent) = span_ref.parent() {
                        span_ref = parent;
                    } else {
                        break;
                    }
                }
            }

            serializer.end()
        };

        visit().map_err(|_| std::fmt::Error)?;
        writeln!(writer)
    }
}

pub struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn std::fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    pub fn new(fmt_write: &'a mut dyn std::fmt::Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
