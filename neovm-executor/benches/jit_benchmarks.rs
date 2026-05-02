use criterion::{criterion_group, criterion_main, Criterion};
use neovm_executor::jit_interp::execute_with_jit;

fn bench_fib_jit(c: &mut Criterion) {
    let source = ";;; -*- lexical-binding: t; -*-
(defun fib (n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))
(fib 10)";
    c.bench_function("fib_10/jit", |b| {
        b.iter(|| execute_with_jit("bench.el", source, &[]));
    });
}

fn bench_arithmetic_loop_jit(c: &mut Criterion) {
    let source = ";;; -*- lexical-binding: t; -*-
(let ((sum 0) (i 0))
  (while (< i 1000)
    (setq sum (+ sum i))
    (setq i (1+ i)))
  sum)";
    c.bench_function("arith_loop_1000/jit", |b| {
        b.iter(|| execute_with_jit("bench.el", source, &[]));
    });
}

fn bench_cons_list_jit(c: &mut Criterion) {
    let source = ";;; -*- lexical-binding: t; -*-
(let ((lst nil) (i 100))
  (while (> i 0)
    (setq lst (cons i lst))
    (setq i (1- i)))
  (car lst))";
    c.bench_function("cons_list_100/jit", |b| {
        b.iter(|| execute_with_jit("bench.el", source, &[]));
    });
}

fn bench_insertion_sort_jit(c: &mut Criterion) {
    let source = ";;; -*- lexical-binding: t; -*-
(defun insert-sorted (x lst)
  (cond ((null lst) (list x))
        ((<= x (car lst)) (cons x lst))
        (t (cons (car lst) (insert-sorted x (cdr lst))))))
(defun isort (lst)
  (if (null lst) nil
    (insert-sorted (car lst) (isort (cdr lst)))))
(isort (list 5 3 8 1 9 2 7 4 6 0))";
    c.bench_function("isort_10/jit", |b| {
        b.iter(|| execute_with_jit("bench.el", source, &[]));
    });
}

criterion_group!(
    benches,
    bench_fib_jit,
    bench_arithmetic_loop_jit,
    bench_cons_list_jit,
    bench_insertion_sort_jit,
);
criterion_main!(benches);
