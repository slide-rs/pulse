
extern crate pulse;

use std::thread;
use std::sync::Arc;
use pulse::Pulse;

#[test]
fn wait() {
    let p = Pulse::new();
    assert!(!p.triggered());
    assert!(p.trigger());
    assert!(p.triggered());
    assert!(!p.trigger());
}

#[test]
fn wake_post() {
    let p = Pulse::new();
    assert!(!p.triggered());
    p.trigger();
    assert!(p.triggered());
    p.wait();
}

#[test]
fn wake_thread_spawn() {
    let p = Arc::new(Pulse::new());
    assert!(!p.triggered());
    let pc = p.clone(); 
    thread::spawn(|| {
        let pc = pc;
        thread::sleep_ms(10);
        pc.trigger();
    });
    assert!(!p.triggered());
    p.wait();
    assert!(p.triggered());
}

#[test]
fn wake_thread_scope() {
    let p = Pulse::new();
    assert!(!p.triggered());
    let guard = thread::scoped(|| {
        thread::sleep_ms(10);
        p.trigger();
    });
    assert!(!p.triggered());
    p.wait();
    assert!(p.triggered());
    guard.join();
}