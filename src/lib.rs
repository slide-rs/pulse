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

use std::sync::atomic::{AtomicUsize};
use std::thread;
use std::mem;
use std::ops::Deref;
use std::sync::atomic::Ordering;
use atom::*;

use std::boxed::FnBox;

pub use select::Select;
pub use barrier::Barrier;
mod select;
mod barrier;

/// Drop rules
/// This may be freed iff state is Signald | Dropped
/// and Waiting is Dropped
struct Inner {
    state: AtomicUsize,
    waiting: Atom<Box<Waiting>>
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
                    trigger.map(|x| x.pulse());
                }
                Wake::Barrier(barrier) => {
                    let count = barrier.0.count.fetch_sub(1, Ordering::Relaxed);
                    if count == 1 {
                        let mut guard = barrier.0.trigger.lock().unwrap();
                        if let Some(t) = guard.take() {
                            t.pulse();
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

unsafe impl Send for Pulse {}

pub struct Pulse {
    inner: *mut Inner
}

fn delete_inner(state: usize, inner: *mut Inner) {
    if state & REF_COUNT == 1 {
        let inner: Box<Inner> = unsafe {
            mem::transmute(inner)
        };
        drop(inner);     
    }
}

impl Drop for Pulse {
    fn drop(&mut self) {
        self.set(TX_DROP);
        self.wake();
        let state = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(state, self.inner)
    }
}

impl Pulse {
    /// Create a Pulse from a usize. This is natrually
    /// unsafe.
    pub unsafe fn cast_from_usize(ptr: usize) -> Pulse {
        Pulse {
            inner: mem::transmute(ptr)
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

    /// Pulse an pulse, this can only occure once
    /// Returns true if this triggering triggered the pulse
    pub fn pulse(self) {
        self.set(PULSED);
        self.wake();

        let state = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(state, self.inner);
        unsafe { mem::forget(self) }
    }
}


unsafe impl Send for Signal {}

pub struct Signal {
    inner: *mut Inner
}

impl Clone for Signal {
    fn clone(&self) -> Signal {
        self.inner().state.fetch_add(1, Ordering::Relaxed);
        Signal { inner: self.inner }
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        let flag = self.inner().state.fetch_sub(1, Ordering::Relaxed);
        delete_inner(flag, self.inner);
    }
}

impl Signal {
    pub fn new() -> (Signal, Pulse) {
        let inner = Box::new(Inner {
            state: AtomicUsize::new(2),
            waiting: Atom::empty()
        });

        let inner = unsafe {mem::transmute(inner)};

        (Signal {
            inner: inner
         },
         Pulse {
            inner: inner,
        })
    }

    fn inner(&self) -> &Inner {
        unsafe { mem::transmute(self.inner) }
    }

    // Read out the state of the Signal
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
    fn arm(self, waiter: Box<Waiting>) -> ArmedSignal {
        let id = self.add_to_waitlist(waiter);
        ArmedSignal {
            id: id,
            pulse: self
        }
    }

    pub fn id(&self) -> usize {
        unsafe { mem::transmute_copy(&self.inner) }
    }

    pub fn recycle(&self) -> Option<Pulse> {
        if self.in_use() {
            None
        } else {
            self.inner().state.store(2, Ordering::Relaxed);
            Some(Pulse{
                inner: self.inner,
            })
        }
    }

    pub fn on_complete<F>(self, f: F) where F: FnOnce() + Send + 'static {
        self.arm(Waiting::callback(Box::new(f)));
    }
}

impl IntoRawPtr for Pulse {
    unsafe fn into_raw(self) -> *mut () {
        let inner = self.inner;
        mem::forget(self);
        mem::transmute(inner)
    }
}

impl FromRawPtr for Pulse {
    unsafe fn from_raw(ptr: *mut ()) -> Pulse {
        Pulse { inner: mem::transmute(ptr) }
    }
}

#[derive(Debug)]
pub struct WaitError(usize);

struct ArmedSignal {
    pulse: Signal,
    id: usize
}

impl Deref for ArmedSignal {
    type Target = Signal;

    fn deref(&self) -> &Signal { &self.pulse }
}

impl ArmedSignal {
    fn disarm(self) -> Signal {
        self.remove_from_waitlist(self.id);
        self.pulse
    }
}


/// allows an object to assert a wait signal
pub trait Signals {
    /// Get a signal from a object
    fn signal(&mut self) -> Signal;

    /// Block the current thread until the object
    /// assets a pulse.
    fn wait(&mut self) -> Result<(), WaitError> {
        let mut signal = self.signal();

        loop {
            if !signal.is_pending() {
                let state = signal.state();
                return if (state & PULSED) == PULSED {
                    Ok(())
                } else {
                    Err(WaitError(state))
                };
            }

            let p = signal.arm(Waiting::thread());
            if p.is_pending() {
                thread::park();
            }
            signal = p.disarm();
        }
    }
}

impl Signals for Signal {
    fn signal(&mut self) -> Signal { self.clone() }
}