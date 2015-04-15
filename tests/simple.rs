
extern crate pulse;

use std::thread;
use pulse::*;

#[test]
fn wait() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    t.pulse();
    assert!(!p.is_pending());
}

#[test]
fn wake_post() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    t.pulse();
    assert!(!p.is_pending());
    p.wait();
}

#[test]
fn wake_thread_spawn() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    thread::spawn(|| {
        thread::sleep_ms(10);
        t.pulse();
    });
    assert!(p.is_pending());
    p.wait();
    assert!(!p.is_pending());
}
