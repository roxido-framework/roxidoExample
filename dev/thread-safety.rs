use roxido::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[unsafe(no_mangle)]
pub extern "C" fn before_init() -> SEXP {
    unsafe { rbindings::Rf_ScalarLogical(R::is_main_thread().into()) }
}

#[roxido]
fn guards() {
    std::thread::spawn(|| {
        assert!(!R::is_main_thread());
        let calls: [fn(); 13] = [
            || _ = R::null(),
            || _ = R::NULL(),
            || _ = R::random_bytes::<1>(),
            R::flush_console,
            || _ = R::check_user_interrupt(),
            || _ = RSymbol::new(c"test"),
            || _ = RSymbol::from("test"),
            || _ = RSymbol::dim(),
            || _ = RSymbol::names(),
            || _ = RSymbol::rownames(),
            || _ = RSymbol::dimnames(),
            || _ = RSymbol::class(),
            || _ = rprintln!("should not print"),
        ];
        for call in calls {
            let payload = catch_unwind(call).expect_err("worker call accepted");
            assert!(payload.downcast_ref::<RStopHelper>().is_some());
        }
        calls.len() as i32
    })
    .join()
    .unwrap()
}

#[roxido]
fn parallel_slices() {
    let values = RVector::from_value(1.0, 100, pc);
    std::thread::scope(|scope| {
        for chunk in values.slice_mut().chunks_mut(25) {
            scope.spawn(move || chunk.iter_mut().for_each(|x| *x *= 2.0));
        }
    });
    values.slice().iter().sum::<f64>()
}

#[roxido]
fn propagated_panic() {
    if let Err(payload) = std::thread::spawn(|| rprintln!("should not print")).join() {
        std::panic::resume_unwind(payload);
    }
}

#[roxido]
fn nested_counters() {
    let list = RList::new(3, pc);
    let outer = RVector::from_value(7.0, 1, pc);
    pc.nested(|inner| {
        let value = RVector::from_value(2.0, 1, inner);
        assert!(catch_unwind(AssertUnwindSafe(|| 3_i32.to_r(pc))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| unsafe { pc.protect(value.sexp()) })).is_err());
        // No references to the inserted values are used afterward.
        unsafe {
            list.set_object(0, value).unwrap();
            inner.nested(|innermost| list.set_object(1, 4_i32.to_r(innermost)).unwrap());
            list.set_object(2, 5_i32.to_r(inner)).unwrap();
        }
    });
    let result = catch_unwind(AssertUnwindSafe(|| {
        pc.nested(|inner| {
            let _ = 6_i32.to_r(inner);
            stop!("nested panic");
        });
    }));
    assert!(result.is_err());
    let result = catch_unwind(AssertUnwindSafe(|| {
        list.set_with_pc(0, |inner| {
            let _ = 7_i32.to_r(pc);
            8_i32.to_r(inner)
        })
        .unwrap();
    }));
    assert!(result.is_err());
    let _ = 9_i32.to_r(pc);
    assert_eq!(outer.slice(), &[7.0]);
    list
}

#[roxido]
fn fill_list(n: i32) {
    let list = RList::new(n as usize, pc);
    let mut count = 0;
    list.set_loop_with_pc(|inner| {
        count += 1;
        count.to_r(inner)
    });
    assert_eq!(count, n);
    list
}

#[roxido]
fn set_values(n: i32) {
    let list = RList::new(n as usize, pc);
    for i in 0..list.len() {
        list.set(i, i as i32 + 1).unwrap();
    }
    list
}

#[roxido]
fn set_attributes() {
    let x = RVector::from_value(0.0, 2, pc);
    x.set_names(&["a", "b"]).unwrap();
    assert!(x.set_names(&["a"]).is_err());
    x.set_class(&["foo"]);
    x.set_attribute(RSymbol::from("note").unwrap(), "hi");
    x
}

