use std::{Box, String};
use std::{cmp, mem, ptr};
use std::cell::UnsafeCell;
use std::io::*;
use std::ops::DerefMut;
use std::rc::Rc;
use std::syscall::SysError;
use std::syscall::ENOENT;

use orbital::event::Event;
use orbital::Point;
use orbital::Size;

use super::display::Display;
use super::window::Window;

pub trait OrbitalResource {
    fn dup(&self) -> Result<Box<OrbitalResource>> {
        Err(SysError::new(ENOENT))
    }

    fn path(&self) -> Result<String> {
        Err(SysError::new(ENOENT))
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Err(SysError::new(ENOENT))
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Err(SysError::new(ENOENT))
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        Err(SysError::new(ENOENT))
    }

    fn sync(&mut self) -> Result<()> {
        Err(SysError::new(ENOENT))
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
    fn path(&self) -> Result<String> {
        Ok(format!("orbital://{}/content", self.id))
    }

    /// Read from resource
    // TODO: implement this
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Err(SysError::new(ENOENT))
    }

    /// Write to resource
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe {
            let content = &mut (*self.window_ptr).content;

            let size = cmp::min(content.size - self.seek, buf.len());
            Display::copy_run(buf.as_ptr() as usize, content.offscreen + self.seek, size);
            self.seek += size;

            Ok(size)
        }
    }
    
    /// Seek
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let end = unsafe {(*self.window_ptr).content.size}; 

        self.seek = match pos {
            SeekFrom::Start(offset) => cmp::min(end as u64, cmp::max(0, offset)) as usize,
            SeekFrom::Current(offset) => cmp::min(end as i64, cmp::max(0, self.seek as i64 + offset)) as usize,
            SeekFrom::End(offset) => cmp::min(end as i64, cmp::max(0, end as i64 + offset)) as usize,
        };

        Ok(self.seek as u64)
    }

    /// Sync the resource, should flip
    fn sync(&mut self) -> Result<()> {
        unsafe {
            (*self.window_ptr).redraw();
            Ok(())
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
    fn path(&self) -> Result<String> {
        Ok(format!("orbital://{}/title",self.id))
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            let title = (*self.window_ptr).title.clone();
            let bytes_to_read = title.len();
            if buf.len() >= bytes_to_read {
                for (src, dest) in title.bytes().zip(buf.iter_mut()) {
                    *dest = src;
                }
                Ok(bytes_to_read)
            } else {
                Err(SysError::new(ENOENT))
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe {
            (*self.window_ptr).title = String::from_utf8_lossy(buf).replace("\u{FFFD}","?");
            Ok(buf.len())
        }
    }
}

pub struct EventResource {
    pub window_ptr: *mut Window,
    // TODO: see if there's a nice way to not have to store the id
    pub id: u64,
}

impl OrbitalResource for EventResource {
    fn path(&self) -> Result<String> {
        Ok(format!("orbital://{}/events", self.id))
    }

    fn read(&mut self, buf: &mut[u8]) -> Result<usize> {
        unsafe {
            //Read events from window
            let mut i = 0;
            while buf.len() - i >= mem::size_of::<Event>() {
                match (*self.window_ptr).poll() {
                    Some(event) => {
                        ptr::write(buf.as_ptr().offset(i as isize) as *mut Event, event);
                        i += mem::size_of::<Event>();
                    }
                    None => break,
                }
            }
            Ok(i)
        }
    }
}

pub struct DimensionResource {
    pub window_ptr: *mut Window,
    // TODO: see if there's a nice way to not have to store the id
    pub id: u64,
}

impl OrbitalResource for DimensionResource {
    fn path(&self) -> Result<String> {
        Ok(format!("orbital://{}/dimensions", self.id))
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            if buf.len() >= mem::size_of::<[u64;2]>() {
                let dimensions: [u64;2] = [(*self.window_ptr).size.width as u64, (*self.window_ptr).size.height as u64];
                ptr::write(buf.as_ptr() as *mut [u64;2], dimensions);
                (*self.window_ptr).resize(Size::new(dimensions[0] as usize, dimensions[1] as usize));
                Ok(mem::size_of::<[u64;2]>())
            } else {
                Err(SysError::new(ENOENT))
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe {
            if buf.len() >= mem::size_of::<[u64;2]>() {
                let dimensions: [u64;2] = ptr::read(buf.as_ptr() as *const [u64;2]);
                (*self.window_ptr).resize(Size::new(dimensions[0] as usize, dimensions[1] as usize));
                Ok(mem::size_of::<[u64;2]>())
            } else {
                Err(SysError::new(ENOENT))
            }
        }
    }
}

/// Window resource
pub struct WindowResource {
    pub content: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub title: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub events: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub dimensions: Rc<UnsafeCell<Box<OrbitalResource>>>,
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
        let dimensions = 
            box DimensionResource {
                window_ptr: window.deref_mut(),
                id: id,
            };
        box WindowResource {
            window: window,
            content: Rc::new(UnsafeCell::new(content)),
            title: Rc::new(UnsafeCell::new(title)),
            events: Rc::new(UnsafeCell::new(events)),
            dimensions: Rc::new(UnsafeCell::new(dimensions)),
            id: id,
        }
    }
}

impl OrbitalResource for WindowResource {
    fn path(&self) -> Result<String> {
        Ok(format!("orbital://{}/", self.id))
    }

    // TODO: maybe read could return information about the window?
}
