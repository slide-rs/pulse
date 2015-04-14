extern crate atom;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use atom::Atom;

enum State {
    ThreadWaiting(thread::Thread)
}

impl State {
    fn trigger(self) {
        match self {
            State::ThreadWaiting(thread) => thread.unpark()
        }        
    }
}

pub struct Pulse {
    fired: AtomicBool,
    state: Atom<State, Box<State>>
}

impl Pulse {
    pub fn new() -> Pulse {
        Pulse {
            fired: AtomicBool::new(false),
            state: Atom::empty()
        }
    }

    // Check if the signal has been triggered or not
    pub fn triggered(&self) -> bool {
        self.fired.load(Ordering::Relaxed)
    }

    /// Trigger an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn trigger(&self) -> bool {
        let old = self.fired.swap(true, Ordering::Relaxed);
        if old == false {
            match self.state.take(Ordering::Acquire) {
                None => (),
                Some(v) => v.trigger()
            }
        }
        old == false
    }

    /// Wait for an pulse to be triggered
    ///
    /// Panics if something else is waiting on this already
    pub fn wait(&self) {
        loop {
            if self.triggered() {
                return;
            }

            let old = self.state.swap(
                Box::new(State::ThreadWaiting(thread::current())),
                Ordering::AcqRel
            );

            if old.is_some() {
                panic!("Pulse cannot be waited on by multiple clients");
            }

            if self.triggered() {
                // cleanup the state. since we set it
                self.state.take(Ordering::Acquire);
            } else {
                // wait for wake
                thread::park();
            }
        }
    }
}
