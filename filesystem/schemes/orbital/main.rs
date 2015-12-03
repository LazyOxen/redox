//#![feature(rc_counts)]
use std::{Box, String, Url};
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::get_slice::GetSlice;
use std::io::*;
use std::ops::DerefMut;
use std::process::Command;
use std::rc::Rc;
use std::to_num::ToNum;
use std::u64;

use orbital::event::Event;
use orbital::Point;
use orbital::Size;

use self::session::Session;
use self::resources::*;

pub mod display;
pub mod package;
pub mod resources;
pub mod scheduler;
pub mod session;
pub mod window;

pub static mut session_ptr: *mut Session = 0 as *mut Session;
pub static mut windows_map: *mut BTreeMap<u64, Rc<UnsafeCell<Box<OrbitalResource>>>> = 0 as *mut BTreeMap<u64, Rc<UnsafeCell<Box<OrbitalResource>>>>;

/// A resource
pub struct Resource {
    pub resource: Rc<UnsafeCell<Box<OrbitalResource>>>,
    pub id: u64,
}

impl Resource {
    pub fn new (resource: Rc<UnsafeCell<Box<OrbitalResource>>>, id: u64) -> Box<Self> {
        box Resource { 
            resource: resource,
            id: id,
        }
    }

    pub fn dup(&self) -> Option<Box<Resource>> {
        None
    }

    pub fn path(&self) -> Option<String> {
        unsafe {
            (*self.resource.get()).path()
        }
    }

    /// Read from resource
    pub fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        unsafe {
            (*self.resource.get()).read(buf)
        }
    }

    /// Write to resource
    pub fn write(&mut self, buf: &[u8]) -> Option<usize> {
        unsafe {
            (*self.resource.get()).write(buf)
        }
    }

    /// Seek
    pub fn seek(&mut self, pos: SeekFrom) -> Option<usize> {
        unsafe {
            (*self.resource.get()).seek(pos)
        }
    }

    /// Sync the resource, should flip
    pub fn sync(&mut self) -> bool {
        unsafe {
            (*self.resource.get()).sync()
        }
    }
}

impl Drop for Resource {
    fn drop(&mut self) {
        unsafe {
            let reenable = scheduler::start_no_ints();
            // TODO: check the reference count before dropping
            //       this will drop the window even if the user is just dropping
            //       the title or some other resource
            // LazyOxen
            /*
            let count = Rc::strong_count(&(*windows_map)[&self.id]);
            println!("ref count: {}", count);
            if count == 1 {
                (*windows_map).remove(&self.id);
            }
            */
            (*windows_map).remove(&self.id);
            scheduler::end_no_ints(reenable);
        }
    }
}
/// A window scheme
pub struct Scheme {
    pub session: Box<Session>,
    pub next_x: isize,
    pub next_y: isize,
    pub next_window_id: u64,
    pub windows: BTreeMap<u64, Rc<UnsafeCell<Box<OrbitalResource>>>>,
}

impl Scheme {
    fn next_id(&mut self) -> Option<u64> {
        if self.next_window_id == u64::MAX {
            None
        } else {
            let id = self.next_window_id;
            self.next_window_id += 1;
            Some(id)
        }
    }

    pub fn new() -> Box<Scheme> {
        println!("- Starting Orbital");
        println!("    Console: Press F1");
        println!("    Desktop: Press F2");
        let mut ret = box Scheme {
            session: Session::new(),
            next_x: 0,
            next_y: 0,
            next_window_id: 1,
            windows: BTreeMap::new(),
        };
        unsafe { 
            session_ptr = ret.session.deref_mut();
            windows_map = &ret.windows as *const _ as  *mut BTreeMap<u64, Rc<UnsafeCell<Box<OrbitalResource>>>>;
        }
        ret
    }

