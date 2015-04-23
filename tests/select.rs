//   Copyright 2015 Colin Sherratt
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.


extern crate pulse;

use std::thread;
use pulse::*;

#[test]
fn select_one() {
    let (p, t) = Signal::new();
    let mut select = Select::new();
    let id = select.add(p);
    t.trigger();
    let p = select.next().unwrap();
    assert_eq!(id, p.id());
}

#[test]
fn select_three() {
    let (p0, t0) = Signal::new();
    let (p1, t1) = Signal::new();
    let (p2, t2) = Signal::new();

    let mut select = Select::new();
    let id0 = select.add(p0);
    let id1 = select.add(p1);
    let id2 = select.add(p2);

    t0.trigger();
    let p = select.next().unwrap();
    assert_eq!(id0, p.id());

    t1.trigger();
    let p = select.next().unwrap();
    assert_eq!(id1, p.id());

    t2.trigger();
    let p = select.next().unwrap();
    assert_eq!(id2, p.id());

    let p = select.next();
    assert!(p.is_none());
}

#[test]
fn select_thread() {
    let (p0, t0) = Signal::new();
    let (p1, t1) = Signal::new();
    let (p2, t2) = Signal::new();

    let mut select = Select::new();
    let id0 = select.add(p0);
    let id1 = select.add(p1);
    let id2 = select.add(p2);

    thread::spawn(move || {
        thread::sleep_ms(10);
        t0.trigger();
        thread::sleep_ms(10);
        t1.trigger();
        thread::sleep_ms(10);
        t2.trigger();
    });

    let p = select.next().unwrap();
    assert_eq!(id0, p.id());
    let p = select.next().unwrap();
    assert_eq!(id1, p.id());
    let p = select.next().unwrap();
    assert_eq!(id2, p.id());
    let p = select.next();
    assert!(p.is_none());
}

#[test]
fn select_barrier() {
    let (p0, t0) = Signal::new();
    let (p1, t1) = Signal::new();
    let (p2, t2) = Signal::new();

    let mut select = Select::new();
    let _ = select.add(p0);
    let _ = select.add(p1);
    let _ = select.add(p2);

    thread::spawn(move || {
        thread::sleep_ms(10);
        t0.trigger();
        thread::sleep_ms(10);
        t1.trigger();
        thread::sleep_ms(10);
        t2.trigger();
    });

    select.into_barrier().pulse().wait().unwrap();
}

#[test]
fn select_already_pulsed() {
    let (p0, t0) = Signal::new();
    t0.trigger();

    let mut select = Select::new();
    let id0 = select.add(p0);

    let p = select.next().unwrap();
    assert_eq!(id0, p.id());
    let p = select.next();
    assert!(p.is_none());
}