#[roxido]
fn attribute_after_allocations(x: &RObject, which: &RSymbol) {
    let value = x.get_attribute(which, pc);
    for _ in 0..20 {
        pc.nested(|inner| {
            let _ = ["replacement", "replacement", "replacement"].to_r(inner);
            let _ = RList::with_names(&["a", "b"], inner);
        });
    }
    value
}

#[roxido]
fn list_element(x: &RList) {
    x.get(0).stop()
}

#[roxido]
fn list_element_mut(x: RCow<RList>) {
    // The test supplies lists that nothing else uses.
    let x = unsafe { x.into_mut_in_place() };
    x.get_mut(0).stop().as_scalar().stop().i32().stop()
}

#[roxido]
fn matrix_dimnames() {
    let x = RMatrix::from_value(0.0, 2, 3, pc);
    x.set_dimnames(&[Some(&["a", "b"]), Some(&["x", "y", "z"])])
        .unwrap();
    let names = x
        .get_dimnames()
        .get(1)
        .unwrap()
        .as_vector()
        .unwrap()
        .as_char()
        .unwrap();
    assert_eq!(names.get(2).unwrap(), "z");
    assert!(x.set_dimnames(&[Some(&["a", "b"])]).is_err());
    assert!(x.set_dimnames(&[Some(&["a"]), None]).is_err());
    x.set_dimnames(&[None, Some(&["x", "y", "z"])]).unwrap();
    assert!(x.get_dimnames().get(0).unwrap().is_null());
    x
}

#[roxido]
fn array_dimnames() {
    let x = RArray::from_value(0.0, &[1, 2, 1], pc);
    x.set_dimnames(&[Some(&["a"]), None, Some(&["d"])]).unwrap();
    assert!(x.get_dimnames().get(1).unwrap().is_null());
    assert!(x.set_dimnames(&[Some(&["a"]), None]).is_err());
    x
}

#[roxido]
fn nul_message() {
    Err::<(), _>("before\0after").stop()
}

#[roxido]
fn callback_error() {
    pc.nested(|inner| {
        let _ = 1_i32.to_r(inner);
        stop!("callback error");
    });
}

#[roxido]
fn embedded_nul() {
    pc.nested(|inner| {
        let _ = "a\0b".to_r(inner);
    });
}

#[roxido]
fn raw_r_error() {
    // Exercise recovery after an R long jump, bypassing the Rust string validation.
    pc.nested(|inner| {
        let _ = 1_i32.to_r(inner);
        unsafe {
            rbindings::Rf_mkCharLenCE(b"a\0b".as_ptr().cast(), 3, rbindings::cetype_t_CE_UTF8);
        }
    });
}

#[roxido]
fn nul_unwinds_workers() {
    use std::cell::Cell;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct OnDrop<'a>(&'a Cell<bool>);
    impl Drop for OnDrop<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let dropped = Cell::new(false);
    let finished = Arc::new(AtomicBool::new(false));
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _on_drop = OnDrop(&dropped);
        std::thread::scope(|scope| {
            let finished = Arc::clone(&finished);
            scope.spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(20));
                finished.store(true, Ordering::SeqCst);
            });
            // Deliberately violate the documented threading rule to test the NUL guard.
            pc.nested(|inner| {
                let _ = "a\0b".to_r(inner);
            });
        });
    }));
    let payload = result.expect_err("NUL was accepted");
    assert!(
        payload
            .downcast_ref::<RStopHelper>()
            .unwrap()
            .0
            .contains("NUL")
    );
    assert!(dropped.get());
    assert!(finished.load(Ordering::SeqCst));
    assert_eq!(42_i32.to_r(pc).get(), 42);
    true
}