    pub fn open(&mut self, url_str: &str, _: usize) -> Option<Box<Resource>> {
        // window://host/path/path/path is the path type we're working with.
        let url = Url::from_str(url_str);

        let host = url.host();
        if host.is_empty() {
            let path = url.path_parts();
            let mut pointx = match path.get(0) {
                Some(x) => x.to_num_signed(),
                None => 0,
            };
            let mut pointy = match path.get(1) {
                Some(y) => y.to_num_signed(),
                None => 0,
            };
            let size_width = match path.get(2) {
                Some(w) => w.to_num(),
                None => 100,
            };
            let size_height = match path.get(3) {
                Some(h) => h.to_num(),
                None => 100,
            };

            let mut title = match path.get(4) {
                Some(t) => t.clone(),
                None => String::new(),
            };
            for i in 5..path.len() {
                if let Some(t) = path.get(i) {
                    title = title + "/" + t;
                }
            }

            if pointx <= 0 || pointy <= 0 {
                if self.next_x > self.session.display.width as isize - size_width as isize {
                    self.next_x = 0;
                }
                self.next_x += 32;
                pointx = self.next_x;

                if self.next_y > self.session.display.height as isize - size_height as isize {
                    self.next_y = 0;
                }
                self.next_y += 32;
                pointy = self.next_y;
            }

            match self.next_id() {
                Some(id) => {
                    let window = Rc::new(UnsafeCell::new(WindowResource::new(
                                                                             Point::new(pointx, pointy),
                                                                             Size::new(size_width, size_height),
                                                                             title, 
                                                                             id)));
                    self.windows.insert(id, window.clone());
                    let resource = 
                        box Resource { 
                            resource: window,
                            id: id,
                        };
                    Some(resource)
                },
                None => None,
            }
        } else if host == "launch" {
            let path = url.path();

            unsafe {
                let reenable = scheduler::start_no_ints();

                for package in self.session.packages.iter() {
                    let mut accepted = false;
                    for accept in package.accepts.iter() {
                        if (accept.starts_with('*') &&
                            path.ends_with(&accept.get_slice(Some(1), None))) ||
                           (accept.ends_with('*') &&
                            path.starts_with(&accept.get_slice(None, Some(accept.len() - 1)))) {
                            accepted = true;
                            break;
                        }
                    }
                    if accepted {
                        if Command::new(&package.binary).arg(&path).spawn_scheme().is_none() {
                            println!("{}: Failed to launch", package.binary);
                        }
                        break;
                    }
                }

                scheduler::end_no_ints(reenable);
            }

            None
        } else if let Ok(id) = host.parse::<u64>() {
            let window = self.windows[&id].clone();
            let path = url.path_parts();
            if let Some(property) = path.get(0) {
                unsafe {
                    //let reenable = scheduler::start_no_ints();
                    let resource =
                        match &property[..] {
                            "content" => Some(Resource::new(
                                    (*(window.get() as *mut Box<WindowResource>))
                                        .content.clone() as Rc<UnsafeCell<Box<OrbitalResource>>>, id)),
                            "title" => Some(Resource::new(
                                    (*(window.get() as *mut Box<WindowResource>))
                                        .title.clone() as Rc<UnsafeCell<Box<OrbitalResource>>>, id)),
                            "events" => Some(Resource::new(
                                    (*(window.get() as *mut Box<WindowResource>))
                                        .events.clone() as Rc<UnsafeCell<Box<OrbitalResource>>>, id)),
                            "dimensions" => Some(Resource::new(
                                    (*(window.get() as *mut Box<WindowResource>))
                                        .dimensions.clone() as Rc<UnsafeCell<Box<OrbitalResource>>>, id)),
                            _ => None
                        };
                    //scheduler::end_no_ints(reenable);
                    resource
                }
            } else {
                Some(Resource::new(window, id))
            }
        } else {
            None
        }
    }

    pub fn event(&mut self, event: &Event) {
        unsafe {
            let reenable = scheduler::start_no_ints();

            self.session.event(event);

            scheduler::end_no_ints(reenable);

            self.session.redraw();
        }
    }
}

// TODO: This is a hack and it will go away
#[cold]
#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn _event(scheme: *mut Scheme, event: *const Event) {
    (*scheme).event(&*event);
}
