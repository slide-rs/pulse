
extern crate pulse;

use std::thread;
use pulse::*;

#[test]
fn wait() {
    let (p, t) = Pulse::new();
    assert!(!p.triggered());
    t.trigger();
    assert!(p.triggered());
}

#[test]
fn wake_post() {
    let (p, t) = Pulse::new();
    assert!(!p.triggered());
    t.trigger();
    assert!(p.triggered());
    p.wait();
}

#[test]
fn wake_thread_spawn() {
    let (p, t) = Pulse::new();
    assert!(!p.triggered());
    thread::spawn(|| {
        thread::sleep_ms(10);
        t.trigger();
    });
    assert!(!p.triggered());
    p.wait();
    assert!(p.triggered());
}
