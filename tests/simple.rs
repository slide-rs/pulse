
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
    p.wait().unwrap();
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
    p.wait().unwrap();
    assert!(!p.is_pending());
}

#[test]
#[should_panic]
fn dropped() {
    let (p, t) = Pulse::new();
    drop(t);
    p.wait().unwrap();
}

#[test]
#[should_panic]
fn dropped_thread() {
    let (p, t) = Pulse::new();
    thread::spawn(|| {
        thread::sleep_ms(10);
        drop(t);
    });
    p.wait().unwrap();
}

#[test]
fn recycle() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    t.pulse();
    assert!(!p.is_pending());
    let t = p.recycle().unwrap();
    assert!(p.is_pending());
    t.pulse();
    assert!(!p.is_pending());
}

#[test]
#[should_panic]
fn recycle_panic() {
    let (p, t) = Pulse::new();
    let _ = p.recycle().unwrap();
    drop(t);
}

#[test]
fn false_positive_wake() {
    let (p, t) = Pulse::new();
    thread::current().unpark();
    thread::spawn(|| {
        thread::sleep_ms(10);
        t.pulse();
    });
    p.wait().unwrap();
}