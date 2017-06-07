#![feature(test)]

extern crate mime;
extern crate test;

use mime::{Mime, TEXT_PLAIN_UTF_8};
use test::Bencher;

#[bench]
fn bench_eq_parsed(b: &mut Bencher) {
    let mime = "text/plain; charset=utf-8".parse::<Mime>().unwrap();
    b.bytes = mime.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime, TEXT_PLAIN_UTF_8);
    })
}

#[bench]
fn bench_eq_consts(b: &mut Bencher) {
    let mime = TEXT_PLAIN_UTF_8;
    b.bytes = mime.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime, TEXT_PLAIN_UTF_8);
    });
}
