use std::Box;
use std::fs::File;
use std::io::*;
use std::mem;
use std::ptr;
use std::slice;
use std::syscall::sys_yield;
use std::String;
use std::ToString;
use std::to_num::ToNum;
use std::Vec;

use super::Event;
use super::Color;

/// A window
pub struct Window {
    /// The x coordinate of the window
    x: isize,
    /// The y coordinate of the window
    y: isize,
    /// The width of the window
    w: usize,
    /// The height of the window
    h: usize,
    /// The title of the window
    t: String,
    /// The input scheme
    file: File,
    /// Window title resource
    title_: File,
    /// Window content resource
    content: File,
    /// Window events resource
    events: File,
    /// Window dimensions resource
    dimensions: File,
    /// Font file
    font: Vec<u8>,
    /// Window data
    data: Vec<u32>,
}

impl Window {
    /// Create a new window
    pub fn new(x: isize, y: isize, w: usize, h: usize, title: &str) -> Option<Box<Self>> {
        let mut font = Vec::new();
        if let Ok(mut font_file) = File::open("file:/ui/unifont.font") {
            font_file.read_to_end(&mut font);
        }

        // TODO: make this sane
        match File::open(&format!("orbital:///{}/{}/{}/{}/{}", x, y, w, h, title)) {
            Ok(file) => {
                let window_path = file.path().unwrap().to_string();
                let _title = &format!("{}title", window_path);
                let _content = &format!("{}content", window_path);
                let _events = &format!("{}events", window_path);
                let _dimensions= &format!("{}dimensions", window_path);
                let title_file = File::open(_title);
                let content_file = File::open(_content);
                let events_file = File::open(_events);
                let dimensions_file = File::open(_dimensions);
                if title_file.is_ok() && 
                   content_file.is_ok() &&
                   events_file.is_ok() &&
                   dimensions_file.is_ok() {
                    Some(box Window {
                        x: x,
                        y: y,
                        w: w,
                        h: h,
                        t: title.to_string(),
                        file: file,
                        title_: title_file.unwrap(),
                        content: content_file.unwrap(),
                        events: events_file.unwrap(),
                        dimensions: dimensions_file.unwrap(),
                        font: font,
                        data: vec![0; w * h * 4],
                    })
                } else {
                    None
                }
            },
            Err(_)=> None
        }
    }

    //TODO: Replace with smarter mechanism, maybe a move event?
    pub fn sync_path(&mut self) {
        if let Ok(path) = self.file.path() {
            //orbital://x/y/w/h/t
            if let Some(path_str) = path.to_str() {
                let parts: Vec<&str> = path_str.split('/').collect();
                if let Some(x) = parts.get(3) {
                    self.x = x.to_num_signed();
                }
                if let Some(y) = parts.get(4) {
                    self.y = y.to_num_signed();
                }
                if let Some(w) = parts.get(5) {
                    self.w = w.to_num();
                }
                if let Some(h) = parts.get(6) {
                    self.h = h.to_num();
                }
            }
        }
    }

    /// Get x
    //TODO: Sync with window movements
    pub fn x(&self) -> isize {
        self.x
    }

    /// Get y
    //TODO: Sync with window movements
    pub fn y(&self) -> isize {
        self.y
    }

    pub fn size(&mut self) -> [u64;2] {
        let mut dims: Vec<u8> = Vec::new();
        self.dimensions.read_to_end(&mut dims);
        unsafe { ptr::read((&dims).as_ptr() as *const [u64;2]) }
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        let dims = [width, height];
        self.w = width as usize;
        self.h = height as usize;
        self.data.resize( (width*height*4) as usize, 0);
        unsafe {
           self.dimensions.write(&mem::transmute::<[u64; 2], [u8; 16]>(dims)[..]);
        }
    }

    /// Get width
    pub fn width(&self) -> usize {
        self.w
    }

    /// Get height
    pub fn height(&self) -> usize {
        self.h
    }

    /// Get title
    pub fn title(&self) -> String {
        self.t.clone()
    }

    /// Set title
    pub fn set_title(&mut self, title: &str) {
        self.t = String::new() + title;
        self.title_.write(title.as_bytes());
    }

    /// Draw a pixel
    pub fn pixel(&mut self, x: isize, y: isize, color: Color) {
        if x >= 0 && y >= 0 && x < self.w as isize && y < self.h as isize {
            let offset = y as usize * self.w + x as usize;
            self.data[offset] = color.data;
        }
    }

    /// Draw a character, using the loaded font
    pub fn char(&mut self, x: isize, y: isize, c: char, color: Color) {
        let mut offset = (c as usize) * 16;
        for row in 0..16 {
            let row_data;
            if offset < self.font.len() {
                row_data = self.font[offset];
            } else {
                row_data = 0;
            }

            for col in 0..8 {
                let pixel = (row_data >> (7 - col)) & 1;
                if pixel > 0 {
                    self.pixel(x + col as isize, y + row as isize, color);
                }
            }
            offset += 1;
        }
    }

    //TODO move, resize, set_title

    /// Set entire window to a color
    // TODO: Improve speed
    #[allow(unused_variables)]
    pub fn set(&mut self, color: Color) {
        let w = self.w;
        let h = self.h;
        self.rect(0, 0, w, h, color);
    }

    /// Draw rectangle
    // TODO: Improve speed
    #[allow(unused_variables)]
    pub fn rect(&mut self, start_x: isize, start_y: isize, w: usize, h: usize, color: Color) {
        for y in start_y..start_y + h as isize {
            for x in start_x..start_x + w as isize {
                self.pixel(x, y, color);
            }
        }
    }

    /// Display an image
    //TODO: Improve speed
    pub fn image(&mut self, start_x: isize, start_y: isize, w: usize, h: usize, data: &[Color]) {
        let mut i = 0;
        for y in start_y..start_y + h as isize {
            for x in start_x..start_x + w as isize {
                if i < data.len() {
                    self.pixel(x, y, data[i])
                }
                i += 1;
            }
        }
    }

    /// Poll for an event
    //TODO: clean this up
    pub fn poll(&mut self) -> Option<Event> {
        let mut event = Event::new();
        let event_ptr: *mut Event = &mut event;
        loop {
            match self.events.read(&mut unsafe {
                slice::from_raw_parts_mut(event_ptr as *mut u8, mem::size_of::<Event>())
            }) {
                Ok(0) => unsafe { sys_yield() },
                Ok(_) => return Some(event),
                Err(_) => return None,
            }
        }
    }

    /// Flip the window buffer
    pub fn sync(&mut self) -> bool {
        self.content.seek(SeekFrom::Start(0));
        self.content.write(& unsafe {
            slice::from_raw_parts(self.data.as_ptr() as *const u8, self.data.len() * mem::size_of::<u32>())
        });
        //return self.file.sync_all().is_ok();
        return self.content.sync_all().is_ok();
    }

    /// Return a iterator over events
    pub fn event_iter<'a>(&'a mut self) -> EventIter<'a> {
        EventIter {
            window: self,
        }
    }
}

/// Event iterator
pub struct EventIter<'a> {
    window: &'a mut Window,
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.window.poll()
    }
}
