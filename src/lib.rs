//   Copyright 2015 Colin Sherratt
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.



#![feature(core)]

extern crate atom;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::mem;
use std::ops::Deref;
use atom::{Atom, GetNextMut};

use std::boxed::FnBox;

pub use select::Select;
pub use barrier::Barrier;
mod select;
mod barrier;

/// Drop rules
/// This may be freed iff state is Pulsed | Dropped
/// and Waiting is Dropped
struct Inner {
    state: AtomicUsize,
    waiting: Atom<Waiting, Box<Waiting>>
}

// TODO 64bit sized, probably does not matter now
const PULSED: usize = 0x8000_0000;
const TX_DROP: usize = 0x4000_0000;
const TX_FLAGS: usize = PULSED | TX_DROP;
const REF_COUNT: usize = !TX_FLAGS;

struct Waiting {
    next: Option<Box<Waiting>>,
    wake: Wake
}

impl GetNextMut for Box<Waiting> {
    type NextPtr = Option<Box<Waiting>>;
    
    fn get_next(&mut self) -> &mut Option<Box<Waiting>> {
        &mut self.next
    }
}

enum Wake {
    Thread(thread::Thread),
    Select(select::Handle),
    Barrier(barrier::Handle),
    Callback(Box<FnBox() + Send>)
}

impl Waiting {
    fn wake(s: Box<Self>, id: usize) {
        let mut next = Some(s);
        while let Some(s) = next {
            // There must be a better way to do this...
            let s = *s;
            let Waiting { next: n, wake } = s;
            next = n;
            match wake {
                Wake::Thread(thread) => thread.unpark(),
                Wake::Select(select) => {
                    let trigger = {
                        let mut guard = select.0.lock().unwrap();
                        guard.ready.push(id);
                        guard.trigger.take()
                    };
                    trigger.map(|x| x.trigger());
                }
                Wake::Barrier(barrier) => {
                    let count = barrier.0.count.fetch_sub(1, Ordering::Relaxed);
                    if count == 1 {
                        let mut guard = barrier.0.trigger.lock().unwrap();
                        if let Some(t) = guard.take() {
                            t.trigger();
                        }
                    }
                }
                Wake::Callback(cb) => cb()
            }
        }
    }

    fn id(&self) -> usize {
        unsafe { mem::transmute(self) }
    }

    fn thread() -> Box<Waiting> {
        Box::new(Waiting {
            next: None,
            wake: Wake::Thread(thread::current())
        })
    }

    fn select(handle: select::Handle) ->Box<Waiting> {
        Box::new(Waiting{
            next: None,
            wake: Wake::Select(handle)
        })
    }

    fn barrier(handle: barrier::Handle) ->Box<Waiting> {
        Box::new(Waiting{
            next: None,
            wake: Wake::Barrier(handle)
        })
    }

    fn callback(f: Box<FnBox() + Send>) ->Box<Waiting> {
        Box::new(Waiting{
            next: None,
            wake: Wake::Callback(f)
        })
    }
}

unsafe impl Send for Trigger {}

pub struct Trigger {
    inner: *mut Inner,
    pulsed: bool
}

fn delete_inner(state: usize, inner: *mut Inner) {
    if state & REF_COUNT == 1 {
        let inner: Box<Inner> = unsafe {
            mem::transmute(inner)
        };
        drop(inner);     
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        if !self.pulsed {
            self.set(TX_DROP);
            self.wake();
        }
        let state = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(state, self.inner)
    }
}

impl Trigger {
    /// Create a Trigger from a usize. This is natrually
    /// unsafe.
    pub unsafe fn cast_from_usize(ptr: usize) -> Trigger {
        Trigger {
            inner: mem::transmute(ptr),
            pulsed: false
        }
    }

    /// Convert a trigger to a usize, This is unsafe
    /// and it will kill your kittens if you are not carful
    /// This is used for cases rare cases
    pub unsafe fn cast_to_usize(self) -> usize {
        let us = mem::transmute(self.inner);
        mem::forget(self);
        us
    }

    fn inner(&self) -> &Inner {
        unsafe { mem::transmute(self.inner) }
    }

