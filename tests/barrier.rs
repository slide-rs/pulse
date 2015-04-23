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
fn using_vec() {
    let mut pulses = Vec::new();
    let mut triggers = Vec::new();
    for _ in 0..8 {
        let (p, t) = Signal::new();
        pulses.push(p);
        triggers.push(t);
    }

    let mut barrier = Barrier::new(pulses);
    let pulse = barrier.signal();

    let last_trigger = triggers.pop().unwrap();
    for t in triggers {
        t.pulse();
        assert!(pulse.is_pending());
    }

    last_trigger.pulse();
    assert!(!pulse.is_pending());
}

// TODO fix
/*#[test]
fn using_slice() {
    let mut pulses = Vec::new();
    let mut triggers = Vec::new();
    for _ in 0..8 {
        let (p, t) = Signal::new();
        pulses.push(p);
        triggers.push(t);
    }

    let barrier = Barrier::new(pulses);
    let pulse = barrier.signal();

    let last_trigger = triggers.pop().unwrap();
    for t in triggers {
        t.signal();
        assert!(pulse.is_pending());
    }

    last_trigger.signal();
    assert!(!pulse.is_pending());
}*/

#[test]
fn empty() {
    let mut barrier = Barrier::new(Vec::new());
    let pulse = barrier.signal();
    assert!(!pulse.is_pending());
}

#[test]
fn using_threads() {
    let mut pulses = Vec::new();
    let mut triggers = Vec::new();
    for _ in 0..8 {
        let (p, t) = Signal::new();
        pulses.push(p);
        triggers.push(t);
    }

    let mut barrier = Barrier::new(pulses);
    let mut pulse = barrier.signal();

    thread::spawn(move || {
        for t in triggers {
            t.pulse();
        }
    });

    pulse.wait().unwrap();
}

#[test]
fn dropped_barrier() {
    let mut pulses = Vec::new();
    let mut triggers = Vec::new();
    for _ in 0..8 {
        let (p, t) = Signal::new();
        pulses.push(p);
        triggers.push(t);
    }

    let pulse = {
        let mut barrier = Barrier::new(pulses);
        barrier.signal()
    };

    let last_trigger = triggers.pop().unwrap();
    for t in triggers {
        t.pulse();
        assert!(pulse.is_pending());
    }

    last_trigger.pulse();
    assert!(!pulse.is_pending());   
}

#[test]
fn barrier_clone() {
    let (p, t) = Signal::new();
    let mut p1 = p.clone();
    let join = thread::spawn(move || {
        p1.wait().unwrap();
    });
    thread::sleep_ms(10);
    let barrier = Barrier::new(vec![p]);
    drop(barrier.take());
    t.pulse();
    join.join().unwrap();
}
