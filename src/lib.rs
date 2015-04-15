extern crate atom;

use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use atom::Atom;

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
    Thread(thread::Thread)
}

impl Waiting {
    fn trigger(self) {
        match self {
            Waiting::Thread(thread) => thread.unpark()
        }        
    }

    pub fn thread() -> Waiting {
        Waiting::Thread(thread::current())
    }
}

pub struct Trigger(Arc<Inner>);

impl Trigger {
    fn set(&self, state: State) {
        let old = self.0.state.compare_and_swap(
            State::Pending as isize,
            state as isize,
            Ordering::Relaxed
        );

        if old == State::Pending as isize {
            match self.0.waiting.take(Ordering::Acquire) {
                None => (),
                Some(v) => v.trigger()
            }
        }
    }

    /// Trigger an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn pulse(self) { self.set(State::Pulsed) }
}

pub struct Pulse(Arc<Inner>);

impl Pulse {
    pub fn new() -> (Pulse, Trigger) {
        let inner = Arc::new(Inner {
            state: AtomicIsize::new(0),
            waiting: Atom::empty()
        });

        (Pulse(inner.clone()), Trigger(inner.clone()))
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
    pub fn wait(&self) {
        loop {
            if !self.is_pending() {
                return;
            }

            self.arm(Box::new(Waiting::thread()));

            if !self.is_pending() {
                // cleanup the state. since we set it
                self.disarm();
            } else {
                // wait for wake
                thread::park();
            }
        }
    }
}
