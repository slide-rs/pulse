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
use {Pulse, ArmedPulse, Trigger, Waiting, Barrier};

struct Inner {
    pub ready: Vec<usize>,
    pub trigger: Option<Trigger>
}

pub struct Handle(pub Arc<Mutex<Inner>>);

pub struct Select {
    inner: Arc<Mutex<Inner>>,
    pulses: HashMap<usize, ArmedPulse>
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

    pub fn add(&mut self, pulse: Pulse) -> usize {
        let id = pulse.id();
        let p = pulse.arm(Waiting::select(Handle(self.inner.clone())));
        self.pulses.insert(id, p);
        id
    }

    pub fn remove(&mut self, id: usize) -> Option<Pulse> {
        self.pulses.remove(&id)
            .map(|x| x.disarm())
    }

    /// Create a pulse that will trigger when something
    /// is ready to be read from this Select
    pub fn pulse(&mut self) -> Pulse {
        let (pulse, t) = Pulse::new();
        let mut guard = self.inner.lock().unwrap();
        if guard.ready.len() == 0 {
            guard.trigger = Some(t);
        } else {
            t.trigger();
        }
        pulse  
    }

    pub fn into_barrier(self) -> Barrier {
        let vec: Vec<Pulse> = 
            self.pulses
                .into_iter()
                .map(|(_, p)| p.disarm())
                .collect();

        Barrier::new(vec)
    }

    /// None blocking next
    pub fn try_next(&mut self) -> Option<Pulse> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(x) = guard.ready.pop() {
            return Some(self.pulses.remove(&x).map(|x| x.disarm()).unwrap())
        }
        None
    }

    /// Get the number of Pulses being watched
    pub fn len(&self) -> usize {
        self.pulses.len()
    }
}

impl Iterator for Select {
    type Item = Pulse;

    fn next(&mut self) -> Option<Pulse> {
        loop {
            if self.pulses.len() == 0 {
                return None;
            }

            let pulse = {
                let mut guard = self.inner.lock().unwrap();
                if let Some(x) = guard.ready.pop() {
                    return Some(self.pulses.remove(&x).map(|x| x.disarm()).unwrap());
                }
                let (pulse, t) = Pulse::new();
                guard.trigger = Some(t);
                pulse
            };
            pulse.wait().unwrap();
        }
    }
}
