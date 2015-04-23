# Pulse

Pulse is a small library build around the idea of a single shot async notification, a Pulse.
A pulse contains no data, just the state of whether is pending, triggered, or dropped.

From this basic building block, a developer can get notifications of when state changes 
have occurred that the program needs to act on. For example, you have have a number of
small tasks that the developer is interested in being woken for. You may want to listen
to one, or more. Currently using `JoinHandle` you have to wait in sequence to join one
or more threads. If each thread has a `Pulse`, you can `Select` over each `Pulse` knowing
what threads have been woken and when.

## Basic Usage

```rust
// Create a pulse & its trigger
let (mut signal, pulse) = Signal::new();

thread::spawn(move || {
	pulse.pulse();
});

// Wait here until the pulse has been triggered
signal.wait().unwrap();
```

## Primitives

### Signal

The basic building block, a set only once flag. It supports cloning
so multiple threads/blocks can wait on a single `Signal` if you need to.

### Pulse

The setting side of a `Signal`. A trigger cannot be cloned, and may only
exist in one place in the system. As part of the triggering process the `Signal`
is moved and dropped. So it is impossible to accidentally fire it twice.

### Select

A `Select` allows the developer to listen to 1-N pulses. When a pulse is fired,
the `Select` will be woken up. A `Select` has no guaranteed ordering of the pulses.

### Barrier

A `Barrier` can listen for 0-N pulses. It will only trigger once all the pulses it
is waiting on have completed.


## Composability

Both a `Select` and a `Barrier` themselves can be turned into a `Pulse`. This allows
for a tree like structure of wake events to be created. This might not seem immediately
useful, but it allows the developer to categorize what woke them up.

For example, you can have a `Select` of tasks that are running. And a `Select` on a messages
coming from the tasks.

```rust
let mut tasks = Select::new();
let mut events = Select::new();

let mut task_id = events.add(tasks.pulse());

for pulse in events {
	if pulse.id() == task_id {
		if let Some(p) == tasks.try_next() {
			// ???
		}
	} else if pulse.id() == some_other_event {
		// ???
	}
}

```

