extern crate pulse;

use std::thread;
use pulse::*;

#[test]
fn wait() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    t.trigger();
    assert!(!p.is_pending());
}

#[test]
fn wake_post() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    t.trigger();
    assert!(!p.is_pending());
    p.wait().unwrap();
}

#[test]
fn wake_thread_spawn() {
    let (p, t) = Pulse::new();
    assert!(p.is_pending());
    thread::spawn(|| {
        thread::sleep_ms(10);
        t.trigger();
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
    t.trigger();
    assert!(!p.is_pending());
    let t = p.recycle().unwrap();
    assert!(p.is_pending());
    t.trigger();
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
        t.trigger();
    });
    p.wait().unwrap();
}

#[test]
fn clone() {
    let (p0, t) = Pulse::new();
    let p1 = p0.clone();

    assert!(p0.is_pending());
    assert!(p1.is_pending());
    assert_eq!(p0.id(), p1.id());

    t.trigger();

    assert!(!p0.is_pending());
    assert!(!p1.is_pending());

    drop(p0);
    assert!(!p1.is_pending());
    drop(p1);
}

#[test]
fn clone_recyle() {
    let (p0, t) = Pulse::new();
    let p1 = p0.clone();

    assert!(p0.is_pending());
    assert!(p1.is_pending());
    assert_eq!(p0.id(), p1.id());

    t.trigger();

    assert!(!p0.is_pending());
    assert!(!p1.is_pending());
    assert!(p0.recycle().is_none());
    assert!(!p1.is_pending());
    drop(p0);
    assert!(p1.recycle().is_some());
}

#[test]
fn clone_wait() {
    let (p0, t) = Pulse::new();
    let p1 = p0.clone();

    let t0 = thread::spawn(move || {
        p0.wait().unwrap();
    });

    let t1 = thread::spawn(move || {
        p1.wait().unwrap();;
    });

    thread::sleep_ms(10);
    t.trigger();
    t0.join().unwrap();
    t1.join().unwrap();
}

#[test]
fn barrier_reuse() {
    let (p, t) = Pulse::new();
    let barrier = Barrier::new(vec![p.clone()]);
    let barriers: Vec<Barrier> =
        (0..20).map(|_| Barrier::new(vec![p.clone()]))
               .collect();

    let triggers: Vec<Pulse> = barriers.into_iter().map(|b| {
        let p = b.pulse();
        assert!(p.is_pending());
        b.take();
        p
    }).collect();

    assert!(p.is_pending());
    assert!(barrier.pulse().is_pending());
    t.trigger();
    assert!(!p.is_pending());
    assert!(!barrier.pulse().is_pending());
    for p in triggers {
        // These will all error out since the trigger
        // was destroyed;
        assert!(p.wait().is_err());
    }
}
