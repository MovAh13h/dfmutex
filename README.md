# DFMutex - Deadlock-free Locks for Rust
DFMutex is a library that provides a ***guaranteed deadlock-free*** Mutex implementation for the Rust language. It's inspiration comes from *[Higher-Order Leak and Deadlock Free Locks](https://dl.acm.org/doi/abs/10.1145/3571229)* and the calculus developed in paper. 

The library relies on five principles to provide a safe locking interface. They are as follows:
 1. *Each thread only holds one reference to any given lock.*
 2. *Any two threads may share at most one lock.*
 3. *If we consider the graph where threads are connected to the locks they hold a reference to, this graph must not have a cycle*
 4. *If we consider the graph where threads are connected to the locks they hold a reference to, and locks are connected to locks they hold a reference to, this graph must not have a cycle.*
 5. *If we consider the graph where threads are connected to the locks they hold a reference to, and locks are connected to locks they hold a reference to, this graph must not have a cycle. Furthermore, each lock must have precisely one owning reference, and zero or more client references.*

The five rules are enough to prevent any form in cycles during lock acquisition and prevent deadlocks. However, these rules also force the library to ship with it's own way of spawning threads and writing certain programs is not possible with such kind of a locking interface.

## Usage
Before we get to examples, it is important to note the changes in the API provided by the library. The library provides the core lock under `dfmutex::DFMutex` and its own way to spawn threads with the `dfmutex::spawn` method. It is important to note the function signature of the `spawn` method:
```rust
fn spawn<D, T, F>(dfm: &DFMutex<D>, f: F) -> JoinHandle<T>
where
    F: FnOnce(DFMutex<D>) -> T + Send + 'static,
    D: Send + 'static,
    T: Send + 'static,
{
	// ...
}
```
The function takes in a shared reference to a `DFMutex` and a closure that takes in an owned value of the `DFMutex`. When a thread is spawned, the Mutex with the shared reference is cloned and passed to the thread as an argument to the closure and run for execution. The result of this entire operation is a `JoinHandle` for the newly created thread which is returned back.

Now to its usage:
```rust
use dfmutex::{DFMutex, spawn};

fn main() {
	// Create a Mutex with any owned value
	let m = DFMutex::new(String::from("Lorem Ipsum"));

	// Create a closure to pass in the thread.
	// The type of the created Mutex above should be same as the
	// argument to the closure.
	let closure = |mut dfm: DFMutex<String>| {
	    let data = dfm.lock().unwrap();
	    
	    // Use the data
	    println!("{}", data);
	};

	// Spawn 8 threads and store their handles
	let mut handles = Vec::new();
	for _ in 0..8 {
	    handles.push(spawn(&m, closure));    
	}

	// Join all the threads
	for handle in handles.into_iter() {
	    handle.join().unwrap();
	}
}
```

`DFMutex` is just a light-weight wrapped around `std::sync::Mutex` and hence, it can be used interchangeably with existing code bases. It should be noted that there are **no deadlock-free guarantees** when `DFMutex` is used with `std::thread::spawn`.

## Testing

A fair number of tests have been written but more complex examples can certainly be introduced:
```sh
cargo test
```

## Acknowledgement

 - Jules Jacobs (Radboud University, The Netherlands)
 - Stephanie Balzer (Carnegie Mellon University, USA)