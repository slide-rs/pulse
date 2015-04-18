use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use {Trigger, Pulse, Waiting};

struct Inner {
    pub count: AtomicUsize,
    pub trigger: Mutex<Option<Trigger>>
}

pub struct Barrier<T> {
    inner: Arc<Inner>,
    pulses: T
}

pub struct Handle(pub Arc<Inner>);

impl<T> Barrier<T> where T: AsRef<[Pulse]> {
    pub fn new(pulses: T) -> Barrier<T> {
        let len = pulses.as_ref().len();
        let inner = Arc::new(Inner{
            count: AtomicUsize::new(len),
            trigger: Mutex::new(None)
        });
        for pulse in pulses.as_ref().iter() {
            pulse.arm(Waiting::barrier(Handle(inner.clone())))
        }
        Barrier {
            inner: inner,
            pulses: pulses
        }
    }

    // 
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

    pub fn take(self) -> T {
        for pulse in self.pulses.as_ref().iter() {
            pulse.disarm();
        }
        self.pulses
    }
}
