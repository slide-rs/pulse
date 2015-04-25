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


use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use {Signal, ArmedSignal, Pulse, Waiting, Barrier, Signals};

struct Inner {
    pub ready: Vec<usize>,
    pub trigger: Option<Pulse>
}

pub struct Handle(pub Arc<Mutex<Inner>>);

pub struct Select {
    inner: Arc<Mutex<Inner>>,
    pulses: HashMap<usize, ArmedSignal>
}

impl Select {
    pub fn new() -> Select {
        Select {
            inner: Arc::new(Mutex::new(Inner{
                ready: Vec::new(),
                trigger: None
            })),
            pulses: HashMap::new()
        }
    }

    pub fn add(&mut self, pulse: Signal) -> usize {
        let id = pulse.id();
        let p = pulse.arm(Waiting::select(Handle(self.inner.clone())));
        self.pulses.insert(id, p);
        id
    }

    pub fn remove(&mut self, id: usize) -> Option<Signal> {
        self.pulses.remove(&id)
            .map(|x| x.disarm())
    }

    pub fn into_barrier(self) -> Barrier {
        let vec: Vec<Signal> = 
            self.pulses
                .into_iter()
                .map(|(_, p)| p.disarm())
                .collect();

        Barrier::new(vec)
    }

    /// None blocking next
    pub fn try_next(&mut self) -> Option<Signal> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(x) = guard.ready.pop() {
            return Some(self.pulses.remove(&x).map(|x| x.disarm()).unwrap())
        }
        None
    }

    /// Get the number of Signals being watched
    pub fn len(&self) -> usize {
        self.pulses.len()
    }
}

impl Iterator for Select {
    type Item = Signal;

    fn next(&mut self) -> Option<Signal> {
        loop {
            if self.pulses.len() == 0 {
                return None;
            }

            let mut pulse = {
                let mut guard = self.inner.lock().unwrap();
                if let Some(x) = guard.ready.pop() {
                    return Some(self.pulses.remove(&x).map(|x| x.disarm()).unwrap());
                }
                let (pulse, t) = Signal::new();
                guard.trigger = Some(t);
                pulse
            };
            pulse.wait().unwrap();
        }
    }
}

impl Signals for Select {
    fn signal(&mut self) -> Signal {
        let (pulse, t) = Signal::new();
        let mut guard = self.inner.lock().unwrap();
        if guard.ready.len() == 0 {
            guard.trigger = Some(t);
        } else {
            t.pulse();
        }
        pulse  
    }
}


pub struct SelectMap<T> {
    select: Select,
    items: HashMap<usize, T>
}

impl<T> SelectMap<T> {
    pub fn new() -> SelectMap<T> {
        SelectMap {
            select: Select::new(),
            items: HashMap::new()
        }
    }

    pub fn add(&mut self, pulse: Signal, value: T) {
        let id = self.select.add(pulse);
        self.items.insert(id, value);
    }

    /// None blocking next
    pub fn try_next(&mut self) -> Option<(Signal, T)> {
        self.select.try_next().map(|x| {
            let id = x.id();
            (x, self.items.remove(&id).unwrap())
        })
    }

    /// Get the number of items in teh Select
    pub fn len(&self) -> usize { self.items.len() }
}

impl<T> Iterator for SelectMap<T> {
    type Item = (Signal, T);

    fn next(&mut self) -> Option<(Signal, T)> {
        self.select.next().map(|x| {
            let id = x.id();
            (x, self.items.remove(&id).unwrap())      
        })
    }
}

impl<T> Signals for SelectMap<T> {
    fn signal(&mut self) -> Signal {
        self.select.signal()
    }
}