#[roxido]
fn external_pointers() {
    let pointer = RExternalPtr::encode(String::from("value"), "String", pc);
    // Types match the allocations; each borrow ends before another access.
    unsafe {
        assert_eq!(pointer.decode_ref::<String>(), "value");
        pointer.decode_mut::<String>().push('!');
        assert_eq!(pointer.decode_ref::<String>(), "value!");
        assert!(pointer.decode_val::<String>().is_err());

        let pointer = RExternalPtr::encode_full(42_i32, R::null(), false, pc);
        assert_eq!(pointer.decode_val::<i32>().unwrap(), 42);
        assert!(pointer.is_null());
        assert!(pointer.decode_val::<i32>().is_err());
        pointer.reencode(|_| 43_i32);
        assert_eq!(pointer.decode_val::<i32>().unwrap(), 43);
    }
    true
}

#[roxido]
fn recover_after_callback(f: &RFunction, fails: bool) {
    let outer = 123_i32.to_r(pc);
    pc.nested(|inner| {
        let before = 456_i32.to_r(inner);
        // The test callbacks cannot reach our local vectors.
        assert_eq!(unsafe { f.call0(inner) }.is_err(), fails);
        assert_eq!(before.get(), 456);
        assert_eq!(789_i32.to_r(inner).get(), 789);
    });
    assert_eq!(outer.get(), 123);
    true
}

#[roxido]
fn call_with_language(f: &RFunction, x: &RObject, y: &RObject) {
    // The test callbacks return their arguments without mutating them.
    unsafe { f.call2(x, y, pc) }.unwrap()
}

#[roxido]
fn strings(n: i32) {
    RVector::<char>::from_value("a", n as usize, pc)
}

#[roxido]
fn bump(x: RCow<RVector<f64>>, y: &[f64]) {
    let x = x.into_mut_or_copy(pc);
    x.slice_mut()[0] += 1.0;
    [x.slice()[0], y[0]]
}

#[roxido]
fn transform(x: RCow<RVector<f64>>) {
    let x = x.into_mut_or_copy(pc);
    x.slice_mut().iter_mut().for_each(|x| *x *= 2.0);
    x
}

#[roxido]
fn bump_first(list: RCow<RList>) {
    let list = list.into_mut_or_copy(pc);
    let first = list.get_mut(0).unwrap().as_vector_mut().unwrap();
    first.as_f64_mut().unwrap().slice_mut()[0] += 1.0;
    list
}

#[roxido]
fn bump_in_place(x: RCow<RVector<f64>>, other: &RObject) {
    // The test supplies a vector that nothing else uses while this runs.
    let x = unsafe { x.into_mut_in_place() };
    x.slice_mut()[0] += 1.0;
    let _ = other;
}

#[roxido]
fn object_in_place(x: RCow<RObject>) {
    // The test supplies only objects that this does not modify.
    let _ = unsafe { x.into_mut_in_place() };
}

#[roxido]
fn optional_cow(x: Option<RCow<RVector<f64>>>) {
    match x {
        Some(x) => {
            let x = x.into_mut_or_copy(pc);
            x.slice_mut()[0] += 1.0;
            x.slice()[0]
        }
        None => -1.0,
    }
}

#[roxido]
fn logical_scalars() {
    for value in [true, false] {
        let x = RScalar::<bool>::from_value(value, pc);
        let y = RScalar::<bool>::from_value(value, pc);
        assert_ne!(x.sexp(), y.sexp());
        x.set(!value);
        assert_eq!(y.get(), value);
        assert_eq!(x.get(), !value);
    }
    true
}

// Deliberately violates ExactSizeIterator's reported length.
struct Miscounted<I> {
    values: I,
    len: usize,
}

impl<I: Iterator> Iterator for Miscounted<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}
impl<I: Iterator> ExactSizeIterator for Miscounted<I> {}

