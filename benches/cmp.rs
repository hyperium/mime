#![feature(test)]

extern crate mime;
extern crate test;

use mime::*;
use test::Bencher;

#[bench]
fn bench_eq_parsed_atom(b: &mut Bencher) {
    let mime = "text/plain; charset=utf-8".parse::<MediaType>().unwrap();
    b.bytes = mime.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime, TEXT_PLAIN_UTF_8);
    })
}

#[bench]
fn bench_eq_parsed_dynamic(b: &mut Bencher) {
    let mime1 = "text/foo; charset=utf-8".parse::<MediaType>().unwrap();
    let mime2 =  mime1.clone();
    b.bytes = mime1.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime1, mime2);
    })
}

#[bench]
fn bench_eq_multiple_parameters(b: &mut Bencher) {
    let mime1 = "text/foo; aaa=bbb; ccc=ddd; eee=fff; ggg=hhh".parse::<MediaType>().unwrap();
    let mime2 =  mime1.clone();
    b.bytes = mime1.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime1, mime2);
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

#[cfg(feature = "macro")]
#[bench]
fn bench_eq_proc_macro(b: &mut Bencher) {
    let mime = media_type!("text/plain; charset=utf-8");
    b.bytes = mime.as_ref().len() as u64;
    b.iter(|| {
        assert_eq!(mime, TEXT_PLAIN_UTF_8);
    });
}

#[bench]
fn bench_ne_consts(b: &mut Bencher) {
    let one = TEXT_XML;
    let two = TEXT_CSS;
    b.bytes = one.as_ref().len() as u64;
    b.iter(|| {
        assert_ne!(one, two);
    });
}

#[bench]
fn bench_eq_type_(b: &mut Bencher) {
    let mime = TEXT_PLAIN_UTF_8;
    let name = TEXT;
    b.bytes = name.len() as u64;
    b.iter(|| {
        assert_eq!(mime.type_(), name);
    });
}
