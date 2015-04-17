use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use {Pulse, Trigger, Waiting, Barrier};

struct Inner {
    pub ready: Vec<usize>,
    pub trigger: Option<Trigger>
}

pub struct Handle(pub Arc<Mutex<Inner>>);

pub struct Select {
    inner: Arc<Mutex<Inner>>,
    pulses: HashMap<usize, Pulse>
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
        pulse.arm(
            Box::new(
                Waiting::Select(Handle(self.inner.clone()))
            )
        );
        self.pulses.insert(id, pulse);
        id
    }

    pub fn remove(&mut self, id: usize) -> Option<Pulse> {
        self.pulses.remove(&id)
            .map(|x| {
                x.disarm();
                x
            })
    }

    /// Create a pulse that will trigger when something
    /// is ready to be read from this Select
    pub fn pulse(&mut self) -> Pulse {
        let (pulse, t) = Pulse::new();
        let mut guard = self.inner.lock().unwrap();
        if guard.ready.len() == 0 {
            guard.trigger = Some(t);
        } else {
            t.pulse();
        }
        pulse  
    }

    pub fn into_barrier(self) -> Barrier<Vec<Pulse>> {
        let vec: Vec<Pulse> = 
            self.pulses
                .into_iter()
                .map(|(_,p)| {
                    p.disarm();
                    p
                }).collect();

        Barrier::new(vec)
    }

    /// None blocking next
    pub fn try_next(&mut self) -> Option<Pulse> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(x) = guard.ready.pop() {
            return Some(self.pulses.remove(&x).unwrap())
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
                    return Some(self.pulses.remove(&x).unwrap())
                }
                let (pulse, t) = Pulse::new();
                guard.trigger = Some(t);
                pulse
            };
            pulse.wait().unwrap();
        }
    }
}