#[roxido]
fn initialized_constructors() {
    macro_rules! check_numeric {
        ($ty:ty) => {
            for n in [0, 5] {
                let vector = RVector::<$ty>::new(n, pc);
                assert_eq!(vector.slice(), vec![0 as $ty; n]);
                assert_eq!(vector.slice_mut(), vec![0 as $ty; n]);
                assert_eq!(
                    RVector::from_value(7 as $ty, n, pc).slice(),
                    vec![7 as $ty; n]
                );
                assert_eq!(RMatrix::<$ty>::new(n, 2, pc).slice(), vec![0 as $ty; n * 2]);
                assert_eq!(
                    RMatrix::from_value(7 as $ty, n, 2, pc).slice(),
                    vec![7 as $ty; n * 2]
                );
                assert_eq!(
                    RArray::<$ty>::new(&[1, n, 2], pc).slice(),
                    vec![0 as $ty; n * 2]
                );
                assert_eq!(
                    RArray::from_value(7 as $ty, &[1, n, 2], pc).slice(),
                    vec![7 as $ty; n * 2]
                );
            }
            let data = [1 as $ty, 2 as $ty, 3 as $ty];
            assert!(RVector::<$ty>::from_slice(&[], pc).slice().is_empty());
            assert!(RVector::<$ty>::from_array([], pc).slice_mut().is_empty());
            assert_eq!(RVector::from_slice(&data, pc).slice(), data);
            assert_eq!(RVector::from_array(data, pc).slice(), data);
            assert_eq!(RVector::from_iter1(data.into_iter(), pc).slice(), data);
            assert_eq!(RVector::from_iter2(data.iter(), pc).slice(), data);
        };
    }
    check_numeric!(f64);
    check_numeric!(i32);
    check_numeric!(u8);
    for n in [0, 5] {
        assert_eq!(RVector::<bool>::new(n, pc).slice(), vec![0; n]);
        assert_eq!(RVector::from_value(true, n, pc).slice(), vec![1; n]);
    }
    let data = [true, false, true];
    assert_eq!(RVector::from_slice(&data, pc).slice(), &[1, 0, 1]);
    assert_eq!(RVector::from_array(data, pc).slice(), &[1, 0, 1]);
    assert_eq!(
        RVector::from_iter1(data.into_iter(), pc).slice(),
        &[1, 0, 1]
    );
    assert_eq!(RVector::from_iter2(data.iter(), pc).slice(), &[1, 0, 1]);
    true
}

#[roxido]
fn incorrect_iterator_lengths() {
    macro_rules! check {
        ($data:expr) => {
            let data = $data;
            for len in [0, 2, 4] {
                for borrowed in [false, true] {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        pc.nested(|inner| {
                            if borrowed {
                                let _ = RVector::from_iter2(
                                    Miscounted {
                                        values: data.iter(),
                                        len,
                                    },
                                    inner,
                                );
                            } else {
                                let _ = RVector::from_iter1(
                                    Miscounted {
                                        values: data.into_iter(),
                                        len,
                                    },
                                    inner,
                                );
                            }
                        })
                    }));
                    let payload = result.expect_err("incorrect iterator length accepted");
                    assert!(
                        payload
                            .downcast_ref::<RStopHelper>()
                            .unwrap()
                            .0
                            .contains("Iterator length")
                    );
                }
            }
        };
    }
    check!([1.0, 2.0, 3.0]);
    check!([1_i32, 2, 3]);
    check!([1_u8, 2, 3]);
    check!([true, false, true]);
    assert_eq!(42_i32.to_r(pc).get(), 42);
    true
}

#[roxido]
fn make_pointer() {
    RExternalPtr::encode(42_i32, "i32", pc)
}

#[roxido]
fn clone_object(value: &RObject) {
    value.clone(pc)
}

#[roxido]
fn borrow_mutable_object(value: RCow<RObject>) {
    value.into_mut_or_copy(pc)
}

#[roxido]
fn borrow_mutable_element(values: RCow<RList>) {
    let _ = values.into_mut_or_copy(pc).get_mut(0).stop();
    true
}

