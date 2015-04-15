
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use atom::{GetNextMut, Atom};
use {Pulse, Trigger, Waiting};

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
}

impl Iterator for Select {
    type Item = Pulse;

    fn next(&mut self) -> Option<Pulse> {
        loop {
            let pulse = {
                let mut guard = self.inner.lock().unwrap();
                if let Some(x) = guard.ready.pop() {
                    return Some(self.pulses.remove(&x).unwrap())
                }
                let (pulse, t) = Pulse::new();
                guard.trigger = Some(t);
                pulse
            };
            pulse.wait();
        }
    }
}
