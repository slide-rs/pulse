extern crate atom;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::mem;
use atom::Atom;

pub use select::Select;
mod select;

/// Drop rules
/// This may be freed iff state is Pulsed | Dropped
/// and Waiting is Dropped
struct Inner {
    state: AtomicUsize,
    waiting: Atom<Waiting, Box<Waiting>>
}

const PULSED: usize = 1;
const TX_DROPPED: usize = 2;
const RX_DROPPED: usize = 4;


pub enum Waiting {
    Thread(thread::Thread),
    Select(select::Handle)
}

impl Waiting {
    fn trigger(s: Box<Self>, id: usize) {
        match *s {
            Waiting::Thread(thread) => thread.unpark(),
            Waiting::Select(select) => {
                let trigger = {
                    let mut guard = select.0.lock().unwrap();
                    guard.ready.push(id);
                    guard.trigger.take()
                };
                trigger.map(|x| x.pulse());
            }
        }        
    }

    pub fn thread() -> Waiting {
        Waiting::Thread(thread::current())
    }
}

pub struct Trigger {
    pulsed: bool,
    inner: Arc<Inner>
}

impl Drop for Trigger {
    fn drop(&mut self) {
        self.set(TX_DROPPED);
        if !self.pulsed {
            self.wake();
        }
    }
}

impl Trigger {
    fn set(&self, state: usize) {
        self.inner.state.fetch_or(state, Ordering::Relaxed);
    }

    fn wake(&self) {
        let id = unsafe { mem::transmute_copy(&self.inner) };
        match self.inner.waiting.take(Ordering::Acquire) {
            None => (),
            Some(v) => Waiting::trigger(v, id)
        }
    }

    /// Trigger an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn pulse(mut self) {
        self.set(PULSED);
        self.wake();
        self.pulsed = true;
    }
}

pub struct Pulse(Arc<Inner>);

impl Pulse {
    pub fn new() -> (Pulse, Trigger) {
        let inner = Arc::new(Inner {
            state: AtomicUsize::new(0),
            waiting: Atom::empty()
        });

        (Pulse(inner.clone()),
         Trigger {
            inner: inner,
            pulsed: false
        })
    }

    // Read out the state of the Pulse
    fn state(&self) -> usize {
        self.0.state.load(Ordering::Relaxed)
    }

    // Check to see if the pulse is pending or not
    pub fn is_pending(&self) -> bool {
        self.state() == 0
    }

    /// Arm a pulse to wake 
    pub fn arm(&self, waiter: Box<Waiting>) {
        let old = self.0.waiting.swap(
            waiter,
            Ordering::AcqRel
        );

        if old.is_some() {
            panic!("Pulse cannot be waited on by multiple clients");
        }        
    }

    /// Disarm a pulse
    pub fn disarm(&self) {
        self.0.waiting.take(Ordering::Acquire);
    }

    /// Wait for an pulse to be triggered
    ///
    /// Panics if something else is waiting on this already
    pub fn wait(&self) -> Result<(), WaitError> {
        loop {
            if !self.is_pending() {
                let state = self.state();
                return if (state & PULSED) == PULSED {
                    Ok(())
                } else {
                    Err(WaitError(state))
                };
            }

            self.arm(Box::new(Waiting::thread()));

            if self.is_pending() {
                // wait for wake
                thread::park();
            }
            self.disarm();
        }
    }

    pub fn id(&self) -> usize {
        unsafe { mem::transmute_copy(&self.0) }
    }

    pub fn recycle(&self) -> Trigger {
        if self.is_pending() {
            panic!("Attempted to recycle pending Pulse")
        }
        self.0.state.store(0, Ordering::Relaxed);
        Trigger {
            pulsed: false,
            inner: self.0.clone()
        }
    }
}

#[derive(Debug)]
pub struct WaitError(usize);