#[roxido]
fn pointer_in_place(value: RCow<RExternalPtr>) {
    // The test supplies an exclusively accessed pointer from make_pointer.
    unsafe {
        let value = value.into_mut_in_place();
        *value.decode_mut::<i32>() += 1;
        value
    }
}

#[roxido]
fn pointer_value(value: &RExternalPtr) {
    // The test supplies an i32 pointer from make_pointer.
    unsafe { *value.decode_ref::<i32>() }
}

#[roxido]
fn cow_fresh(x: RCow<RVector<f64>>) {
    x.is_fresh()
}

#[roxido]
fn cow_scale(x: RCow<RVector<f64>>) {
    let y = x.into_mut_or_copy(pc);
    y.slice_mut().iter_mut().for_each(|v| *v *= 2.0);
    y
}

#[roxido]
fn cow_coerce(x: RCow<RVector>) {
    let x = x.to_f64(pc);
    let fresh = x.is_fresh();
    let y = x.into_mut_or_copy(pc);
    y.slice_mut()[0] += 1.0;
    y.set_attribute(RSymbol::from("fresh").unwrap(), fresh);
    y
}

#[roxido]
fn cow_pair(x: RCow<RVector<f64>>, y: RCow<RVector<f64>>) {
    [x.is_fresh(), y.is_fresh()]
}

#[roxido]
fn cow_aliased() {
    let x = RVector::from_value(1.0, 2, pc);
    let y = RVector::from_value(1.0, 2, pc);
    let mut result = Vec::new();
    // Calling the generated entry point directly is the only way to pass one fresh object twice.
    for (a, b) in [(x.sexp(), x.sexp()), (x.sexp(), y.sexp())] {
        let pair = unsafe { RObject::from_sexp(pc.protect(cow_pair(a, b)), pc) };
        result.extend(
            pair.as_vector()
                .unwrap()
                .as_bool()
                .unwrap()
                .slice()
                .iter()
                .map(|&v| v != 0),
        );
    }
    result.into_iter().to_r(pc)
}

static GUARD_DROPS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        // R's API stays usable, and does not disturb R's pending long jump, while Rust unwinds.
        let _ = RSymbol::from("roxido_guard").unwrap();
        GUARD_DROPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[roxido]
fn guard_drops() {
    GUARD_DROPS.swap(0, std::sync::atomic::Ordering::SeqCst)
}

// Each case makes R raise an error, or warn under options(warn = 2), from safe code.
#[roxido]
fn r_error(case: &str, x: &RObject) {
    let _guard = Guard;
    match case {
        "to_i32" => _ = x.as_vector().stop().to_i32(pc),
        "to_f64" => _ = x.as_vector().stop().to_f64(pc),
        "to_u8" => _ = x.as_vector().stop().to_u8(pc),
        "f64" => _ = x.as_scalar().stop().f64(),
        "symbol" => _ = RSymbol::from(""),
        "allocate" => _ = RVector::<f64>::new(1 << 52, pc),
        "protect" => {
            for i in 0..200_000 {
                let _ = (i as f64).to_r(pc);
            }
        }
        "random" => _ = R::random_bytes::<1>(),
        "dim" => {
            let y = RVector::from_value(0.0, 3, pc);
            y.set_attribute(RSymbol::dim(), &[2, 2][..]);
        }
        "class" => RVector::from_value(0.0, 3, pc).set_class(&["factor"]),
        _ => stop!("Unknown case."),
    }
    true
}

#[roxido]
fn swallowed_r_error() {
    let _guard = Guard;
    // Catching R's jump abandons it, as R's own tryCatch does.
    let result = catch_unwind(AssertUnwindSafe(|| {
        pc.nested(|inner| _ = RVector::<f64>::new(1 << 52, inner))
    }));
    assert!(result.unwrap_err().downcast_ref::<RUnwind>().is_some());
    42_i32.to_r(pc).get()
}

#[roxido]
fn echo_str(x: &str) {
    x
}
