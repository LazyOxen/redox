use std::{Box, String, Url};
use std::{cmp, mem, ptr};
use std::cell::UnsafeCell;
use std::io::*;
use std::ops::DerefMut;
use std::rc::Rc;

use orbital::event::Event;
use orbital::Point;
use orbital::Size;

use super::display::Display;
use super::window::Window;

pub trait OrbitalResource {
    fn dup(&self) -> Option<Box<OrbitalResource>> {
        None
    }

    fn path(&self) -> Option<String> {
        None
    }

    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn write(&mut self, buf: &[u8]) -> Option<usize> {
        None
    }

    fn seek(&mut self, pos: SeekFrom) -> Option<usize> {
        None
    }

    fn sync(&mut self) -> bool {
        false
    }
}

pub struct ContentResource {
    /// Pointer to the window
    pub window_ptr: *mut Window,
    /// Seek point
    pub seek: usize,
    /// Parent resource id
    // TODO: get rid of this? seems hacky
    pub id: u64,
}

impl OrbitalResource for ContentResource {
    /// Return the url of this resource
    fn path(&self) -> Option<String> {
        unsafe {
            Some(format!("orbital://{}/content", self.id))
        }
    }

    /// Read from resource
    // TODO: implement this
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        None
    }

    /// Write to resource
    fn write(&mut self, buf: &[u8]) -> Option<usize> {
        unsafe {
            let content = &mut (*self.window_ptr).content;

            let size = cmp::min(content.size - self.seek, buf.len());
            Display::copy_run(buf.as_ptr() as usize, content.offscreen + self.seek, size);
            self.seek += size;

            Some(size)
        }
    }
    
    /// Seek
    fn seek(&mut self, pos: SeekFrom) -> Option<usize> {
        let end = unsafe { (*self.window_ptr).content.size }; 

        self.seek = match pos {
            SeekFrom::Start(offset) => cmp::min(end, cmp::max(0, offset)),
            SeekFrom::Current(offset) =>
                cmp::min(end, cmp::max(0, self.seek as isize + offset) as usize),
            SeekFrom::End(offset) => cmp::min(end, cmp::max(0, end as isize + offset) as usize),
        };

        return Some(self.seek);
    }

    /// Sync the resource, should flip
    fn sync(&mut self) -> bool {
        unsafe {
            (*self.window_ptr).redraw();
            true
        }
    }
}

/// Title Resource
pub struct TitleResource {
    pub window_ptr: *mut Window,
    // TODO: try to get rid of this
    pub id: u64,
}

impl OrbitalResource for TitleResource {
    fn path(&self) -> Option<String> {
        unsafe {
            Some(format!("orbital://{}/title",self.id))
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        unsafe {
            let title = (*self.window_ptr).title.clone();
            let bytes_to_read = title.len();
            if buf.len() >= bytes_to_read {
                for (src, dest) in title.bytes().zip(buf.iter_mut()) {
                    *dest = src;
                }
                Some(bytes_to_read)
            } else {
                None
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Option<usize> {
        unsafe {
            (*self.window_ptr).title = String::from_utf8_lossy(buf).replace("\u{FFFD}","?");
            Some(buf.len())
        }
    }
}

pub struct EventResource {
    pub window_ptr: *mut Window,
    // TODO: see if there's a nice way to not have to store the id
    pub id: u64,
}

impl OrbitalResource for EventResource {
    fn path(&self) -> Option<String> {
        Some(format!("orbital://{}/events", self.id))
    }

    fn read(&mut self, buf: &mut[u8]) -> Option<usize> {
        unsafe {
            //Read events from window
            let mut i = 0;
            while buf.len() - i >= mem::size_of::<Event>() {
                match (*self.window_ptr).poll() {
                    Some(event) => {
                        unsafe { ptr::write(buf.as_ptr().offset(i as isize) as *mut Event, event) };
                        i += mem::size_of::<Event>();
                    }
                    None => break,
                }
            }
            Some(i)
        }
    }
}

/// Window resource
pub struct WindowResource {
    pub content: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub title: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub events: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub id: u64,
    window: Box<Window>,
}

impl WindowResource {
    pub fn new(top_left: Point, size: Size, title: String, id: u64) -> Box<OrbitalResource> {
         let mut window = Window::new(top_left, size, title);
        let content = 
            box ContentResource {
                window_ptr: window.deref_mut(),
                seek: 0,
                id: id
            };
        let title = 
            box TitleResource {
                window_ptr: window.deref_mut(),
                id: id,
            };
        let events = 
            box EventResource {
                window_ptr: window.deref_mut(),
                id: id,
            };
        box WindowResource {
            window: window,
            content: Rc::new(UnsafeCell::new(content)),
            title: Rc::new(UnsafeCell::new(title)),
            events: Rc::new(UnsafeCell::new(events)),
            id: id,
        }
    }
}

impl OrbitalResource for WindowResource {
    fn path(&self) -> Option<String> {
        Some(format!("orbital://{}/", self.id))
    }

    // TODO: maybe read could return information about the window?
}
