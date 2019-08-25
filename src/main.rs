// Another example

use crossbeam::channel::unbounded;
extern crate rayon;
// use instrument;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process;
use std::thread;

fn main() {
    println!("Hello, world, {}!", "I'm here");

    let (x1, x2) = rayon::join(
        || {
            999_999_999_999_999u128 * 999_999_999_999_999u128 / 9 * 9999 % 459394924
                * 12
                * 2805327594
                * 4937023
                * 345
                / 248347530304703
                + hello(7, 8, 9) as u128
        },
        || {
            999_999_999_999_999u128 * 999_999_999_999_999u128 / 9 * 9999 % 459394924
                * 12
                * 2805327594
                * 4937023
                * 345
                / 248347530304703
                + hello(7, 8, 9) as u128
        },
    );

    // Create a channel of unbounded capacity.
    let (s, r) = unbounded();

    // Send a message into the channel.
    s.send("Hello, world!").unwrap();

    // Receive the message from the channel.
    let r = r.recv().unwrap();
    println!("{}", r == "Hello, world!");

    let (y1, y2) = rayon::join(
        || 999_999_999_999_999u128 * 999_999_999_999_999u128,
        || 999_999_999_999_999u128 * 999_999_999_999_999u128,
    );
    println!("{}", x1 + x2);
    // println!("{}", y1 + y2);
    // let y = vec_calc(4, 6, 8, 5, 7, 9);

    // println!("{}", y);

    let func = {
        println!("before");
        let ret = || hello(1, 4, 7);
        println!("after");
        ret
    };

    thread::spawn(func);

    println!("Whats the matter here?");

    thread::spawn(|| hello(1, 4, 7));

    let struc = SomeStruct::new();

    let stuff = struc.do_some().hello("Robert");

    let contains_hello = SomeStruct::new().do_some().hello("Tim").contains("Hello");

    let contains_name = { SomeStruct::new().do_some().hello("Tim") }.contains("Tim");

    println!(
        "The result of {} + {} * {} / {} is: {}",
        1,
        2,
        3,
        4,
        1 + 2 * 3 / 4
    );

    // dbg!(result.join());
    let mut joins = Vec::new();

    for _ in 0..4 {
        joins.push(thread::spawn(move || {
            hello(6, 7, 8);
        }));
    }
    for handle in joins {
        handle.join();
    }

    let closure = || hello(11, 22, 33);
    thread::spawn(move || {
        let ret = {
            if false {
                thread::spawn(closure);
                hello(7, 8, 9)
            } else {
                thread::spawn(move || {
                    let ret = {
                        if false {
                            hello(7, 8, 9)
                        } else {
                            SomeStruct::new().do_some().hello("Tim").contains("Hello") as i32
                        }
                    };
                    ret
                });
                5
            }
        };
        ret
    });

    // (1..6).for_each(|x| {
    //     println!("{}", x);
    // });

    // SomeStruct {
    //     some_field: "hello".to_string(),
    //     other_field: true,
    // }
    // .hello("Steven");

    // let v = vec![1, 2, 3, 4, 5];
    // dbg!(v.iter().next());
    // let x = (3 + hello(1, 2, 3)) * 2;
    // let mut i = 1;
    // while hello(i, 2, 3) < 18 {
    //     println!("{}", "starting...");
    //     let ret = func(i);
    //     println!("{}", ret);
    //     i += 1;
    //     println!("{}", "ending...");
    // }
}

fn hello(a: i32, b: i32, c: i32) -> i32 {
    (a + b + c + 1) * 2
}

fn func(a: i32) -> i32 {
    hello(a + 1, a * 2, a * 3 + 1)
}

// fn vec_calc(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
//     let (x, y) = rayon::join(|| hello(a, b, c), || hello(d, e, f));
//     x + y
// }

struct SomeStruct {
    some_field: String,
    other_field: bool,
}

impl SomeStruct {
    fn new() -> SomeStruct {
        SomeStruct {
            some_field: String::from("I am a struct"),
            other_field: true,
        }
    }

    fn hello(&self, greated: &str) -> String {
        let fs = format!("Hello {}, {}", greated, self.some_field);
        eprintln!("{}", fs);
        fs
    }

    fn do_some(mut self) -> SomeStruct {
        self.other_field = false;
        self
    }
}
