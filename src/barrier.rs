use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::iter::IntoIterator;

use {Trigger, Pulse, ArmedPulse, Waiting};

struct Inner {
    pub count: AtomicUsize,
    pub trigger: Mutex<Option<Trigger>>
}

pub struct Barrier {
    inner: Arc<Inner>,
    pulses: Vec<ArmedPulse>
}

pub struct Handle(pub Arc<Inner>);

// This is dumb, I can't find a trait that gives me 
// Mutable access to pulses as an array
impl Barrier {
    pub fn new(pulses: Vec<Pulse>) -> Barrier {
        // count items
        let inner = Arc::new(Inner{
            count: AtomicUsize::new(pulses.len()),
            trigger: Mutex::new(None)
        });
        let pulses: Vec<ArmedPulse> = 
            pulses.into_iter()
                  .map(|x| x.arm(Waiting::barrier(Handle(inner.clone()))))
                  .collect();

        Barrier {
            inner: inner,
            pulses: pulses
        }
    }

    pub fn pulse(&self) -> Pulse {
        let (p, t) = Pulse::new();
        if self.inner.count.load(Ordering::Relaxed) == 0 {
            t.pulse();
        } else {
            let mut guard = self.inner.trigger.lock().unwrap();
            *guard = Some(t);
        }
        p
    }

    pub fn take(self) -> Vec<Pulse> {
        self.pulses.into_iter().map(|x| x.disarm()).collect()
    }
}

