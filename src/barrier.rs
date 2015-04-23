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



use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::iter::IntoIterator;

use {Pulse, Signal, ArmedSignal, Waiting, Signals};

struct Inner {
    pub count: AtomicUsize,
    pub trigger: Mutex<Option<Pulse>>
}

pub struct Barrier {
    inner: Arc<Inner>,
    pulses: Vec<ArmedSignal>
}

pub struct Handle(pub Arc<Inner>);

// This is dumb, I can't find a trait that gives me 
// Mutable access to pulses as an array
impl Barrier {
    pub fn new(pulses: Vec<Signal>) -> Barrier {
        // count items
        let inner = Arc::new(Inner{
            count: AtomicUsize::new(pulses.len()),
            trigger: Mutex::new(None)
        });
        let pulses: Vec<ArmedSignal> = 
            pulses.into_iter()
                  .map(|x| x.arm(Waiting::barrier(Handle(inner.clone()))))
                  .collect();

        Barrier {
            inner: inner,
            pulses: pulses
        }
    }

    pub fn take(self) -> Vec<Signal> {
        self.pulses.into_iter().map(|x| x.disarm()).collect()
    }
}

impl Signals for Barrier {
    fn signal(&mut self) -> Signal {
        let (p, t) = Signal::new();
        if self.inner.count.load(Ordering::Relaxed) == 0 {
            t.pulse();
        } else {
            let mut guard = self.inner.trigger.lock().unwrap();
            *guard = Some(t);
        }
        p
    }
}
