#![feature(test)]

extern crate mime;
extern crate test;

use mime::MediaType;
use test::Bencher;


#[bench]
fn text_plain(b: &mut Bencher) {
    let s = "text/plain";
    b.bytes = s.as_bytes().len() as u64;
    b.iter(|| s.parse::<MediaType>())
}

#[bench]
fn text_nonatom(b: &mut Bencher) {
    let s = "text/other";
    b.bytes = s.as_bytes().len() as u64;
    b.iter(|| s.parse::<MediaType>())
}

#[bench]
fn text_plain_charset_utf8(b: &mut Bencher) {
    let s = "text/plain; charset=utf-8";
    b.bytes = s.as_bytes().len() as u64;
    b.iter(|| s.parse::<MediaType>())
}

#[bench]
fn text_nonatom_charset_utf8(b: &mut Bencher) {
    let s = "text/other; charset=utf-8";
    b.bytes = s.as_bytes().len() as u64;
    b.iter(|| s.parse::<MediaType>())
}

#[bench]
fn text_plain_charset_utf8_extended(b: &mut Bencher) {
    let s = "text/plain; charset=utf-8; foo=bar";
    b.bytes = s.as_bytes().len() as u64;
    b.iter(|| s.parse::<MediaType>())
}