    fn set(&self, state: usize) -> usize {
        self.inner().state.fetch_or(state, Ordering::Relaxed)
    }

    fn wake(&self) {
        let id = unsafe { mem::transmute(self.inner) };
        match self.inner().waiting.take(Ordering::Acquire) {
            None => (),
            Some(v) => Waiting::wake(v, id)
        }
    }

    /// Trigger an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn trigger(mut self) {
        self.pulsed = true;
        self.set(PULSED);
        self.wake();

        let state = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(state, self.inner);
        unsafe { mem::forget(self) }
    }
}


unsafe impl Send for Pulse {}

pub struct Pulse {
    inner: *mut Inner
}

impl Clone for Pulse {
    fn clone(&self) -> Pulse {
        self.inner().state.fetch_add(1, Ordering::Relaxed);
        Pulse { inner: self.inner }
    }
}

impl Drop for Pulse {
    fn drop(&mut self) {
        let flag = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(flag, self.inner);
    }
}

impl Pulse {
    pub fn new() -> (Pulse, Trigger) {
        let inner = Box::new(Inner {
            state: AtomicUsize::new(2),
            waiting: Atom::empty()
        });

        let inner = unsafe {mem::transmute(inner)};

        (Pulse {
            inner: inner
         },
         Trigger {
            inner: inner,
            pulsed: false
        })
    }

    fn inner(&self) -> &Inner {
        unsafe { mem::transmute(self.inner) }
    }

    // Read out the state of the Pulse
    fn state(&self) -> usize {
        self.inner().state.load(Ordering::Relaxed)
    }

    /// Check to see if the pulse is pending or not
    pub fn is_pending(&self) -> bool {
        self.state() & TX_FLAGS == 0
    }

    // Check to see if the pulse is pending or not
    fn in_use(&self) -> bool {
        let state = self.state();
        (state & REF_COUNT) != 1 || (state & TX_FLAGS) == 0
    }

    /// Add a waiter to a waitlist
    fn add_to_waitlist(&self, waiter: Box<Waiting>) -> usize {
        let id = waiter.id();

        self.inner().waiting.replace_and_set_next(
            waiter,
            Ordering::AcqRel
        );

        // if armed fire now
        if !self.is_pending() {
            if let Some(t) = self.inner().waiting.take(Ordering::Acquire) {
                Waiting::wake(t, self.id());
            }
        }
        id
    }

    /// Remove Waiter with `id` from the waitlist
    fn remove_from_waitlist(&self, id: usize) {
        let mut wl = self.inner().waiting.take(Ordering::Acquire);
        while let Some(mut w) = wl {
            let next = w.next.take();
            if w.id() != id {
                self.add_to_waitlist(w);
            }
            wl = next;
        }
    }

    /// Arm a pulse to wake 
    fn arm(self, waiter: Box<Waiting>) -> ArmedPulse {
        let id = self.add_to_waitlist(waiter);
        ArmedPulse {
            id: id,
            pulse: self
        }
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

            let p = self.clone().arm(Waiting::thread());
            if self.is_pending() {
                thread::park();
            }
            drop(p);
        }
    }

    pub fn id(&self) -> usize {
        unsafe { mem::transmute_copy(&self.inner) }
    }

    pub fn recycle(&self) -> Option<Trigger> {
        if self.in_use() {
            None
        } else {
            self.inner().state.store(2, Ordering::Relaxed);
            Some(Trigger{
                inner: self.inner,
                pulsed: false,
            })
        }
    }

    pub fn on_complete<F>(self, f: F) where F: FnOnce() + Send + 'static {
        self.arm(Waiting::callback(Box::new(f)));
    }
}

#[derive(Debug)]
pub struct WaitError(usize);

struct ArmedPulse {
    pulse: Pulse,
    id: usize
}

impl Deref for ArmedPulse {
    type Target = Pulse;

    fn deref(&self) -> &Pulse { &self.pulse }
}

impl ArmedPulse {
    fn disarm(self) -> Pulse {
        self.remove_from_waitlist(self.id);
        self.pulse
    }
}
