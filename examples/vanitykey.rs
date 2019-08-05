use blake2::{Blake2b, Digest};
use crossbeam::channel;
use sha2::Sha512;
use sha3::Sha3_512;
use std::thread;

static MAX_LEN: usize = 1;

fn producer(sender: channel::Sender<u128>) {
    let mut salt_gen = SaltGen { counter: 0 };
    while sender.send(salt_gen.new_salt()).is_ok() {}
}

struct SaltGen {
    counter: u128,
}

impl SaltGen {
    fn new_salt(&mut self) -> u128 {
        self.counter += 1;
        self.counter
    }
}

fn add_salt(mut input_val: Vec<u8>, salt: u128) -> Vec<u8> {
    input_val.append(&mut salt.to_be_bytes().to_vec());
    input_val
}

fn consumer_blake2(
    receiver: channel::Receiver<u128>,
    input_val: String,
    vanity: String,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let input = input_val.into_bytes();
        let mut hasher = Blake2b::new();
        let mut return_vec = Vec::new();
        while let Ok(salt) = receiver.recv() {
            hasher.input(&add_salt(input.clone(), salt));
            let result = format!("{:x}", hasher.result_reset());
            if vanity == result[..vanity.len()] {
                return_vec.push(result);
            }

            if return_vec.len() >= MAX_LEN {
                drop(receiver);
                break;
            }
        }
        return_vec
    })
}

fn consumer_sha3(
    receiver: channel::Receiver<u128>,
    input_val: String,
    vanity: String,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let input = input_val.into_bytes();
        let mut hasher = Sha3_512::new();
        let mut return_vec = Vec::new();
        while let Ok(salt) = receiver.recv() {
            hasher.input(&add_salt(input.clone(), salt));
            let result = format!("{:x}", hasher.result_reset());
            if vanity == result[..vanity.len()] {
                return_vec.push(result);
            }

            if return_vec.len() >= MAX_LEN {
                drop(receiver);
                break;
            }
        }
        return_vec
    })
}

fn consumer_sha2(
    receiver: channel::Receiver<u128>,
    input_val: String,
    vanity: String,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let input = input_val.into_bytes();
        let mut hasher = Sha512::new();
        let mut return_vec = Vec::new();
        while let Ok(salt) = receiver.recv() {
            hasher.input(&add_salt(input.clone(), salt));
            let result = format!("{:x}", hasher.result_reset());
            if vanity == result[..vanity.len()] {
                return_vec.push(result);
            }

            if return_vec.len() >= MAX_LEN {
                drop(receiver);
                break;
            }
        }
        return_vec
    })
}

fn main() {
    dbg!("start");
    let vanity = String::from("cab");
    let input_val = String::from("kex generation");

    let (sender, receiver) = channel::bounded(1);

    dbg!("spawning threads");
    let c_blake2 = consumer_blake2(receiver.clone(), input_val.clone(), vanity.clone());
    let c_sha3 = consumer_sha3(receiver.clone(), input_val.clone(), vanity.clone());
    let c_sha2 = consumer_sha2(receiver, input_val, vanity);
    dbg!("returned after threads!");
    producer(sender);
    dbg!("producer finished!");
    println!("{:?}", c_blake2.join());
    println!("{:?}", c_sha3.join());
    println!("{:?}", c_sha2.join());
}
