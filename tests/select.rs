extern crate pulse;

use std::thread;
use pulse::*;

#[test]
fn select_one() {
    let (p, t) = Pulse::new();
    let mut select = Select::new();
    let id = select.add(p);
    t.pulse();
    let p = select.next().unwrap();
    assert_eq!(id, p.id());
}

#[test]
fn select_three() {
    let (p0, t0) = Pulse::new();
    let (p1, t1) = Pulse::new();
    let (p2, t2) = Pulse::new();

    let mut select = Select::new();
    let id0 = select.add(p0);
    let id1 = select.add(p1);
    let id2 = select.add(p2);

    t0.pulse();
    let p = select.next().unwrap();
    assert_eq!(id0, p.id());

    t1.pulse();
    let p = select.next().unwrap();
    assert_eq!(id1, p.id());

    t2.pulse();
    let p = select.next().unwrap();
    assert_eq!(id2, p.id());
}

#[test]
fn select_thread() {
    let (p0, t0) = Pulse::new();
    let (p1, t1) = Pulse::new();
    let (p2, t2) = Pulse::new();

    let mut select = Select::new();
    let id0 = select.add(p0);
    let id1 = select.add(p1);
    let id2 = select.add(p2);

    thread::spawn(move || {
        thread::sleep_ms(10);
        t0.pulse();
        thread::sleep_ms(10);
        t1.pulse();
        thread::sleep_ms(10);
        t2.pulse();
    });

    let p = select.next().unwrap();
    assert_eq!(id0, p.id());
    let p = select.next().unwrap();
    assert_eq!(id1, p.id());
    let p = select.next().unwrap();
    assert_eq!(id2, p.id());
}