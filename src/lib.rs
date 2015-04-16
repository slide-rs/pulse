extern crate atom;

use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use std::mem;
use atom::Atom;

pub use select::Select;
mod select;


struct Inner {
    state: AtomicIsize,
    waiting: Atom<Waiting, Box<Waiting>>
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    Pending = 0is,
    Pulsed = 1is,
    Dropped = 2is
}

impl State {
    fn from_isize(v: isize) -> State {
        match v {
            0 => State::Pending,
            1 => State::Pulsed,
            2 => State::Dropped,
            v => panic!("read invalid State {}", v)
        }
    }
}

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
        if !self.pulsed {
            self.set(State::Dropped)
        }
    }
}

impl Trigger {
    fn set(&self, state: State) {
        self.inner.state.store(state as isize, Ordering::Relaxed);

        let id = unsafe { mem::transmute_copy(&self.inner) };

        match self.inner.waiting.take(Ordering::Acquire) {
            None => (),
            Some(v) => Waiting::trigger(v, id)
        }
    }

    /// Trigger an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn pulse(mut self) {
        self.set(State::Pulsed);
        self.pulsed = true;
    }
}

pub struct Pulse(Arc<Inner>);

impl Pulse {
    pub fn new() -> (Pulse, Trigger) {
        let inner = Arc::new(Inner {
            state: AtomicIsize::new(0),
            waiting: Atom::empty()
        });

        (Pulse(inner.clone()),
         Trigger {
            inner: inner,
            pulsed: false
        })
    }

    // Read out the state of the Pulse
    pub fn state(&self) -> State {
        State::from_isize(self.0.state.load(Ordering::Relaxed))
    }

    // Check to see if the pulse is pending or not
    pub fn is_pending(&self) -> bool {
        self.state() == State::Pending
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
                return match self.state() {
                    State::Pulsed => Ok(()),
                    State::Dropped => Err(WaitError(State::Dropped)),
                    State::Pending => panic!("should not have been pending")
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
        let state = self.0.state.load(Ordering::Relaxed);
        if state == State::Pending as isize {
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
pub struct WaitError(State);