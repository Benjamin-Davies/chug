use std::{
    fmt,
    io::{self, Write},
    sync::Mutex,
};

use reqwest::blocking::Response;
use terminal_size::{Height, Width, terminal_size};

pub trait ContentLength {
    fn content_length(&self) -> u64;
}

#[derive(Debug)]
pub struct Progress {
    inner: Mutex<Inner>,
}

#[derive(Debug)]
pub struct ProgressHandle<'a> {
    parent: &'a Progress,
    index: usize,
}

#[derive(Debug)]
pub struct Track<'a, R> {
    handle: &'a ProgressHandle<'a>,
    content_length: u64,
    inner: R,
}

#[derive(Debug)]
struct Inner {
    items: Vec<Item>,
    width: usize,
    last_line_count: usize,
}

#[derive(Debug)]
struct Item {
    name: String,
    progress: f32,
    finished: bool,
}

const NAME_LENGTH: usize = 20;
const FALLBACK_WIDTH: usize = 80;
const MAX_WIDTH: usize = 80;

impl Progress {
    pub fn new() -> Self {
        Progress {
            inner: Mutex::new(Inner {
                items: Vec::new(),
                width: output_width(),
                last_line_count: 0,
            }),
        }
    }

    pub fn start(&self, mut name: String) -> io::Result<ProgressHandle> {
        let mut inner = self.inner.lock().unwrap();

        if name.len() > NAME_LENGTH {
            name.truncate(NAME_LENGTH - 3);
            name.push_str("...");
        }
        while name.len() < NAME_LENGTH {
            name.push(' ');
        }

        let index;
        if let Some(finished) = inner.items.iter().position(|item| item.finished) {
            index = finished;
            inner.items[index].name = name;
            inner.items[index].progress = 0.0;
            inner.items[index].finished = false;
        } else {
            index = inner.items.len();
            inner.items.push(Item {
                name,
                progress: 0.0,
                finished: false,
            });
        }

        inner.draw()?;

        Ok(ProgressHandle {
            parent: self,
            index,
        })
    }
}

impl ProgressHandle<'_> {
    pub fn advance(&self, progress: f32) -> io::Result<()> {
        let mut inner = self.parent.inner.lock().unwrap();
        inner.items[self.index].progress += progress;

        inner.draw()?;

        Ok(())
    }

    pub fn track<R: io::Read + ContentLength>(&self, read: R) -> Track<R> {
        Track {
            handle: self,
            content_length: read.content_length(),
            inner: read,
        }
    }

    pub fn finish(mut self) -> io::Result<()> {
        let mut inner = self.parent.inner.lock().unwrap();
        inner.items[self.index].finished = true;

        inner.draw()?;

        self.index = usize::MAX;

        Ok(())
    }
}

impl Drop for ProgressHandle<'_> {
    fn drop(&mut self) {
        if self.index == usize::MAX {
            return;
        }

        let mut inner = self.parent.inner.lock().unwrap();
        inner.items[self.index].finished = true;
    }
}

impl<R: io::Read> io::Read for Track<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes = self.inner.read(buf)?;
        self.handle
            .advance(bytes as f32 / self.content_length as f32)?;
        Ok(bytes)
    }
}

impl Inner {
    fn draw(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout().lock();

        if self.last_line_count > 0 {
            // Cursor Up
            write!(stdout, "\x1b[{}A", self.last_line_count)?;
        }

        let bar_width = self.width.saturating_sub(NAME_LENGTH).saturating_sub(9);
        for item in &self.items {
            write!(stdout, "{} [", item.name)?;
            if item.finished {
                write!(stdout, "{}", "=".repeat(bar_width + 1),)?;
            } else {
                write!(
                    stdout,
                    "{}>{}",
                    "=".repeat((bar_width as f32 * item.progress).round() as usize),
                    " ".repeat(bar_width - (bar_width as f32 * item.progress).round() as usize),
                )?;
            }
            write!(stdout, "] {:>3.0}%", item.progress * 100.0)?;
            writeln!(stdout)?;
        }

        stdout.flush()?;

        self.last_line_count = self.items.len();

        Ok(())
    }
}

impl ContentLength for Response {
    fn content_length(&self) -> u64 {
        self.content_length()
            .expect("Failed to get content length of HTTP response")
    }
}

impl<R: ContentLength> ContentLength for &mut R {
    fn content_length(&self) -> u64 {
        (**self).content_length()
    }
}

pub fn print_list<T: fmt::Display>(list: impl IntoIterator<Item = T>) -> io::Result<()> {
    let mut stdout = io::stdout().lock();

    let width = output_width();
    let mut x = 0;
    for item in list {
        let s = item.to_string();
        if x > 0 && x + s.len() + 3 > width {
            writeln!(stdout, ",")?;
            x = 0;
        }

        if x == 0 {
            write!(stdout, "  ")?;
            x += 2;
        } else {
            write!(stdout, ", ")?;
            x += 2;
        }

        write!(stdout, "{s}")?;
        x += s.len();
    }
    if x > 0 {
        writeln!(stdout)?;
    }

    stdout.flush()?;

    Ok(())
}

fn output_width() -> usize {
    terminal_size()
        .map_or(FALLBACK_WIDTH, |(Width(w), Height(_))| w as usize)
        .min(MAX_WIDTH)
}
