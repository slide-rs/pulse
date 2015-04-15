#![feature(test)]

extern crate test;
extern crate pulse;

use std::sync::mpsc::channel;
use test::Bencher;
use pulse::*;

#[bench]
fn pulse_already_set(b: &mut Bencher) {
    let (p, t) = Pulse::new();
    t.pulse();

    b.iter(|| {
        p.wait().unwrap();
    });
}

#[bench]
fn pulse_set(b: &mut Bencher) {
    b.iter(|| {
        let (p, t) = Pulse::new();
        t.pulse();
        p.wait().unwrap();
    });
}

#[bench]
fn oneshot_channel(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = channel();
        tx.send(()).unwrap();
        rx.recv().unwrap();
    });
}